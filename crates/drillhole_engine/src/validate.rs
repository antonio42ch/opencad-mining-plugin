//! Validation rules. Pure: takes the raw data set and returns a cleaned copy
//! plus messages. The input (and its collars) is never mutated.

use std::collections::HashMap;

use mining_core::normalize360;

use crate::model::*;

const OVERLAP_TOL: f64 = 0.001;

trait Interval {
    fn hole(&self) -> &str;
    fn from(&self) -> f64;
    fn to(&self) -> f64;
    fn set_to(&mut self, v: f64);
}
macro_rules! impl_interval {
    ($t:ty) => {
        impl Interval for $t {
            fn hole(&self) -> &str {
                &self.hole_id
            }
            fn from(&self) -> f64 {
                self.from
            }
            fn to(&self) -> f64 {
                self.to
            }
            fn set_to(&mut self, v: f64) {
                self.to = v
            }
        }
    };
}
impl_interval!(Assay);
impl_interval!(GeoLog);

fn msg(sev: Severity, hole: &str, rule: &'static str, message: String) -> ValidationMessage {
    ValidationMessage {
        severity: sev,
        hole_id: hole.to_string(),
        rule,
        message,
        row: None,
    }
}

pub fn validate(input: &DataSet) -> (DataSet, Vec<ValidationMessage>) {
    let mut out = input.clone();
    let mut m = Vec::new();

    // R01: orphan records.
    let keys: HashMap<String, usize> = out
        .collars
        .iter()
        .enumerate()
        .map(|(i, c)| (hole_key(&c.hole_id), i))
        .collect();
    macro_rules! drop_orphans {
        ($v:expr, $name:expr) => {
            $v.retain(|r| {
                let ok = keys.contains_key(&hole_key(&r.hole_id));
                if !ok {
                    m.push(msg(
                        Severity::Warning,
                        &r.hole_id,
                        "R01",
                        format!("{} sin collar, descartado", $name),
                    ));
                }
                ok
            });
        };
    }
    drop_orphans!(out.surveys, "Survey");
    drop_orphans!(out.assays, "Assay");
    drop_orphans!(out.geo, "Geo");

    // Collars with unusable coordinates cannot be drawn.
    out.collars.retain(|c| {
        let ok = c.x.is_finite() && c.y.is_finite() && c.z.is_finite();
        if !ok {
            m.push(msg(
                Severity::Error,
                &c.hole_id,
                "R05",
                "Collar con coordenadas no numéricas, descartado".into(),
            ));
        }
        ok
    });

    // R02: azimuth normalisation, dip clamp (Survey only).
    for s in &mut out.surveys {
        if !s.depth.is_finite() || !s.azimuth.is_finite() || !s.dip.is_finite() {
            continue; // removed below
        }
        s.azimuth = normalize360(s.azimuth);
        if !(-90.0..=90.0).contains(&s.dip) {
            m.push(msg(
                Severity::Warning,
                &s.hole_id,
                "R02",
                format!("Dip {} fuera de [-90, 90], limitado", s.dip),
            ));
            s.dip = s.dip.clamp(-90.0, 90.0);
        }
    }
    out.surveys.retain(|s| {
        let ok = s.depth.is_finite() && s.azimuth.is_finite() && s.dip.is_finite();
        if !ok {
            m.push(msg(
                Severity::Error,
                &s.hole_id,
                "R05",
                "Survey con valores no numéricos, descartado".into(),
            ));
        }
        ok
    });

    // R05 + R06 on intervals.
    out.assays = fix_intervals(std::mem::take(&mut out.assays), &mut m);
    out.geo = fix_intervals(std::mem::take(&mut out.geo), &mut m);

    // R03: EOH exceeded -> raise max_depth on the *cleaned copy* and report it.
    let mut deepest: HashMap<String, f64> = HashMap::new();
    let mut bump = |h: &str, d: f64| {
        let e = deepest.entry(hole_key(h)).or_insert(f64::NEG_INFINITY);
        if d > *e {
            *e = d;
        }
    };
    for s in &out.surveys {
        bump(&s.hole_id, s.depth);
    }
    for a in &out.assays {
        bump(&a.hole_id, a.to);
    }
    for g in &out.geo {
        bump(&g.hole_id, g.to);
    }
    for c in &mut out.collars {
        if let Some(&d) = deepest.get(&hole_key(&c.hole_id)) {
            if c.max_depth.is_nan() || c.max_depth < d {
                m.push(msg(
                    Severity::Warning,
                    &c.hole_id,
                    "R03",
                    format!("EOH excedido: MaxDepth {} -> {}", c.max_depth, d),
                ));
                c.max_depth = d;
            }
        }
    }
    (out, m)
}

fn fix_intervals<T: Interval>(items: Vec<T>, m: &mut Vec<ValidationMessage>) -> Vec<T> {
    // R05: NaN From is rejected explicitly; NaN To -> From + 0.01; From >= To rejected.
    let mut ok: Vec<T> = Vec::new();
    for mut it in items {
        if !it.from().is_finite() {
            m.push(msg(
                Severity::Error,
                it.hole(),
                "R05",
                "Intervalo sin From válido, descartado".into(),
            ));
            continue;
        }
        if !it.to().is_finite() {
            let f = it.from();
            it.set_to(f + 0.01);
        }
        if it.from() >= it.to() {
            m.push(msg(
                Severity::Error,
                it.hole(),
                "R05",
                format!("From {} >= To {}, descartado", it.from(), it.to()),
            ));
            continue;
        }
        ok.push(it);
    }

    // R06: per hole, sorted by From, against the furthest accepted To.
    let mut by_hole: HashMap<String, Vec<T>> = HashMap::new();
    for it in ok {
        by_hole.entry(hole_key(it.hole())).or_default().push(it);
    }
    let mut keys: Vec<String> = by_hole.keys().cloned().collect();
    keys.sort();
    let mut result = Vec::new();
    for k in keys {
        let mut list = by_hole.remove(&k).unwrap();
        list.sort_by(|a, b| a.from().total_cmp(&b.from()));
        let mut accepted: Vec<T> = Vec::new();
        for it in list {
            if let Some(prev) = accepted.last_mut() {
                let prev_to = prev.to();
                if it.from() < prev_to - OVERLAP_TOL {
                    m.push(msg(
                        Severity::Fatal,
                        it.hole(),
                        "R06",
                        format!(
                            "Solape [{}, {}] con intervalo previo hasta {}, descartado",
                            it.from(),
                            it.to(),
                            prev_to
                        ),
                    ));
                    continue;
                } else if it.from() < prev_to {
                    prev.set_to(it.from());
                }
            }
            accepted.push(it);
        }
        result.extend(accepted);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn collar(id: &str, md: f64) -> Collar {
        Collar {
            hole_id: id.into(),
            x: 0.0,
            y: 0.0,
            z: 0.0,
            max_depth: md,
            azimuth: None,
            dip: None,
        }
    }
    fn assay(id: &str, f: f64, t: f64) -> Assay {
        Assay {
            hole_id: id.into(),
            from: f,
            to: t,
            attributes: BTreeMap::new(),
        }
    }

    #[test]
    fn orphans_case_insensitive_and_eoh_not_mutating_input() {
        let ds = DataSet {
            collars: vec![collar("DH1", 10.0)],
            assays: vec![assay("dh1", 0.0, 12.0), assay("zzz", 0.0, 1.0)],
            ..Default::default()
        };
        let (out, msgs) = validate(&ds);
        assert_eq!(out.assays.len(), 1);
        assert_eq!(out.collars[0].max_depth, 12.0);
        assert_eq!(ds.collars[0].max_depth, 10.0, "input must stay untouched");
        assert!(msgs.iter().any(|x| x.rule == "R01"));
        assert!(msgs.iter().any(|x| x.rule == "R03"));
    }

    #[test]
    fn nan_from_is_rejected_and_nan_to_defaults() {
        let ds = DataSet {
            collars: vec![collar("A", 10.0)],
            assays: vec![assay("A", f64::NAN, 2.0), assay("A", 1.0, f64::NAN)],
            ..Default::default()
        };
        let (out, _) = validate(&ds);
        assert_eq!(out.assays.len(), 1);
        assert!((out.assays[0].to - 1.01).abs() < 1e-12);
    }

    #[test]
    fn overlap_rules() {
        let ds = DataSet {
            collars: vec![collar("A", 10.0)],
            assays: vec![assay("A", 0.0, 2.0), assay("A", 1.9995, 3.0), assay("A", 2.5, 4.0)],
            ..Default::default()
        };
        let (out, msgs) = validate(&ds);
        // 1.9995 is within tolerance -> prev.to trimmed; 2.5 vs prev.to=3.0 -> Fatal drop? No:
        // after the trim prev is [1.9995,3.0]; 2.5 < 3.0 - tol -> Fatal.
        assert_eq!(out.assays.len(), 2);
        assert!((out.assays[0].to - 1.9995).abs() < 1e-12);
        assert!(msgs.iter().any(|x| x.severity == Severity::Fatal));
    }
}
