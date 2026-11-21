//! Contact points of the vein: mapped hanging-wall / footwall lines and the
//! vein intercepts of drillholes and channels.

use drillhole_engine::model::MappedInterval;
use drillhole_engine::pipeline::HoleModel;
use mining_core::{dist, lerp, Vec3};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Hanging,
    Foot,
}

impl Side {
    pub fn key(self) -> &'static str {
        match self {
            Side::Hanging => "hanging_wall",
            Side::Foot => "footwall",
        }
    }
}

/// A point on one of the vein walls; `source` names where it comes from
/// (layer, polyline handle, hole id).
#[derive(Debug, Clone, PartialEq)]
pub struct Contact {
    pub p: Vec3,
    pub side: Side,
    pub source: String,
}

/// The vein crossed by a hole or channel between `from` and `to`.
#[derive(Debug, Clone, PartialEq)]
pub struct Intercept {
    pub hole: String,
    pub from: f64,
    pub to: f64,
    pub a: Vec3,
    pub b: Vec3,
}

fn is_vein(m: &MappedInterval, attr: &str, values: &[String]) -> bool {
    let a = attr.trim().to_lowercase();
    m.geo_attributes
        .iter()
        .any(|(k, v)| k.trim().to_lowercase() == a && values.iter().any(|x| x.trim().eq_ignore_ascii_case(v.trim())))
}

/// Vein intercepts: consecutive geology intervals whose `attr` is one of
/// `values` (e.g. `tipo` = `VETA`), merged per hole.
pub fn intercepts(holes: &[HoleModel], attr: &str, values: &[String]) -> Vec<Intercept> {
    let mut out = Vec::new();
    for h in holes {
        let mut cur: Option<Intercept> = None;
        let mut list: Vec<&MappedInterval> = h.geo.iter().filter(|m| is_vein(m, attr, values)).collect();
        list.sort_by(|a, b| a.from.total_cmp(&b.from));
        for m in list {
            match cur.as_mut() {
                Some(c) if m.from <= c.to + 0.01 => {
                    c.to = m.to;
                    c.b = m.to_xyz;
                }
                _ => {
                    if let Some(c) = cur.take() {
                        out.push(c);
                    }
                    cur = Some(Intercept {
                        hole: h.collar.hole_id.clone(),
                        from: m.from,
                        to: m.to,
                        a: m.from_xyz,
                        b: m.to_xyz,
                    });
                }
            }
        }
        if let Some(c) = cur {
            out.push(c);
        }
    }
    out
}

/// Points of a polyline every `step` at most (vertices kept).
pub fn densify(points: &[Vec3], step: f64) -> Vec<Vec3> {
    let mut out = Vec::new();
    for w in points.windows(2) {
        let n = if step > 0.0 {
            (dist(w[0], w[1]) / step).ceil().max(1.0) as usize
        } else {
            1
        };
        for k in 0..n {
            out.push(lerp(w[0], w[1], k as f64 / n as f64));
        }
    }
    if let Some(l) = points.last() {
        out.push(*l);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use drillhole_engine::model::{Collar, DataSet, GeoLog, Survey};

    #[test]
    fn intercepts_merge_vein_intervals() {
        let g = |f: f64, t: f64, tipo: &str| GeoLog {
            hole_id: "CH-1".into(),
            from: f,
            to: t,
            attributes: [("tipo".to_string(), tipo.to_string())].into(),
        };
        let ds = DataSet {
            collars: vec![Collar {
                hole_id: "CH-1".into(),
                x: 0.0,
                y: 0.0,
                z: 0.0,
                max_depth: 2.0,
                azimuth: Some(90.0),
                dip: Some(0.0),
            }],
            surveys: vec![Survey {
                hole_id: "CH-1".into(),
                depth: 0.0,
                azimuth: 90.0,
                dip: 0.0,
            }],
            geo: vec![
                g(0.0, 0.3, "CAJA"),
                g(0.3, 0.8, "veta"),
                g(0.8, 1.2, "VETA"),
                g(1.2, 2.0, "CAJA"),
            ],
            ..Default::default()
        };
        let holes = drillhole_engine::pipeline::build_holes(&ds);
        let i = intercepts(&holes, "Tipo", &["VETA".into()]);
        assert_eq!(i.len(), 1);
        assert_eq!((i[0].from, i[0].to), (0.3, 1.2));
        assert!(
            (i[0].a[0] - 0.3).abs() < 1e-9 && (i[0].b[0] - 1.2).abs() < 1e-9,
            "{:?}",
            i[0]
        );
        assert!(intercepts(&holes, "tipo", &["OTRA".into()]).is_empty());
    }

    #[test]
    fn densify_keeps_vertices() {
        let d = densify(&[[0.0; 3], [10.0, 0.0, 0.0], [10.0, 3.0, 0.0]], 4.0);
        // 10 m in steps of 4 -> 3 points, 3 m -> 1 point, plus the last vertex.
        assert_eq!(d.len(), 3 + 1 + 1);
        assert_eq!(d[0], [0.0; 3]);
        assert_eq!(*d.last().unwrap(), [10.0, 3.0, 0.0]);
        assert!(d.windows(2).all(|w| dist(w[0], w[1]) <= 4.0 + 1e-9));
    }
}
