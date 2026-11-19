//! Solids from four boundary polylines.
//!
//! * Horizontal mode: roof, floor, left, right (rectangular section whose
//!   height comes from roof/floor Z). Tangent is horizontal by default; use
//!   `full_3d_tangent` for steep ramps.
//! * Vertical mode (raises/shafts): front, back, left, right, full 3D frame.

use mining_core::{add, dist, dot, lerp, mean, normalize_or, scale, sub, Vec3, Z};

use crate::error::SolidWarning;
use crate::frame::aaa_frame;
use crate::polyline::{clean_vertices, sample_by_count, sample_by_distance};
use crate::{Ring, SolidError};

const COARSE_N: usize = 300;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundaryMode {
    Horizontal { full_3d_tangent: bool },
    Vertical,
}

/// The four boundaries. Horizontal: a = roof, b = floor. Vertical: a = front, b = back.
pub struct Boundaries<'a> {
    pub a: &'a [Vec3],
    pub b: &'a [Vec3],
    pub left: &'a [Vec3],
    pub right: &'a [Vec3],
    /// Manual reversal of each polyline (a, b, left, right).
    pub flips: [bool; 4],
}

pub struct BoundaryResult {
    pub rings: Vec<Ring>,
    pub warnings: Vec<SolidWarning>,
}

/// Reverses `x` when its ends match `reference` better reversed.
pub fn align_direction(reference: &[Vec3], x: &mut [Vec3]) {
    let (rf, rl) = (reference[0], *reference.last().unwrap());
    let (xf, xl) = (x[0], *x.last().unwrap());
    if dist(rf, xl) + dist(rl, xf) < dist(rf, xf) + dist(rl, xl) {
        x.reverse();
    }
}

/// Intersection of a polyline with the plane through `origin` with `normal`:
/// the crossing closest to `origin`, or `None` if the polyline never crosses.
pub fn plane_intersect(verts: &[Vec3], origin: Vec3, normal: Vec3) -> Option<Vec3> {
    let d = dot(normal, origin);
    let s: Vec<f64> = verts.iter().map(|v| dot(normal, *v) - d).collect();
    let mut best: Option<(f64, Vec3)> = None;
    for i in 0..verts.len().saturating_sub(1) {
        if s[i] * s[i + 1] <= 0.0 {
            let den = s[i + 1] - s[i];
            let t = if den.abs() > 1e-12 {
                (-s[i] / den).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let p = lerp(verts[i], verts[i + 1], t);
            let dd = dist(p, origin);
            if best.is_none_or(|(b, _)| dd < b) {
                best = Some((dd, p));
            }
        }
    }
    best.map(|(_, p)| p)
}

fn nearest_vertex(verts: &[Vec3], origin: Vec3, normal: Vec3) -> Vec3 {
    let d = dot(normal, origin);
    *verts
        .iter()
        .min_by(|a, b| (dot(normal, **a) - d).abs().total_cmp(&(dot(normal, **b) - d).abs()))
        .unwrap()
}

pub fn boundary_rings(b: &Boundaries, interval: f64, mode: BoundaryMode) -> Result<BoundaryResult, SolidError> {
    if interval.is_nan() || interval <= 0.0 {
        return Err(SolidError::InvalidInterval(interval));
    }
    let names: [&'static str; 4] = match mode {
        BoundaryMode::Horizontal { .. } => ["roof", "floor", "left", "right"],
        BoundaryMode::Vertical => ["front", "back", "left", "right"],
    };
    let mut lines: Vec<Vec<Vec3>> = Vec::new();
    for (i, src) in [b.a, b.b, b.left, b.right].into_iter().enumerate() {
        let mut v = clean_vertices(src.to_vec());
        if v.len() < 2 {
            return Err(SolidError::TooFewVertices { what: names[i] });
        }
        if b.flips[i] {
            v.reverse();
        }
        lines.push(v);
    }
    let reference = lines[0].clone();
    for l in lines.iter_mut().skip(1) {
        align_direction(&reference, l);
    }

    // Coarse centre line = mean of the four boundaries at equal arc fractions.
    let coarse: Vec<Vec<Vec3>> = lines.iter().map(|l| sample_by_count(l, COARSE_N)).collect();
    let centre: Vec<Vec3> = (0..COARSE_N)
        .map(|i| mean(&[coarse[0][i], coarse[1][i], coarse[2][i], coarse[3][i]]))
        .collect();
    let st = sample_by_distance(&centre, interval);
    if st.len() < 2 {
        return Err(SolidError::IntervalTooLong {
            interval,
            length: crate::polyline::total_length(&centre),
        });
    }

    let n = st.len();
    let mut tangents: Vec<Vec3> = Vec::with_capacity(n);
    for i in 0..n {
        let (p, q) = if i + 1 < n {
            (st[i], st[i + 1])
        } else {
            (st[i - 1], st[i])
        };
        let d = sub(q, p);
        tangents.push(match mode {
            BoundaryMode::Horizontal { full_3d_tangent: false } => normalize_or([d[0], d[1], 0.0], [0.0, 1.0, 0.0]),
            BoundaryMode::Horizontal { full_3d_tangent: true } => normalize_or(d, [0.0, 1.0, 0.0]),
            BoundaryMode::Vertical => normalize_or(d, Z),
        });
    }

    let mut warnings = Vec::new();
    let mut rings = Vec::with_capacity(n);
    for k in 0..n {
        let (p, t) = (st[k], tangents[k]);
        let (right, up) = match mode {
            BoundaryMode::Horizontal { full_3d_tangent: false } => {
                (normalize_or(mining_core::cross(Z, t), [1.0, 0.0, 0.0]), Z)
            }
            _ => aaa_frame(t),
        };
        let hit: Vec<Vec3> = lines
            .iter()
            .enumerate()
            .map(|(i, l)| {
                plane_intersect(l, p, t).unwrap_or_else(|| {
                    warnings.push(SolidWarning::NoPlaneCrossing {
                        station: k,
                        boundary: names[i],
                    });
                    nearest_vertex(l, p, t)
                })
            })
            .collect();
        let (pl, pr) = (dot(hit[2], right), dot(hit[3], right));
        let (pa, pb) = (dot(hit[0], up), dot(hit[1], up));
        let hw = (pr - pl).abs() / 2.0;
        let hh = (pa - pb).abs() / 2.0;
        if hw < 1e-9 || hh < 1e-9 {
            return Err(SolidError::DegenerateSection { station: k });
        }
        let c = add(
            add(scale(t, dot(p, t)), scale(right, (pl + pr) / 2.0)),
            scale(up, (pa + pb) / 2.0),
        );
        let corner = |su: f64, sv: f64| add(c, add(scale(right, su * hw), scale(up, sv * hh)));
        rings.push(vec![
            corner(-1.0, -1.0),
            corner(1.0, -1.0),
            corner(1.0, 1.0),
            corner(-1.0, 1.0),
        ]);
    }
    Ok(BoundaryResult { rings, warnings })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::ClosedMesh;

    fn line(x: f64, z: f64, reversed: bool) -> Vec<Vec3> {
        let mut v = vec![[x, 0.0, z], [x, 40.0, z]];
        if reversed {
            v.reverse();
        }
        v
    }

    #[test]
    fn box_from_horizontal_boundaries() {
        // 4 m wide (x = -2..2), 3 m high (z = 0..3), 40 m long; one line drawn backwards.
        let (roof, floor, left, right) = (
            line(0.0, 3.0, false),
            line(0.0, 0.0, true),
            line(-2.0, 1.5, false),
            line(2.0, 1.5, false),
        );
        let r = boundary_rings(
            &Boundaries {
                a: &roof,
                b: &floor,
                left: &left,
                right: &right,
                flips: [false; 4],
            },
            5.0,
            BoundaryMode::Horizontal { full_3d_tangent: false },
        )
        .unwrap();
        assert!(r.warnings.is_empty());
        let v = ClosedMesh::from_rings(&r.rings).unwrap().volume();
        assert!((v - 4.0 * 3.0 * 40.0).abs() < 1e-6, "{v}");
    }

    #[test]
    fn raise_from_vertical_boundaries() {
        let col = |x: f64, y: f64| vec![[x, y, 0.0], [x, y, 20.0]];
        let (front, back, left, right) = (col(0.0, 1.0), col(0.0, -1.0), col(-1.5, 0.0), col(1.5, 0.0));
        let r = boundary_rings(
            &Boundaries {
                a: &front,
                b: &back,
                left: &left,
                right: &right,
                flips: [false; 4],
            },
            2.0,
            BoundaryMode::Vertical,
        )
        .unwrap();
        let v = ClosedMesh::from_rings(&r.rings).unwrap().volume();
        assert!((v - 3.0 * 2.0 * 20.0).abs() < 1e-6, "{v}");
    }

    #[test]
    fn missing_crossing_is_reported() {
        let roof = line(0.0, 3.0, false);
        let floor = line(0.0, 0.0, false);
        let left = line(-2.0, 1.5, false);
        let short_right = vec![[2.0, 0.0, 1.5], [2.0, 10.0, 1.5]];
        let r = boundary_rings(
            &Boundaries {
                a: &roof,
                b: &floor,
                left: &left,
                right: &short_right,
                flips: [false; 4],
            },
            5.0,
            BoundaryMode::Horizontal { full_3d_tangent: false },
        )
        .unwrap();
        assert!(r
            .warnings
            .iter()
            .any(|w| matches!(w, SolidWarning::NoPlaneCrossing { boundary: "right", .. })));
    }
}
