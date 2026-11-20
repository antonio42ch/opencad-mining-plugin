//! Classification of a geotechnical record: RMR89, Q, GSI and class, with
//! the reasons a value could not be obtained. Nothing is invented: a missing
//! parameter leaves the value empty unless the user enabled a documented
//! correlation, which is then reported in `derived`.

use std::collections::BTreeMap;

use crate::data::{GtField, GtRecord, GtTable};
use crate::gsi::{gsi_from_q_prime, gsi_from_rmr89, Correlation};
use crate::q::{q_prime, QParams};
use crate::rmr::*;

/// Laboratory values of one lithology.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LabRow {
    pub ucs: Option<f64>,
    pub is50: Option<f64>,
    pub mi: Option<f64>,
    pub ei: Option<f64>,
    pub poisson: Option<f64>,
    pub density: Option<f64>,
}

/// Lab table keyed by lowercase lithology.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Lab(pub BTreeMap<String, LabRow>);

impl Lab {
    pub fn from_table(t: &GtTable) -> Lab {
        let mut m = BTreeMap::new();
        for r in &t.records {
            if let Some(l) = r.text(GtField::Lith) {
                m.insert(
                    l.to_lowercase(),
                    LabRow {
                        ucs: r.num(GtField::Ucs),
                        is50: r.num(GtField::Is50),
                        mi: r.num(GtField::Mi),
                        ei: r.num(GtField::Ei),
                        poisson: r.num(GtField::Poisson),
                        density: r.num(GtField::Density),
                    },
                );
            }
        }
        Lab(m)
    }
    pub fn get(&self, lith: Option<&str>) -> Option<&LabRow> {
        lith.and_then(|l| self.0.get(&l.trim().to_lowercase()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClassifyOptions {
    pub correlation: Correlation,
    /// Fill RMR from Q (and Q from RMR) with `correlation` when one of them
    /// cannot be computed. Off by default.
    pub cross_fill: bool,
    /// Estimate RQD from the fracture frequency (Priest & Hudson) when RQD is
    /// not logged.
    pub rqd_from_ff: bool,
}

impl Default for ClassifyOptions {
    fn default() -> Self {
        ClassifyOptions {
            correlation: Correlation::Bieniawski,
            cross_fill: false,
            rqd_from_ff: true,
        }
    }
}

/// How a value was obtained when it was not measured directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Derived {
    UcsFromLab,
    Is50FromLab,
    RqdFromFf,
    SpacingFromFf,
    RmrGiven,
    QGiven,
    GsiGiven,
    RmrFromQ,
    QFromRmr,
    GsiFromRmr,
    GsiFromQ,
}

impl Derived {
    pub fn key(self) -> &'static str {
        match self {
            Derived::UcsFromLab => "ucs_lab",
            Derived::Is50FromLab => "is50_lab",
            Derived::RqdFromFf => "rqd_ff",
            Derived::SpacingFromFf => "spacing_ff",
            Derived::RmrGiven => "rmr_given",
            Derived::QGiven => "q_given",
            Derived::GsiGiven => "gsi_given",
            Derived::RmrFromQ => "rmr_q",
            Derived::QFromRmr => "q_rmr",
            Derived::GsiFromRmr => "gsi_rmr",
            Derived::GsiFromQ => "gsi_q",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Classification {
    pub parts: Option<RmrParts>,
    /// Basic RMR89 (no orientation adjustment).
    pub rmr: Option<f64>,
    /// RMR89' (dry), when the water rating is known.
    pub rmr_dry: Option<f64>,
    pub q_prime: Option<f64>,
    pub q: Option<f64>,
    pub gsi: Option<f64>,
    /// Rock mass class I..V (from RMR).
    pub class: Option<u8>,
    /// Parameters that prevented RMR / Q.
    pub rmr_missing: Vec<GtField>,
    pub q_missing: Vec<GtField>,
    /// Values outside the tables (field, value).
    pub invalid: Vec<(GtField, String)>,
    pub derived: Vec<Derived>,
}

impl Classification {
    pub fn is_classified(&self) -> bool {
        self.rmr.is_some() || self.q.is_some() || self.gsi.is_some()
    }
    /// log10 Q: Q spans orders of magnitude, so it is interpolated in log.
    pub fn log_q(&self) -> Option<f64> {
        self.q.filter(|q| *q > 0.0).map(f64::log10)
    }
}

/// A rating: a direct rating column wins, then the measurement.
fn rating(
    rec: &GtRecord,
    rating_field: GtField,
    max: f64,
    measured: Option<f64>,
    c: &mut Classification,
    missing: &[GtField],
) -> Option<f64> {
    if let Some(v) = rec.num(rating_field) {
        if (0.0..=max).contains(&v) {
            return Some(v);
        }
        c.invalid.push((rating_field, v.to_string()));
        return None;
    }
    if measured.is_none() {
        c.rmr_missing.extend_from_slice(missing);
    }
    measured
}

fn text_rating(rec: &GtRecord, f: GtField, parse: fn(&str) -> Option<f64>, c: &mut Classification) -> Option<f64> {
    match rec.text(f) {
        None => None,
        Some(t) => {
            let v = parse(t);
            if v.is_none() {
                c.invalid.push((f, t.to_string()));
            }
            v
        }
    }
}

fn infilling(t: &str) -> Option<f64> {
    let s = t.trim().to_lowercase();
    match s.as_str() {
        "none" | "ninguno" | "ninguna" | "sin relleno" | "limpia" | "limpio" => Some(6.0),
        _ => s.replace(',', ".").parse::<f64>().ok().and_then(rating_infilling),
    }
}

fn rmr_parts(rec: &GtRecord, lab: Option<&LabRow>, opts: &ClassifyOptions, c: &mut Classification) -> Option<RmrParts> {
    // Strength: UCS, then Is50, then laboratory by lithology.
    let measured_strength = rec
        .num(GtField::Ucs)
        .map(rating_ucs)
        .or_else(|| rec.num(GtField::Is50).and_then(rating_is50))
        .or_else(|| {
            let l = lab?;
            if let Some(u) = l.ucs {
                c.derived.push(Derived::UcsFromLab);
                Some(rating_ucs(u))
            } else {
                let r = l.is50.and_then(rating_is50);
                if r.is_some() {
                    c.derived.push(Derived::Is50FromLab);
                }
                r
            }
        });
    let strength = rating(rec, GtField::RStrength, 15.0, measured_strength, c, &[GtField::Ucs]);

    let ff = rec.num(GtField::Ff);
    let rqd_val = rec.num(GtField::Rqd).or_else(|| {
        let v = ff.filter(|_| opts.rqd_from_ff).and_then(rqd_from_ff);
        if v.is_some() {
            c.derived.push(Derived::RqdFromFf);
        }
        v
    });
    let rqd = rating(rec, GtField::RRqd, 20.0, rqd_val.map(rating_rqd), c, &[GtField::Rqd]);

    let sp = rec.num(GtField::Spacing).or_else(|| {
        let v = ff.and_then(spacing_from_ff);
        if v.is_some() {
            c.derived.push(Derived::SpacingFromFf);
        }
        v
    });
    let spacing = rating(
        rec,
        GtField::RSpacing,
        20.0,
        sp.map(rating_spacing),
        c,
        &[GtField::Spacing],
    );

    // Condition: total rating, or the five sub-ratings (all of them).
    let condition = if let Some(v) = rec.num(GtField::RCondition) {
        if (0.0..=30.0).contains(&v) {
            Some(v)
        } else {
            c.invalid.push((GtField::RCondition, v.to_string()));
            None
        }
    } else {
        let parts = [
            (
                GtField::Persistence,
                rec.num(GtField::Persistence).map(rating_persistence),
            ),
            (GtField::Aperture, rec.num(GtField::Aperture).map(rating_aperture)),
            (
                GtField::Roughness,
                text_rating(rec, GtField::Roughness, rating_roughness, c),
            ),
            (GtField::Infilling, text_rating(rec, GtField::Infilling, infilling, c)),
            (
                GtField::Weathering,
                text_rating(rec, GtField::Weathering, rating_weathering, c),
            ),
        ];
        let miss: Vec<GtField> = parts
            .iter()
            .filter(|(f, v)| v.is_none() && rec.text(*f).is_none())
            .map(|(f, _)| *f)
            .collect();
        if parts.iter().all(|(_, v)| v.is_some()) {
            Some(parts.iter().map(|(_, v)| v.unwrap()).sum())
        } else {
            if miss.len() == parts.len() {
                c.rmr_missing.push(GtField::RCondition);
            } else {
                c.rmr_missing.extend(miss);
            }
            None
        }
    };

    let water = match rec.num(GtField::RWater) {
        Some(v) if (0.0..=15.0).contains(&v) => Some(v),
        Some(v) => {
            c.invalid.push((GtField::RWater, v.to_string()));
            None
        }
        None => {
            let w = text_rating(rec, GtField::Water, rating_water, c);
            if w.is_none() && rec.text(GtField::Water).is_none() {
                c.rmr_missing.push(GtField::Water);
            }
            w
        }
    };
    Some(RmrParts {
        strength: strength?,
        rqd: rqd?,
        spacing: spacing?,
        condition: condition?,
        water: water?,
    })
}

/// Classifies one record.
pub fn classify(rec: &GtRecord, lab: &Lab, opts: &ClassifyOptions) -> Classification {
    let mut c = Classification::default();
    let lab_row = lab.get(rec.text(GtField::Lith));

    // RMR89: computed from parameters, else a given RMR.
    if let Some(p) = rmr_parts(rec, lab_row, opts, &mut c) {
        c.parts = Some(p);
        c.rmr = Some(p.total());
        c.rmr_dry = Some(p.dry_total());
        c.rmr_missing.clear();
    } else if let Some(r) = rec.num(GtField::Rmr) {
        if (0.0..=100.0).contains(&r) {
            c.rmr = Some(r);
            c.derived.push(Derived::RmrGiven);
            c.rmr_missing.clear();
            let w = rec
                .num(GtField::RWater)
                .or_else(|| rec.text(GtField::Water).and_then(rating_water));
            c.rmr_dry = w.map(|w| r - w + 15.0);
        } else {
            c.invalid.push((GtField::Rmr, r.to_string()));
        }
    }
    c.rmr_missing.sort();
    c.rmr_missing.dedup();

    // Q-system.
    let rqd = rec
        .num(GtField::Rqd)
        .or_else(|| rec.num(GtField::Ff).filter(|_| opts.rqd_from_ff).and_then(rqd_from_ff));
    let need = [
        (GtField::Rqd, rqd),
        (GtField::Jn, rec.num(GtField::Jn)),
        (GtField::Jr, rec.num(GtField::Jr)),
        (GtField::Ja, rec.num(GtField::Ja)),
    ];
    if let [Some(rqd), Some(jn), Some(jr), Some(ja)] = need.map(|(_, v)| v) {
        match q_prime(rqd, jn, jr, ja) {
            Ok(qp) => {
                c.q_prime = Some(qp);
                if let (Some(jw), Some(srf)) = (rec.num(GtField::Jw), rec.num(GtField::Srf)) {
                    match (QParams {
                        rqd,
                        jn,
                        jr,
                        ja,
                        jw,
                        srf,
                    })
                    .q()
                    {
                        Ok(q) => c.q = Some(q),
                        Err(crate::q::QError::OutOfRange(p, v)) => c.invalid.push((field_of(p), v.to_string())),
                    }
                } else {
                    c.q_missing.extend(
                        [GtField::Jw, GtField::Srf]
                            .into_iter()
                            .filter(|f| rec.num(*f).is_none()),
                    );
                }
            }
            Err(crate::q::QError::OutOfRange(p, v)) => c.invalid.push((field_of(p), v.to_string())),
        }
    } else {
        c.q_missing
            .extend(need.iter().filter(|(_, v)| v.is_none()).map(|(f, _)| *f));
    }
    if c.q.is_none() {
        if let Some(q) = rec.num(GtField::Q).filter(|q| *q > 0.0) {
            c.q = Some(q);
            c.derived.push(Derived::QGiven);
            c.q_missing.clear();
        }
    }

    // Cross-fill with the selected correlation (opt-in).
    if opts.cross_fill {
        match (c.rmr, c.q) {
            (None, Some(q)) => {
                if let Some(r) = opts.correlation.rmr_from_q(q) {
                    c.rmr = Some(r);
                    c.derived.push(Derived::RmrFromQ);
                }
            }
            (Some(r), None) => {
                c.q = Some(opts.correlation.q_from_rmr(r));
                c.derived.push(Derived::QFromRmr);
            }
            _ => {}
        }
    }

    // GSI: given, then RMR89', then Q'.
    if let Some(g) = rec.num(GtField::Gsi).filter(|g| (0.0..=100.0).contains(g)) {
        c.gsi = Some(g);
        c.derived.push(Derived::GsiGiven);
    } else if let Some(g) = c.rmr_dry.and_then(gsi_from_rmr89) {
        c.gsi = Some(g);
        c.derived.push(Derived::GsiFromRmr);
    } else if let Some(g) = c.q_prime.and_then(gsi_from_q_prime) {
        c.gsi = Some(g);
        c.derived.push(Derived::GsiFromQ);
    }
    c.class = c.rmr.map(crate::rmr_class);
    c
}

fn field_of(p: &str) -> GtField {
    match p {
        "RQD" => GtField::Rqd,
        "Jn" => GtField::Jn,
        "Jr" => GtField::Jr,
        "Ja" => GtField::Ja,
        "Jw" => GtField::Jw,
        _ => GtField::Srf,
    }
}

/// Summary of a classified table.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ClassifySummary {
    pub total: usize,
    pub with_rmr: usize,
    pub with_q: usize,
    pub with_gsi: usize,
    pub unclassified: usize,
    /// How many records lacked each parameter (RMR or Q).
    pub missing: BTreeMap<GtField, usize>,
    pub invalid: usize,
}

pub fn summarize(list: &[Classification]) -> ClassifySummary {
    let mut s = ClassifySummary {
        total: list.len(),
        ..Default::default()
    };
    for c in list {
        s.with_rmr += c.rmr.is_some() as usize;
        s.with_q += c.q.is_some() as usize;
        s.with_gsi += c.gsi.is_some() as usize;
        s.unclassified += (!c.is_classified()) as usize;
        s.invalid += (!c.invalid.is_empty()) as usize;
        let mut fs: Vec<GtField> = c.rmr_missing.iter().chain(&c.q_missing).copied().collect();
        fs.sort();
        fs.dedup();
        for f in fs {
            *s.missing.entry(f).or_default() += 1;
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use GtField::*;

    fn full() -> GtRecord {
        GtRecord::default()
            .with(Ucs, 150)
            .with(Rqd, 70)
            .with(Spacing, 0.5)
            .with(Persistence, 2)
            .with(Aperture, 0.5)
            .with(Roughness, "rugosa")
            .with(Infilling, "ninguno")
            .with(Weathering, "ligeramente meteorizada")
            .with(Water, "húmedo")
            .with(Jn, 9)
            .with(Jr, 1.5)
            .with(Ja, 1)
            .with(Jw, 1)
            .with(Srf, 1)
    }

    #[test]
    fn full_record() {
        let c = classify(&full(), &Lab::default(), &ClassifyOptions::default());
        // 12 + 13 + 10 + (4+4+5+6+5 = 24) + 10 = 69.
        assert_eq!(c.rmr, Some(69.0));
        assert_eq!(c.rmr_dry, Some(74.0));
        assert_eq!(c.gsi, Some(69.0));
        assert_eq!(c.class, Some(2));
        assert!((c.q.unwrap() - 70.0 / 9.0 * 1.5).abs() < 1e-9);
        assert!(c.rmr_missing.is_empty() && c.q_missing.is_empty() && c.invalid.is_empty());
        assert!(c.derived.contains(&Derived::GsiFromRmr));
    }

    #[test]
    fn missing_parameters_are_reported_not_invented() {
        let r = GtRecord::default().with(Rqd, 70).with(Jn, 9);
        let c = classify(&r, &Lab::default(), &ClassifyOptions::default());
        assert_eq!(c.rmr, None);
        assert_eq!(c.q, None);
        assert!(
            c.rmr_missing.contains(&Ucs) && c.rmr_missing.contains(&Spacing) && c.rmr_missing.contains(&RCondition)
        );
        assert_eq!(c.q_missing, vec![Jr, Ja]);
        assert!(!c.is_classified());
        // One condition sub-rating missing is named precisely.
        let r = full().with(Aperture, "");
        let c = classify(&r, &Lab::default(), &ClassifyOptions::default());
        assert_eq!(c.rmr, None);
        assert_eq!(c.rmr_missing, vec![Aperture]);
        assert!(c.q.is_some());
    }

    #[test]
    fn ratings_lab_and_fracture_frequency() {
        // Direct ratings, strength from the lab by lithology, spacing from FF.
        let lab = Lab([(
            "andesita".to_string(),
            LabRow {
                ucs: Some(80.0),
                ..Default::default()
            },
        )]
        .into());
        let r = GtRecord::default()
            .with(Lith, "Andesita")
            .with(Ff, 4)
            .with(RCondition, 20)
            .with(RWater, 15);
        let c = classify(&r, &lab, &ClassifyOptions::default());
        // 7 (80 MPa) + RQD(ff 4 -> 93.8 -> 20) + spacing 0.25 -> 10 + 20 + 15 = 72.
        assert_eq!(c.rmr, Some(72.0));
        assert!(c.derived.contains(&Derived::UcsFromLab) && c.derived.contains(&Derived::RqdFromFf));
        // Without permission no RQD is derived.
        let c = classify(
            &r,
            &lab,
            &ClassifyOptions {
                rqd_from_ff: false,
                ..Default::default()
            },
        );
        assert_eq!(c.rmr, None);
        assert!(c.rmr_missing.contains(&Rqd));
    }

    #[test]
    fn given_values_and_cross_fill() {
        let r = GtRecord::default().with(Rmr, 55).with(RWater, 10);
        let c = classify(&r, &Lab::default(), &ClassifyOptions::default());
        assert_eq!(
            (c.rmr, c.rmr_dry, c.gsi, c.q),
            (Some(55.0), Some(60.0), Some(55.0), None)
        );
        let c = classify(
            &r,
            &Lab::default(),
            &ClassifyOptions {
                cross_fill: true,
                ..Default::default()
            },
        );
        assert!((c.q.unwrap() - (11.0f64 / 9.0).exp()).abs() < 1e-9);
        assert!(c.derived.contains(&Derived::QFromRmr));
        let r = GtRecord::default().with(Q, 1);
        let c = classify(
            &r,
            &Lab::default(),
            &ClassifyOptions {
                cross_fill: true,
                ..Default::default()
            },
        );
        assert_eq!(c.rmr, Some(44.0));
        assert_eq!(c.class, Some(3));
    }

    #[test]
    fn invalid_values() {
        let r = full().with(Roughness, "brillante").with(Jw, 3);
        let c = classify(&r, &Lab::default(), &ClassifyOptions::default());
        assert_eq!(c.rmr, None);
        assert!(c.invalid.iter().any(|(f, _)| *f == Roughness));
        assert!(c.invalid.iter().any(|(f, _)| *f == Jw));
        let s = summarize(&[c, classify(&full(), &Lab::default(), &ClassifyOptions::default())]);
        assert_eq!((s.total, s.with_rmr, s.invalid), (2, 1, 1));
    }
}
