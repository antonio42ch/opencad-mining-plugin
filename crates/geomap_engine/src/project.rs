//! Projection of a mapped trace to another height along its dip.
//!
//! The trace is taken to lie on a plane of known **dip** (0..90, positive
//! down). Moving a point from its height `z` to the height `z_t` along the
//! line of steepest descent changes its position in plan by
//!
//! ```text
//! s = (z - z_t) / tan(dip)      measured along the dip direction
//! ```
//!
//! (`s > 0` going **down**, i.e. towards the dip direction; going **up** the
//! point moves against it). A vertical plane (dip = 90) does not move in plan.
//!
//! The dip direction is either fixed (an azimuth) or derived from the trace:
//! perpendicular to it, to its right or to its left. On a polyline every
//! segment has its own strike, so each vertex is moved to the intersection of
//! the two shifted segments that meet there (a mitre join), which is exactly
//! where the two planes meet the target height.

use std::fmt;

use mining_core::{normalize360, Vec3};

/// Where the dip points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DipDirection {
    /// Perpendicular to the trace, on its right-hand side (clockwise, looking
    /// along the drawing direction).
    Right,
    /// Perpendicular to the trace, on its left-hand side.
    Left,
    /// Fixed geological azimuth (0 = north, clockwise).
    Fixed(f64),
}

/// Target height of the projection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Height {
    /// Every vertex goes to this absolute elevation.
    Absolute(f64),
    /// Every vertex moves by this amount (+ up, - down), keeping its own
    /// relative height.
    Delta(f64),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProjectError {
    /// Dip must be in (0, 90]: a horizontal plane never reaches another height.
    InvalidDip(f64),
    InvalidHeight,
    /// Fewer than 2 distinct vertices.
    TooFewVertices,
    /// Right/Left needs a strike, and every segment is vertical in plan.
    NoStrike,
}

impl fmt::Display for ProjectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDip(d) => write!(f, "dip must be within (0, 90] degrees (got {d})"),
            Self::InvalidHeight => write!(f, "the target height must be a finite number"),
            Self::TooFewVertices => write!(f, "a trace needs at least 2 distinct vertices"),
            Self::NoStrike => write!(f, "the trace has no direction in plan (use a fixed dip azimuth)"),
        }
    }
}
impl std::error::Error for ProjectError {}

/// Longest allowed mitre (in units of the shift): sharp reversals would
/// otherwise send a vertex to infinity.
const MITRE_LIMIT: f64 = 4.0;

fn unit(az: f64) -> [f64; 2] {
    let a = normalize360(az).to_radians();
    [a.sin(), a.cos()]
}

/// Plan unit normal of the segment `a -> b` on the chosen side, `None` for a
/// zero-length segment.
fn normal(a: Vec3, b: Vec3, right: bool) -> Option<[f64; 2]> {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let l = dx.hypot(dy);
    if l < 1e-12 {
        return None;
    }
    let (ux, uy) = (dx / l, dy / l);
    Some(if right { [uy, -ux] } else { [-uy, ux] })
}

/// Projects `points` (a polyline, or a closed ring when `closed`).
pub fn project_polyline(
    points: &[Vec3],
    closed: bool,
    dip: f64,
    dir: DipDirection,
    height: Height,
) -> Result<Vec<Vec3>, ProjectError> {
    if !(dip.is_finite() && dip > 0.0 && dip <= 90.0) {
        return Err(ProjectError::InvalidDip(dip));
    }
    let h = match height {
        Height::Absolute(z) | Height::Delta(z) => z,
    };
    if !h.is_finite() || points.iter().any(|p| p.iter().any(|c| !c.is_finite())) {
        return Err(ProjectError::InvalidHeight);
    }
    let mut pts: Vec<Vec3> = points.to_vec();
    pts.dedup_by(|a, b| (a[0] - b[0]).hypot(a[1] - b[1]).hypot(a[2] - b[2]) < 1e-12);
    if closed && pts.len() > 1 && (pts[0][0] - pts[pts.len() - 1][0]).hypot(pts[0][1] - pts[pts.len() - 1][1]) < 1e-12 {
        pts.pop();
    }
    if pts.len() < 2 {
        return Err(ProjectError::TooFewVertices);
    }
    let n = pts.len();
    let ring = closed && n > 2;
    // Dip 90: the plane is vertical, the plan position does not change.
    let cot = if dip >= 90.0 { 0.0 } else { 1.0 / dip.to_radians().tan() };
    let shift_of = |z: f64| -> f64 {
        let target = match height {
            Height::Absolute(zt) => zt,
            Height::Delta(d) => z + d,
        };
        (z - target) * cot
    };
    let z_out = |z: f64| match height {
        Height::Absolute(zt) => zt,
        Height::Delta(d) => z + d,
    };
    let seg_normal = |i: usize, right: bool| -> Option<[f64; 2]> {
        let (a, b) = (pts[i], pts[(i + 1) % n]);
        normal(a, b, right)
    };
    let mut out = Vec::with_capacity(n);
    for (i, p) in pts.iter().enumerate() {
        let z = p[2];
        let s = shift_of(z);
        let (nx, ny) = match dir {
            DipDirection::Fixed(az) => {
                let u = unit(az);
                (u[0] * s, u[1] * s)
            }
            DipDirection::Right | DipDirection::Left => {
                let right = dir == DipDirection::Right;
                // Segment before and after the vertex (open ends have one).
                let after = if i + 1 < n || ring { seg_normal(i, right) } else { None };
                let before = if i > 0 || ring {
                    seg_normal((i + n - 1) % n, right)
                } else {
                    None
                };
                let m = match (before, after) {
                    (Some(a), Some(b)) => {
                        let d = 1.0 + a[0] * b[0] + a[1] * b[1];
                        let mut m = if d < 1e-9 {
                            a
                        } else {
                            [(a[0] + b[0]) / d, (a[1] + b[1]) / d]
                        };
                        let l = m[0].hypot(m[1]);
                        if l > MITRE_LIMIT {
                            m = [m[0] / l * MITRE_LIMIT, m[1] / l * MITRE_LIMIT];
                        }
                        m
                    }
                    (Some(a), None) | (None, Some(a)) => a,
                    (None, None) => [0.0, 0.0],
                };
                (m[0] * s, m[1] * s)
            }
        };
        out.push([p[0] + nx, p[1] + ny, z_out(z)]);
    }
    if matches!(dir, DipDirection::Right | DipDirection::Left)
        && !(0..n).any(|i| seg_normal(i, true).is_some() && (i + 1 < n || ring))
    {
        return Err(ProjectError::NoStrike);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Vec3, b: Vec3) -> bool {
        (0..3).all(|i| (a[i] - b[i]).abs() < 1e-9)
    }

    /// East-west line at z = 100 (azimuth 90). To its right is the south.
    fn ew() -> Vec<Vec3> {
        vec![[0.0, 0.0, 100.0], [10.0, 0.0, 100.0]]
    }

    #[test]
    fn going_down_moves_towards_the_dip_direction_by_dz_over_tan_dip() {
        let p = project_polyline(&ew(), false, 45.0, DipDirection::Right, Height::Absolute(90.0)).unwrap();
        assert!(
            close(p[0], [0.0, -10.0, 90.0]) && close(p[1], [10.0, -10.0, 90.0]),
            "{p:?}"
        );
        let p60 = project_polyline(&ew(), false, 60.0, DipDirection::Right, Height::Absolute(90.0)).unwrap();
        assert!((p60[0][1] + 10.0 / 60f64.to_radians().tan()).abs() < 1e-9);
        // Left of an east-going line is north.
        let l = project_polyline(&ew(), false, 45.0, DipDirection::Left, Height::Absolute(90.0)).unwrap();
        assert!(close(l[0], [0.0, 10.0, 90.0]));
    }

    #[test]
    fn going_up_moves_against_the_dip_direction() {
        let p = project_polyline(&ew(), false, 45.0, DipDirection::Right, Height::Absolute(110.0)).unwrap();
        assert!(close(p[0], [0.0, 10.0, 110.0]), "{p:?}");
    }

    #[test]
    fn delta_mode_equals_absolute_and_keeps_relative_heights() {
        let a = project_polyline(&ew(), false, 30.0, DipDirection::Right, Height::Delta(-8.0)).unwrap();
        let b = project_polyline(&ew(), false, 30.0, DipDirection::Right, Height::Absolute(92.0)).unwrap();
        assert!((0..2).all(|i| close(a[i], b[i])));
        // A line that rises 5 m keeps its 5 m under a delta, and is flattened by an absolute height.
        let rising = vec![[0.0, 0.0, 100.0], [10.0, 0.0, 105.0]];
        let d = project_polyline(&rising, false, 45.0, DipDirection::Fixed(180.0), Height::Delta(-10.0)).unwrap();
        assert_eq!((d[0][2], d[1][2]), (90.0, 95.0));
        assert!(
            (d[0][1] + 10.0).abs() < 1e-9 && (d[1][1] + 10.0).abs() < 1e-9,
            "same shift: pure translation"
        );
        let f = project_polyline(&rising, false, 45.0, DipDirection::Fixed(180.0), Height::Absolute(90.0)).unwrap();
        assert_eq!((f[0][2], f[1][2]), (90.0, 90.0));
        // Each vertex moves by its own height difference: the higher one goes further.
        assert!((f[0][1] + 10.0).abs() < 1e-9 && (f[1][1] + 15.0).abs() < 1e-9);
    }

    #[test]
    fn vertical_plane_does_not_move_in_plan_and_fixed_azimuth_is_followed() {
        let v = project_polyline(&ew(), false, 90.0, DipDirection::Right, Height::Absolute(50.0)).unwrap();
        assert!(close(v[0], [0.0, 0.0, 50.0]) && close(v[1], [10.0, 0.0, 50.0]));
        // Fixed azimuth 90 (east), dip 45, down 10 -> 10 m east.
        let f = project_polyline(&ew(), false, 45.0, DipDirection::Fixed(90.0), Height::Absolute(90.0)).unwrap();
        assert!(close(f[0], [10.0, 0.0, 90.0]));
        // Azimuth 0 (north), up 10 -> 10 m south.
        let g = project_polyline(&ew(), false, 45.0, DipDirection::Fixed(0.0), Height::Absolute(110.0)).unwrap();
        assert!(close(g[0], [0.0, -10.0, 110.0]));
    }

    #[test]
    fn polyline_corners_are_the_intersection_of_the_shifted_segments() {
        // East then north; right side = south of the first leg, east of the second.
        let l = vec![[0.0, 0.0, 100.0], [10.0, 0.0, 100.0], [10.0, 10.0, 100.0]];
        let p = project_polyline(&l, false, 45.0, DipDirection::Right, Height::Absolute(95.0)).unwrap();
        // Shift 5 m: legs become y = -5 and x = 15, meeting at (15, -5).
        assert!(
            close(p[0], [0.0, -5.0, 95.0]) && close(p[1], [15.0, -5.0, 95.0]) && close(p[2], [15.0, 10.0, 95.0]),
            "{p:?}"
        );
    }

    #[test]
    fn closed_rings_shift_every_vertex_with_mitres() {
        // A 10 x 10 square drawn clockwise: its right side is the inside.
        let sq = vec![
            [0.0, 0.0, 100.0],
            [0.0, 10.0, 100.0],
            [10.0, 10.0, 100.0],
            [10.0, 0.0, 100.0],
        ];
        let p = project_polyline(&sq, true, 45.0, DipDirection::Right, Height::Absolute(98.0)).unwrap();
        assert!(close(p[0], [2.0, 2.0, 98.0]) && close(p[2], [8.0, 8.0, 98.0]), "{p:?}");
        // A repeated closing vertex is accepted.
        let mut closed = sq.clone();
        closed.push(sq[0]);
        assert_eq!(
            project_polyline(&closed, true, 45.0, DipDirection::Right, Height::Absolute(98.0))
                .unwrap()
                .len(),
            4
        );
    }

    #[test]
    fn errors_and_degenerate_geometry() {
        let ok = |dip| project_polyline(&ew(), false, dip, DipDirection::Right, Height::Absolute(90.0));
        assert!(matches!(ok(0.0), Err(ProjectError::InvalidDip(_))));
        assert!(matches!(ok(91.0), Err(ProjectError::InvalidDip(_))));
        assert!(matches!(ok(f64::NAN), Err(ProjectError::InvalidDip(_))));
        assert_eq!(
            project_polyline(&ew(), false, 45.0, DipDirection::Right, Height::Absolute(f64::NAN)),
            Err(ProjectError::InvalidHeight)
        );
        assert_eq!(
            project_polyline(&ew()[..1], false, 45.0, DipDirection::Right, Height::Absolute(1.0)),
            Err(ProjectError::TooFewVertices)
        );
        // A vertical line has no strike: Right/Left cannot work, a fixed azimuth can.
        let vertical = vec![[0.0, 0.0, 100.0], [0.0, 0.0, 90.0]];
        assert_eq!(
            project_polyline(&vertical, false, 45.0, DipDirection::Right, Height::Absolute(80.0)),
            Err(ProjectError::NoStrike)
        );
        assert!(project_polyline(
            &vertical,
            false,
            45.0,
            DipDirection::Fixed(90.0),
            Height::Absolute(80.0)
        )
        .is_ok());
        // Duplicated vertices collapse to one: too few.
        let dup = vec![[1.0, 1.0, 5.0], [1.0, 1.0, 5.0]];
        assert_eq!(
            project_polyline(&dup, false, 45.0, DipDirection::Right, Height::Absolute(80.0)),
            Err(ProjectError::TooFewVertices)
        );
        let flat_in_plan = vec![[0.0, 0.0, 100.0], [0.0, 0.0, 90.0], [0.0, 0.0, 80.0]];
        assert_eq!(
            project_polyline(&flat_in_plan, false, 45.0, DipDirection::Left, Height::Absolute(70.0)),
            Err(ProjectError::NoStrike)
        );
        assert!(project_polyline(
            &flat_in_plan,
            false,
            45.0,
            DipDirection::Fixed(45.0),
            Height::Absolute(70.0)
        )
        .is_ok());
        // The same height as the trace is a valid (identity) projection.
        let same = project_polyline(&ew(), false, 45.0, DipDirection::Right, Height::Absolute(100.0)).unwrap();
        assert!(close(same[0], [0.0, 0.0, 100.0]));
    }

    #[test]
    fn a_sharp_reversal_does_not_send_the_vertex_to_infinity() {
        let hairpin = vec![[0.0, 0.0, 100.0], [10.0, 0.0, 100.0], [0.0, 0.001, 100.0]];
        let p = project_polyline(&hairpin, false, 45.0, DipDirection::Right, Height::Absolute(90.0)).unwrap();
        assert!(
            p.iter().all(|q| q.iter().all(|c| c.is_finite() && c.abs() < 100.0)),
            "{p:?}"
        );
    }
}
