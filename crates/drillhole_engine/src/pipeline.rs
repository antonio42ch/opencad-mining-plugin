//! Per-hole processing of a validated data set: desurvey + interval mapping.
//! Runs on all cores with the `parallel` feature.

use std::collections::HashMap;

use crate::desurvey::desurvey;
use crate::mapper::{map_assays, map_geo};
use crate::model::*;

/// Everything needed to draw one hole.
#[derive(Debug, Clone)]
pub struct HoleModel {
    pub collar: Collar,
    pub trace: Vec<SurveyNode>,
    pub assays: Vec<MappedInterval>,
    pub geo: Vec<MappedInterval>,
}

struct Grouped<'a> {
    surveys: HashMap<String, Vec<Survey>>,
    assays: HashMap<String, Vec<Assay>>,
    geo: HashMap<String, Vec<GeoLog>>,
    collars: &'a [Collar],
}

fn group(ds: &DataSet) -> Grouped<'_> {
    let mut g = Grouped {
        surveys: HashMap::new(),
        assays: HashMap::new(),
        geo: HashMap::new(),
        collars: &ds.collars,
    };
    for s in &ds.surveys {
        g.surveys.entry(hole_key(&s.hole_id)).or_default().push(s.clone());
    }
    for a in &ds.assays {
        g.assays.entry(hole_key(&a.hole_id)).or_default().push(a.clone());
    }
    for x in &ds.geo {
        g.geo.entry(hole_key(&x.hole_id)).or_default().push(x.clone());
    }
    g
}

fn build_one(g: &Grouped, c: &Collar) -> HoleModel {
    let k = hole_key(&c.hole_id);
    let empty_s = Vec::new();
    let trace = desurvey(c, g.surveys.get(&k).unwrap_or(&empty_s));
    let assays = g
        .assays
        .get(&k)
        .map(|a| map_assays(&c.hole_id, a, &trace))
        .unwrap_or_default();
    let geo = g
        .geo
        .get(&k)
        .map(|x| map_geo(&c.hole_id, x, &trace))
        .unwrap_or_default();
    HoleModel {
        collar: c.clone(),
        trace,
        assays,
        geo,
    }
}

/// Builds every hole of a **validated** data set, in collar order.
pub fn build_holes(ds: &DataSet) -> Vec<HoleModel> {
    let g = group(ds);
    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;
        g.collars.par_iter().map(|c| build_one(&g, c)).collect()
    }
    #[cfg(not(feature = "parallel"))]
    {
        g.collars.iter().map(|c| build_one(&g, c)).collect()
    }
}

/// Range scope for grade colouring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RangeScope {
    /// One min/max for the whole project (comparable colours across holes).
    Global,
    /// Min/max per hole (the legacy behaviour).
    PerHole,
}

/// `(min, max)` of an assay attribute over non-synthetic intervals with value.
pub fn attribute_range<'a>(intervals: impl IntoIterator<Item = &'a MappedInterval>, attr: &str) -> Option<(f64, f64)> {
    intervals
        .into_iter()
        .filter(|m| !m.is_synthetic)
        .filter_map(|m| m.attributes.get(attr).copied().flatten())
        .filter(|v| v.is_finite())
        .fold(None, |acc, v| match acc {
            None => Some((v, v)),
            Some((lo, hi)) => Some((lo.min(v), hi.max(v))),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn builds_all_holes_in_order_and_global_range() {
        let ds = DataSet {
            collars: (0..20)
                .map(|i| Collar {
                    hole_id: format!("H{i}"),
                    x: i as f64,
                    y: 0.0,
                    z: 0.0,
                    max_depth: 10.0,
                    azimuth: None,
                    dip: None,
                })
                .collect(),
            assays: (0..20)
                .map(|i| Assay {
                    hole_id: format!("h{i}"),
                    from: 0.0,
                    to: 5.0,
                    attributes: BTreeMap::from([("cu".to_string(), Some(i as f64))]),
                })
                .collect(),
            ..Default::default()
        };
        let holes = build_holes(&ds);
        assert_eq!(holes.len(), 20);
        assert_eq!(holes[7].collar.hole_id, "H7");
        assert_eq!(holes[7].assays.len(), 1);
        assert_eq!(
            attribute_range(holes.iter().flat_map(|h| &h.assays), "cu"),
            Some((0.0, 19.0))
        );
    }
}
