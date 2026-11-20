//! Drillholes -> composites. Domains come from a geology attribute: each
//! rule lists the attribute values that belong to a domain.

use drillhole_engine::pipeline::HoleModel;
use drillhole_engine::MappedInterval;
use mining_core::Vec3;

use crate::composite::{composite_hole, CompositeConfig, Interval};
use crate::{par_collect, Sample};

#[derive(Debug, Clone, PartialEq)]
pub struct DomainRule {
    pub name: String,
    /// Geology attribute values (compared case-insensitively) of this domain.
    pub values: Vec<String>,
}

/// A composite with everything needed to draw and export it.
#[derive(Debug, Clone, PartialEq)]
pub struct Composite3d {
    pub hole: String,
    pub hole_index: u32,
    pub from: f64,
    pub to: f64,
    pub from_xyz: Vec3,
    pub to_xyz: Vec3,
    /// Point on the trace at mid-depth.
    pub xyz: Vec3,
    pub value: f64,
    pub domain: u32,
}

impl Composite3d {
    pub fn length(&self) -> f64 {
        self.to - self.from
    }

    pub fn sample(&self) -> Sample {
        Sample {
            xyz: self.xyz,
            value: self.value,
            length: self.length(),
            hole: self.hole_index,
            domain: self.domain,
        }
    }
}

fn domain_of(value: &str, rules: &[DomainRule]) -> u32 {
    let v = value.trim().to_lowercase();
    rules
        .iter()
        .position(|r| r.values.iter().any(|x| x.trim().to_lowercase() == v))
        .map_or(0, |i| i as u32 + 1)
}

/// Splits the assay intervals of a hole at the geology contacts and tags
/// each piece with its domain (0 where the geology has a gap).
fn domain_intervals(hole: &HoleModel, attr: &str, geo_attr: &str, rules: &[DomainRule]) -> Vec<Interval> {
    let mut assays: Vec<&MappedInterval> = hole.assays.iter().collect();
    assays.sort_by(|a, b| a.from.total_cmp(&b.from));
    let single = geo_attr.is_empty() || rules.is_empty();
    // Assay intervals are already split at survey stations; merge nothing here.
    let mut geo: Vec<(f64, f64, u32)> = hole
        .geo
        .iter()
        .filter(|g| !g.is_synthetic)
        .filter_map(|g| {
            g.geo_attributes
                .get(geo_attr)
                .map(|v| (g.from, g.to, domain_of(v, rules)))
        })
        .collect();
    geo.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut out = Vec::new();
    for a in assays {
        let value = if a.is_synthetic {
            None
        } else {
            a.attributes.get(attr).copied().flatten()
        };
        if single {
            out.push(Interval {
                from: a.from,
                to: a.to,
                value,
                domain: 1,
            });
            continue;
        }
        let mut cursor = a.from;
        for &(gf, gt, d) in geo.iter().filter(|g| g.1 > a.from && g.0 < a.to) {
            let (s, e) = (gf.max(a.from), gt.min(a.to));
            if s > cursor + 1e-9 {
                out.push(Interval {
                    from: cursor,
                    to: s,
                    value,
                    domain: 0,
                });
            }
            if e > s + 1e-9 {
                out.push(Interval {
                    from: s.max(cursor),
                    to: e,
                    value,
                    domain: d,
                });
            }
            cursor = cursor.max(e);
        }
        if a.to > cursor + 1e-9 {
            out.push(Interval {
                from: cursor,
                to: a.to,
                value,
                domain: 0,
            });
        }
    }
    // Contiguous unsampled pieces of the same domain merge naturally in compositing.
    out
}

/// Composites every hole (in parallel). `attr` is the assay attribute.
pub fn composites_from_holes(
    holes: &[HoleModel],
    attr: &str,
    geo_attr: &str,
    rules: &[DomainRule],
    cfg: &CompositeConfig,
) -> Vec<Composite3d> {
    let per_hole = par_collect(holes.len(), |h| {
        let hole = &holes[h];
        let ivs = domain_intervals(hole, attr, geo_attr, rules);
        composite_hole(&ivs, cfg)
            .into_iter()
            .map(|c| Composite3d {
                hole: hole.collar.hole_id.clone(),
                hole_index: h as u32,
                from: c.from,
                to: c.to,
                from_xyz: drillhole_engine::mapper::interpolate_xyz(&hole.trace, c.from),
                to_xyz: drillhole_engine::mapper::interpolate_xyz(&hole.trace, c.to),
                xyz: drillhole_engine::mapper::interpolate_xyz(&hole.trace, 0.5 * (c.from + c.to)),
                value: c.value,
                domain: c.domain,
            })
            .collect::<Vec<_>>()
    });
    per_hole.into_iter().flatten().collect()
}

/// First contact of each hole from `above` values to `below` values (case
/// insensitive): the top of the first `below` interval that follows an
/// `above` interval.
pub fn contact_points(holes: &[HoleModel], geo_attr: &str, above: &[String], below: &[String]) -> Vec<Vec3> {
    let norm = |v: &[String]| -> Vec<String> { v.iter().map(|s| s.trim().to_lowercase()).collect() };
    let (above, below) = (norm(above), norm(below));
    let mut out = Vec::new();
    for h in holes {
        let mut geo: Vec<&MappedInterval> = h.geo.iter().filter(|g| !g.is_synthetic).collect();
        geo.sort_by(|a, b| a.from.total_cmp(&b.from));
        let mut seen_above = false;
        for g in geo {
            let v = g
                .geo_attributes
                .get(geo_attr)
                .map(|s| s.trim().to_lowercase())
                .unwrap_or_default();
            if above.contains(&v) {
                seen_above = true;
            } else if below.contains(&v) {
                if seen_above {
                    out.push(g.from_xyz);
                }
                break;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use drillhole_engine::pipeline::build_holes;
    use drillhole_engine::{Assay, Collar, DataSet, GeoLog};
    use std::collections::BTreeMap;

    fn dataset() -> DataSet {
        let geo = |h: &str, f, t, v: &str| GeoLog {
            hole_id: h.into(),
            from: f,
            to: t,
            attributes: BTreeMap::from([("rock".to_string(), v.to_string())]),
        };
        let assay = |h: &str, f, t, v| Assay {
            hole_id: h.into(),
            from: f,
            to: t,
            attributes: BTreeMap::from([("au".to_string(), Some(v))]),
        };
        DataSet {
            collars: vec![
                Collar {
                    hole_id: "A".into(),
                    x: 0.0,
                    y: 0.0,
                    z: 100.0,
                    max_depth: 20.0,
                    azimuth: None,
                    dip: None,
                },
                Collar {
                    hole_id: "B".into(),
                    x: 50.0,
                    y: 0.0,
                    z: 100.0,
                    max_depth: 20.0,
                    azimuth: None,
                    dip: None,
                },
            ],
            geo: vec![
                geo("A", 0.0, 6.0, "OXIDO"),
                geo("A", 6.0, 20.0, "SULFURO"),
                geo("B", 0.0, 10.0, "oxido"),
                geo("B", 10.0, 20.0, "Sulfuro"),
            ],
            assays: (0..20)
                .flat_map(|i| ["A", "B"].map(|h| assay(h, i as f64, i as f64 + 1.0, if i < 6 { 1.0 } else { 5.0 })))
                .collect(),
            ..Default::default()
        }
    }

    fn rules() -> Vec<DomainRule> {
        vec![
            DomainRule {
                name: "Óxidos".into(),
                values: vec!["OXIDO".into()],
            },
            DomainRule {
                name: "Sulfuros".into(),
                values: vec!["sulfuro".into()],
            },
        ]
    }

    #[test]
    fn composites_split_at_geology_contacts_with_case_insensitive_domains() {
        let holes = build_holes(&dataset());
        let comps = composites_from_holes(
            &holes,
            "au",
            "rock",
            &rules(),
            &CompositeConfig {
                length: 4.0,
                ..Default::default()
            },
        );
        for c in &comps {
            let want = if c.hole == "A" { 6.0 } else { 10.0 };
            let ox = c.to <= want + 1e-9;
            assert_eq!(c.domain, if ox { 1 } else { 2 }, "{c:?}");
            if c.hole == "A" {
                assert_eq!(c.value, if ox { 1.0 } else { 5.0 });
            }
        }
        // Hole A: oxide 0..6 -> 4 + 2 (2 >= 50 % kept), sulphide 6..20 -> 4,4,4,2.
        let a: Vec<_> = comps.iter().filter(|c| c.hole == "A").collect();
        assert_eq!(a.len(), 6);
        // Positions lie on the (vertical) trace.
        assert!((a[0].xyz[2] - 98.0).abs() < 1e-9 && a[0].xyz[0] == 0.0);
        assert_eq!(a[0].sample().domain, 1);
    }

    #[test]
    fn no_geology_means_a_single_domain() {
        let holes = build_holes(&dataset());
        let comps = composites_from_holes(&holes, "au", "", &[], &CompositeConfig::default());
        assert!(comps.iter().all(|c| c.domain == 1));
    }

    #[test]
    fn contacts_from_holes() {
        let holes = build_holes(&dataset());
        let pts = contact_points(&holes, "rock", &["oxido".into()], &["SULFURO".into()]);
        assert_eq!(pts.len(), 2);
        assert!((pts[0][2] - 94.0).abs() < 1e-9 && (pts[1][2] - 90.0).abs() < 1e-9);
        // Reversed order has no contacts.
        assert!(contact_points(&holes, "rock", &["sulfuro".into()], &["oxido".into()]).is_empty());
    }
}
