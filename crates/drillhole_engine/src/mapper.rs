//! Maps depth intervals onto the 3D trace.

use crate::desurvey::{min_curvature_step, slerp, tangent};
use crate::model::*;
use mining_core::{add, Vec3};

const GAP_TOL: f64 = 1e-6;

/// Position at depth `d`: before/after the trace it is clamped to the ends;
/// between stations it is minimum curvature from the previous station with the
/// direction interpolated spherically (not a linear chord).
pub fn interpolate_xyz(trace: &[SurveyNode], d: f64) -> Vec3 {
    let first = &trace[0];
    let last = trace.last().unwrap();
    if !d.is_finite() || d <= first.md {
        return first.xyz;
    }
    if d >= last.md {
        return last.xyz;
    }
    let i = trace.partition_point(|n| n.md <= d) - 1;
    let (n1, n2) = (&trace[i], &trace[i + 1]);
    let (t1, t2) = (tangent(n1.azimuth, n1.dip), tangent(n2.azimuth, n2.dip));
    let s = (d - n1.md) / (n2.md - n1.md);
    add(n1.xyz, min_curvature_step(t1, slerp(t1, t2, s), d - n1.md))
}

/// Splits `[from, to]` at survey depths strictly inside it.
pub fn split_at_nodes(from: f64, to: f64, trace: &[SurveyNode]) -> Vec<(f64, f64)> {
    let mut cuts: Vec<f64> = trace
        .iter()
        .map(|n| n.md)
        .filter(|&d| d > from + 1e-9 && d < to - 1e-9)
        .collect();
    cuts.sort_by(f64::total_cmp);
    let mut out = Vec::new();
    let mut a = from;
    for c in cuts {
        out.push((a, c));
        a = c;
    }
    out.push((a, to));
    out
}

fn build(hole: &str, from: f64, to: f64, trace: &[SurveyNode], source: SourceType) -> MappedInterval {
    MappedInterval {
        hole_id: hole.to_string(),
        from,
        to,
        from_xyz: interpolate_xyz(trace, from),
        to_xyz: interpolate_xyz(trace, to),
        centroid_xyz: interpolate_xyz(trace, (from + to) / 2.0),
        attributes: Default::default(),
        geo_attributes: Default::default(),
        lithology: String::new(),
        is_synthetic: false,
        source,
    }
}

/// Assay intervals (sorted, already validated). Gaps (R07) become synthetic
/// intervals whose attributes are all `None` (NaN in the spec).
pub fn map_assays(hole: &str, assays: &[Assay], trace: &[SurveyNode]) -> Vec<MappedInterval> {
    let mut list: Vec<&Assay> = assays.iter().collect();
    list.sort_by(|a, b| a.from.total_cmp(&b.from));
    let mut out = Vec::new();
    let mut prev_to: Option<f64> = None;
    for a in list {
        if let Some(p) = prev_to {
            if a.from > p + GAP_TOL {
                let mut g = build(hole, p, a.from, trace, SourceType::Assay);
                g.is_synthetic = true;
                out.push(g);
            }
        }
        for (f, t) in split_at_nodes(a.from, a.to, trace) {
            let mut m = build(hole, f, t, trace, SourceType::Assay);
            m.attributes = a.attributes.clone();
            out.push(m);
        }
        prev_to = Some(a.to);
    }
    out
}

/// Geology intervals; R07 gaps are also synthesised (improvement over legacy).
pub fn map_geo(hole: &str, geo: &[GeoLog], trace: &[SurveyNode]) -> Vec<MappedInterval> {
    let mut list: Vec<&GeoLog> = geo.iter().collect();
    list.sort_by(|a, b| a.from.total_cmp(&b.from));
    let mut out = Vec::new();
    let mut prev_to: Option<f64> = None;
    for g in list {
        if let Some(p) = prev_to {
            if g.from > p + GAP_TOL {
                let mut s = build(hole, p, g.from, trace, SourceType::Geo);
                s.is_synthetic = true;
                out.push(s);
            }
        }
        for (f, t) in split_at_nodes(g.from, g.to, trace) {
            let mut m = build(hole, f, t, trace, SourceType::Geo);
            m.geo_attributes = g.attributes.clone();
            m.lithology = g.lithology();
            out.push(m);
        }
        prev_to = Some(g.to);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desurvey::desurvey;
    use std::collections::BTreeMap;

    fn vertical_trace() -> Vec<SurveyNode> {
        let c = Collar {
            hole_id: "H".into(),
            x: 0.0,
            y: 0.0,
            z: 0.0,
            max_depth: 100.0,
            azimuth: None,
            dip: None,
        };
        desurvey(&c, &[])
    }
    fn assay(f: f64, t: f64) -> Assay {
        Assay {
            hole_id: "H".into(),
            from: f,
            to: t,
            attributes: BTreeMap::new(),
        }
    }

    #[test]
    fn interpolation_on_vertical_hole() {
        let t = vertical_trace();
        assert!((interpolate_xyz(&t, 25.0)[2] + 25.0).abs() < 1e-9);
        assert!((interpolate_xyz(&t, 999.0)[2] + 100.0).abs() < 1e-9);
    }

    #[test]
    fn gaps_become_synthetic() {
        let t = vertical_trace();
        let m = map_assays("H", &[assay(0.0, 10.0), assay(15.0, 20.0)], &t);
        assert_eq!(m.len(), 3);
        assert!(m[1].is_synthetic && (m[1].from - 10.0).abs() < 1e-12 && (m[1].to - 15.0).abs() < 1e-12);
    }

    #[test]
    fn interval_splits_at_survey_nodes() {
        let c = Collar {
            hole_id: "H".into(),
            x: 0.0,
            y: 0.0,
            z: 0.0,
            max_depth: 100.0,
            azimuth: None,
            dip: None,
        };
        let sv = |d| Survey {
            hole_id: "H".into(),
            depth: d,
            azimuth: 0.0,
            dip: -90.0,
        };
        let t = desurvey(&c, &[sv(0.0), sv(10.0), sv(20.0)]);
        assert_eq!(
            split_at_nodes(5.0, 25.0, &t),
            vec![(5.0, 10.0), (10.0, 20.0), (20.0, 25.0)]
        );
    }
}
