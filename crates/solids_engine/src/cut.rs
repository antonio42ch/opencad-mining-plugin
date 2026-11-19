//! Cutting meshes with planes: cross-sections of excavation solids.
//!
//! A [`Plane`] carries its own 2D frame (`u`, `v`) so every section can be
//! drawn flat (section sheets) with meaningful coordinates:
//!
//! | Planes | u | v |
//! |---|---|---|
//! | perpendicular to an axis | offset from the axis, + to the right of the advance | elevation Z |
//! | vertical through 2 points | distance from the first point along the line | elevation Z |
//! | horizontal (levels) | X (east) | Y (north) |
//! | through 3 points | along p1→p2 | in-plane, perpendicular |

use std::collections::HashMap;

use mining_core::{add, cross, dot, len, lerp, normalize_or, scale, sub, Vec2, Vec3, Z};

/// A planar polygon mesh (faces are vertex-index lists, any winding).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MeshData {
    pub vertices: Vec<Vec3>,
    pub faces: Vec<Vec<usize>>,
}

impl MeshData {
    pub fn bounds(&self) -> Option<(Vec3, Vec3)> {
        let first = *self.vertices.first()?;
        Some(self.vertices.iter().fold((first, first), |(lo, hi), p| {
            (
                [lo[0].min(p[0]), lo[1].min(p[1]), lo[2].min(p[2])],
                [hi[0].max(p[0]), hi[1].max(p[1]), hi[2].max(p[2])],
            )
        }))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plane {
    pub origin: Vec3,
    /// Unit normal.
    pub normal: Vec3,
    /// Unit in-plane axes; `u × v = normal`.
    pub u: Vec3,
    pub v: Vec3,
}

impl Plane {
    pub fn to_2d(&self, p: Vec3) -> Vec2 {
        let d = sub(p, self.origin);
        [dot(d, self.u), dot(d, self.v)]
    }
    pub fn to_3d(&self, q: Vec2) -> Vec3 {
        add(self.origin, add(scale(self.u, q[0]), scale(self.v, q[1])))
    }
    pub fn distance(&self, p: Vec3) -> f64 {
        dot(sub(p, self.origin), self.normal)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PlaneError {
    /// The 2 or 3 points do not define a plane.
    Degenerate,
    InvalidInterval(f64),
    InvalidCount(usize),
    /// Too many planes for one run.
    TooMany(usize),
    /// The axis has no horizontal length.
    AxisTooShort,
}

impl std::fmt::Display for PlaneError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Degenerate => write!(f, "the points do not define a plane"),
            Self::InvalidInterval(v) => write!(f, "interval must be > 0 (got {v})"),
            Self::InvalidCount(n) => write!(f, "count must be >= 1 (got {n})"),
            Self::TooMany(n) => write!(f, "{n} planes requested; the limit is {MAX_PLANES}"),
            Self::AxisTooShort => write!(f, "the axis has no horizontal length"),
        }
    }
}
impl std::error::Error for PlaneError {}

pub const MAX_PLANES: usize = 2_000;

/// A plane with a label (chainage, level, index) for drawing.
#[derive(Debug, Clone, PartialEq)]
pub struct LabeledPlane {
    pub plane: Plane,
    pub label: String,
    /// Chainage / elevation / offset the label was built from.
    pub value: f64,
    /// Point to mark on the section (the axis for perpendicular planes).
    pub marker: Option<Vec3>,
}

fn check_interval(i: f64) -> Result<f64, PlaneError> {
    if i.is_finite() && i > 0.0 {
        Ok(i)
    } else {
        Err(PlaneError::InvalidInterval(i))
    }
}

fn count_steps(from: f64, to: f64, step: f64) -> Result<usize, PlaneError> {
    let n = ((to - from) / step + 1e-9).floor() as i64 + 1;
    let n = n.max(0) as usize;
    if n > MAX_PLANES {
        Err(PlaneError::TooMany(n))
    } else {
        Ok(n)
    }
}

/// Formats a chainage as `0+125.00`.
pub fn chainage_label(ch: f64) -> String {
    let sign = if ch < 0.0 { "-" } else { "" };
    let a = ch.abs();
    let km = (a / 1000.0).floor();
    format!("{sign}{}+{:06.2}", km as i64, a - km * 1000.0)
}

/// Vertical planes perpendicular (in plan) to an axis polyline, every
/// `interval` metres of **horizontal** chainage within `[from, to]`
/// (`to <= from` means "to the end").
pub fn planes_along_axis(axis: &[Vec3], interval: f64, from: f64, to: f64) -> Result<Vec<LabeledPlane>, PlaneError> {
    let step = check_interval(interval)?;
    // Cumulative horizontal chainage per vertex.
    let mut ch = vec![0.0];
    for w in axis.windows(2) {
        ch.push(ch.last().unwrap() + (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1]));
    }
    let total = *ch.last().unwrap_or(&0.0);
    if axis.len() < 2 || total <= 1e-9 {
        return Err(PlaneError::AxisTooShort);
    }
    let from = from.max(0.0);
    let to = if to > from { to.min(total) } else { total };
    let n = count_steps(from, to, step)?;
    let mut out = Vec::with_capacity(n);
    for k in 0..n {
        let s = from + k as f64 * step;
        let i = ch.partition_point(|&c| c <= s).clamp(1, axis.len() - 1) - 1;
        // Skip zero-length (vertical) segments to find a usable direction.
        let mut j = i;
        while j + 1 < axis.len() && ch[j + 1] - ch[j] <= 1e-12 {
            j += 1;
        }
        if j + 1 >= axis.len() {
            break;
        }
        let (a, b) = (axis[j], axis[j + 1]);
        let seg = ch[j + 1] - ch[j];
        let t = ((s - ch[j]) / seg).clamp(0.0, 1.0);
        let p = lerp(a, b, t);
        let tan = normalize_or([b[0] - a[0], b[1] - a[1], 0.0], [0.0, 1.0, 0.0]);
        // Right of the advance direction, horizontal.
        let right = [tan[1], -tan[0], 0.0];
        // Origin at z = 0 so `v` is the true elevation.
        out.push(LabeledPlane {
            plane: Plane {
                origin: [p[0], p[1], 0.0],
                normal: tan,
                u: right,
                v: Z,
            },
            label: chainage_label(s),
            value: s,
            marker: Some(p),
        });
    }
    Ok(out)
}

/// Vertical plane through `p1`, `p2` (plan direction p1→p2) and `count`
/// parallels every `spacing` metres to the right of p1→p2.
pub fn vertical_planes(p1: Vec3, p2: Vec3, spacing: f64, count: usize) -> Result<Vec<LabeledPlane>, PlaneError> {
    if count == 0 {
        return Err(PlaneError::InvalidCount(count));
    }
    if count > MAX_PLANES {
        return Err(PlaneError::TooMany(count));
    }
    let step = if count > 1 { check_interval(spacing)? } else { 0.0 };
    let d = [p2[0] - p1[0], p2[1] - p1[1], 0.0];
    if len(d) <= 1e-9 {
        return Err(PlaneError::Degenerate);
    }
    let u = normalize_or(d, [1.0, 0.0, 0.0]);
    // normal = u × v with v = Z  ->  points to the right of p1→p2.
    let normal = cross(u, Z);
    Ok((0..count)
        .map(|k| {
            let off = k as f64 * step;
            LabeledPlane {
                plane: Plane {
                    origin: add([p1[0], p1[1], 0.0], scale(normal, off)),
                    normal,
                    u,
                    v: Z,
                },
                label: format!("{off:.2}"),
                value: off,
                marker: None,
            }
        })
        .collect())
}

/// Horizontal planes at `z_from, z_from+step, … ≤ z_to` (2D frame = plan X/Y).
pub fn level_planes(z_from: f64, z_to: f64, step: f64) -> Result<Vec<LabeledPlane>, PlaneError> {
    let step = check_interval(step)?;
    let (lo, hi) = if z_to >= z_from { (z_from, z_to) } else { (z_to, z_from) };
    let n = count_steps(lo, hi, step)?;
    Ok((0..n)
        .map(|k| {
            let z = lo + k as f64 * step;
            LabeledPlane {
                plane: Plane {
                    origin: [0.0, 0.0, z],
                    normal: Z,
                    u: [1.0, 0.0, 0.0],
                    v: [0.0, 1.0, 0.0],
                },
                label: format!("{z:.2}"),
                value: z,
                marker: None,
            }
        })
        .collect())
}

/// Plane through three points; u along p1→p2.
pub fn plane_3_points(p1: Vec3, p2: Vec3, p3: Vec3) -> Result<LabeledPlane, PlaneError> {
    let n = cross(sub(p2, p1), sub(p3, p1));
    if len(n) <= 1e-9 * len(sub(p2, p1)).max(1.0) * len(sub(p3, p1)).max(1.0) {
        return Err(PlaneError::Degenerate);
    }
    let normal = normalize_or(n, Z);
    let u = normalize_or(sub(p2, p1), [1.0, 0.0, 0.0]);
    let v = cross(normal, u);
    Ok(LabeledPlane {
        plane: Plane {
            origin: p1,
            normal,
            u,
            v,
        },
        label: String::new(),
        value: 0.0,
        marker: None,
    })
}

/// One polyline of a section.
#[derive(Debug, Clone, PartialEq)]
pub struct SectionLoop {
    pub points: Vec<Vec3>,
    pub points_2d: Vec<Vec2>,
    pub closed: bool,
}

impl SectionLoop {
    /// Absolute area in the plane (0 for open polylines).
    pub fn area(&self) -> f64 {
        if !self.closed || self.points_2d.len() < 3 {
            return 0.0;
        }
        let p = &self.points_2d;
        let s: f64 = (0..p.len())
            .map(|i| {
                let (a, b) = (p[i], p[(i + 1) % p.len()]);
                a[0] * b[1] - b[0] * a[1]
            })
            .sum();
        (s / 2.0).abs()
    }
}

/// Signed distance with symbolic perturbation: vertices exactly on the plane
/// count as being on the positive side, so no crossing is produced twice.
fn side(d: f64) -> bool {
    d >= 0.0
}

/// Intersection segments of the plane with every face.
fn segments(mesh: &MeshData, plane: &Plane) -> Vec<(Vec3, Vec3)> {
    let d: Vec<f64> = mesh.vertices.iter().map(|p| plane.distance(*p)).collect();
    let mut out = Vec::new();
    for f in &mesh.faces {
        if f.len() < 3 || f.iter().any(|&i| i >= mesh.vertices.len()) {
            continue;
        }
        let mut hits: Vec<Vec3> = Vec::new();
        for k in 0..f.len() {
            let (i, j) = (f[k], f[(k + 1) % f.len()]);
            if side(d[i]) != side(d[j]) {
                let t = d[i] / (d[i] - d[j]);
                hits.push(lerp(mesh.vertices[i], mesh.vertices[j], t));
            }
        }
        if hits.len() < 2 {
            continue;
        }
        if hits.len() == 2 {
            out.push((hits[0], hits[1]));
            continue;
        }
        // Concave face: order the crossings along the cut line and pair them.
        let fnorm = newell(mesh, f);
        let line = normalize_or(cross(plane.normal, fnorm), plane.u);
        hits.sort_by(|a, b| dot(*a, line).total_cmp(&dot(*b, line)));
        for [a, b] in hits.as_chunks::<2>().0 {
            out.push((*a, *b));
        }
    }
    out
}

fn newell(mesh: &MeshData, f: &[usize]) -> Vec3 {
    let mut n = [0.0; 3];
    for k in 0..f.len() {
        let (a, b) = (mesh.vertices[f[k]], mesh.vertices[f[(k + 1) % f.len()]]);
        n[0] += (a[1] - b[1]) * (a[2] + b[2]);
        n[1] += (a[2] - b[2]) * (a[0] + b[0]);
        n[2] += (a[0] - b[0]) * (a[1] + b[1]);
    }
    normalize_or(n, Z)
}

/// Joins segments into polylines (closed loops when they come back).
fn chain(segs: Vec<(Vec3, Vec3)>, tol: f64) -> Vec<Vec<Vec3>> {
    let key = |p: Vec3| {
        (
            (p[0] / tol).round() as i64,
            (p[1] / tol).round() as i64,
            (p[2] / tol).round() as i64,
        )
    };
    let mut adj: HashMap<(i64, i64, i64), Vec<usize>> = HashMap::new();
    let segs: Vec<(Vec3, Vec3)> = segs.into_iter().filter(|(a, b)| key(*a) != key(*b)).collect();
    for (i, (a, b)) in segs.iter().enumerate() {
        adj.entry(key(*a)).or_default().push(i);
        adj.entry(key(*b)).or_default().push(i);
    }
    let mut used = vec![false; segs.len()];
    let mut out = Vec::new();
    // Start open chains at dangling ends first, then the remaining loops.
    let mut starts: Vec<usize> = (0..segs.len()).collect();
    starts.sort_by_key(|&i| {
        let deg = |p: Vec3| adj.get(&key(p)).map_or(0, |v| v.len());
        std::cmp::min(deg(segs[i].0), deg(segs[i].1))
    });
    for s in starts {
        if used[s] {
            continue;
        }
        used[s] = true;
        // Walk forward from the segment, then backwards from its first point,
        // so open chains started mid-way are complete.
        let (first, mut cur) = segs[s];
        let mut pts = vec![first, cur];
        loop {
            let next = adj.get(&key(cur)).and_then(|v| v.iter().copied().find(|&i| !used[i]));
            let Some(i) = next else { break };
            used[i] = true;
            let (p, q) = segs[i];
            cur = if key(p) == key(cur) { q } else { p };
            pts.push(cur);
        }
        // Extend backwards from the first point for open chains started mid-way.
        let mut back = vec![];
        let mut cur = first;
        loop {
            let next = adj.get(&key(cur)).and_then(|v| v.iter().copied().find(|&i| !used[i]));
            let Some(i) = next else { break };
            used[i] = true;
            let (p, q) = segs[i];
            cur = if key(p) == key(cur) { q } else { p };
            back.push(cur);
        }
        if !back.is_empty() {
            back.reverse();
            back.extend(pts);
            pts = back;
        }
        out.push(pts);
    }
    out
}

/// Cuts every mesh with the plane and returns the section polylines.
pub fn cut(meshes: &[MeshData], plane: &Plane) -> Vec<SectionLoop> {
    let scale_hint = meshes
        .iter()
        .filter_map(|m| m.bounds())
        .map(|(lo, hi)| len(sub(hi, lo)))
        .fold(1.0f64, f64::max);
    let tol = (scale_hint * 1e-9).max(1e-9);
    let eps = scale_hint * 1e-7;
    let mut out = Vec::new();
    for m in meshes {
        let mut loops = cut_one(m, plane, plane, tol);
        // A plane lying exactly on a face (e.g. the end face of a gallery at
        // chainage 0) touches the mesh without crossing it: cut a hair inside
        // and project the result back onto the requested plane.
        if loops.is_empty() {
            for sign in [1.0, -1.0] {
                let shifted = Plane {
                    origin: add(plane.origin, scale(plane.normal, sign * eps)),
                    ..*plane
                };
                loops = cut_one(m, &shifted, plane, tol);
                if !loops.is_empty() {
                    break;
                }
            }
        }
        out.extend(loops);
    }
    out
}

/// Cuts one mesh with `cutter` and expresses the result on `target`.
fn cut_one(m: &MeshData, cutter: &Plane, target: &Plane, tol: f64) -> Vec<SectionLoop> {
    let mut out = Vec::new();
    for mut pts in chain(segments(m, cutter), tol) {
        let closed = pts.len() > 3 && len(sub(pts[0], *pts.last().unwrap())) <= tol * 10.0;
        if closed {
            pts.pop();
        }
        pts.dedup_by(|a, b| len(sub(*a, *b)) <= tol * 10.0);
        if pts.len() < 2 {
            continue;
        }
        let pts: Vec<Vec3> = pts
            .into_iter()
            .map(|p| sub(p, scale(target.normal, target.distance(p))))
            .collect();
        let points_2d = pts.iter().map(|p| target.to_2d(*p)).collect();
        out.push(SectionLoop {
            points: pts,
            points_2d,
            closed,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axis::{axis_rings, AxisParams};
    use crate::mesh::ClosedMesh;
    use crate::section::Profile;

    fn gallery(axis: &[Vec3], prof: Profile) -> MeshData {
        let rings = axis_rings(&AxisParams {
            axis,
            profile: &prof,
            interval: 2.0,
            offset: [0.0, 0.0],
        })
        .unwrap();
        let m = ClosedMesh::from_rings(&rings).unwrap();
        MeshData {
            vertices: m.vertices,
            faces: m.faces,
        }
    }

    #[test]
    fn perpendicular_cut_of_box_gallery_is_its_section() {
        let axis = [[0.0, 0.0, 0.0], [0.0, 40.0, 0.0]];
        let mesh = gallery(
            &axis,
            Profile::Rectangular {
                width: 4.0,
                height: 3.0,
            },
        );
        let planes = planes_along_axis(&axis, 10.0, 5.0, 35.0).unwrap();
        assert_eq!(
            planes.iter().map(|p| p.value).collect::<Vec<_>>(),
            vec![5.0, 15.0, 25.0, 35.0]
        );
        assert_eq!(planes[0].label, "0+005.00");
        for p in &planes {
            let loops = cut(std::slice::from_ref(&mesh), &p.plane);
            assert_eq!(loops.len(), 1);
            assert!(loops[0].closed);
            assert!((loops[0].area() - 12.0).abs() < 1e-9, "{}", loops[0].area());
            // u spans the width around the axis, v the height around z = 0.
            let us: Vec<f64> = loops[0].points_2d.iter().map(|q| q[0]).collect();
            assert!((us.iter().cloned().fold(f64::MIN, f64::max) - 2.0).abs() < 1e-9);
            // v is the absolute elevation: the 3 m section is centred on z = 0.
            let vs: Vec<f64> = loops[0].points_2d.iter().map(|q| q[1]).collect();
            assert!((vs.iter().cloned().fold(f64::MIN, f64::max) - 1.5).abs() < 1e-9);
            assert_eq!(p.plane.to_2d(p.marker.unwrap()), [0.0, 0.0]);
        }
    }

    #[test]
    fn level_cut_of_horseshoe_and_planes_outside_give_nothing() {
        let axis = [[0.0, 0.0, 0.0], [30.0, 0.0, 0.0]];
        let mesh = gallery(
            &axis,
            Profile::Horseshoe {
                width: 4.0,
                wall_height: 3.0,
                arc_radius: None,
                arc_points: 17,
            },
        );
        let levels = level_planes(0.0, 10.0, 5.0).unwrap();
        assert_eq!(levels.len(), 3);
        let at0 = cut(std::slice::from_ref(&mesh), &levels[0].plane);
        assert_eq!(at0.len(), 1);
        // Horizontal cut at mid height: 30 m long x 4 m wide.
        assert!((at0[0].area() - 120.0).abs() < 1e-6, "{}", at0[0].area());
        assert!(cut(&[mesh], &levels[2].plane).is_empty());
    }

    #[test]
    fn vertical_series_and_three_points() {
        let axis = [[0.0, 0.0, 0.0], [0.0, 40.0, 0.0]];
        let mesh = gallery(
            &axis,
            Profile::Circular {
                radius: 2.0,
                segments: 32,
            },
        );
        // Longitudinal vertical planes along the gallery, 1 m apart to the right.
        let planes = vertical_planes([0.0, -5.0, 0.0], [0.0, 50.0, 0.0], 1.0, 3).unwrap();
        assert_eq!(planes.len(), 3);
        let c0 = cut(std::slice::from_ref(&mesh), &planes[0].plane);
        assert_eq!(c0.len(), 1);
        assert!(c0[0].closed);
        // Plane through the axis: 40 m long, 4 m (chord of the 32-gon ≈ diameter) high.
        assert!((c0[0].area() - 40.0 * 4.0).abs() < 0.5, "{}", c0[0].area());
        let p3 = plane_3_points([0.0, 10.0, 0.0], [1.0, 10.0, 0.0], [0.0, 10.0, 1.0]).unwrap();
        let c3 = cut(&[mesh], &p3.plane);
        assert!((c3[0].area() - std::f64::consts::PI * 4.0).abs() < 0.3);
        assert_eq!(
            plane_3_points([0.0; 3], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]),
            Err(PlaneError::Degenerate)
        );
    }

    #[test]
    fn plane_on_an_end_face_still_gives_the_profile() {
        let axis = [[0.0, 0.0, 0.0], [0.0, 20.0, 0.0]];
        let mesh = gallery(
            &axis,
            Profile::Rectangular {
                width: 4.0,
                height: 3.0,
            },
        );
        for p in planes_along_axis(&axis, 20.0, 0.0, 0.0).unwrap() {
            let loops = cut(std::slice::from_ref(&mesh), &p.plane);
            assert_eq!(loops.len(), 1, "at {}", p.label);
            assert!((loops[0].area() - 12.0).abs() < 1e-6);
            assert!(loops[0].points.iter().all(|q| p.plane.distance(*q).abs() < 1e-9));
        }
    }

    #[test]
    fn concave_face_is_cut_into_two_segments() {
        // A U-shaped planar face in the XZ plane, cut by z = 1 -> two segments.
        let mesh = MeshData {
            vertices: vec![
                [0.0, 0.0, 0.0],
                [3.0, 0.0, 0.0],
                [3.0, 0.0, 2.0],
                [2.0, 0.0, 2.0],
                [2.0, 0.0, 0.5],
                [1.0, 0.0, 0.5],
                [1.0, 0.0, 2.0],
                [0.0, 0.0, 2.0],
            ],
            faces: vec![(0..8).collect()],
        };
        let plane = level_planes(1.0, 1.0, 1.0).unwrap()[0].plane;
        let loops = cut(&[mesh], &plane);
        assert_eq!(loops.len(), 2);
        assert!(loops.iter().all(|l| !l.closed && l.points.len() == 2));
    }

    #[test]
    fn limits_and_labels() {
        assert_eq!(chainage_label(1234.5), "1+234.50");
        assert!(matches!(level_planes(0.0, 1e6, 1.0), Err(PlaneError::TooMany(_))));
        assert!(matches!(
            planes_along_axis(&[[0.0; 3], [0.0, 0.0, 10.0]], 1.0, 0.0, 0.0),
            Err(PlaneError::AxisTooShort)
        ));
        assert!(matches!(
            vertical_planes([0.0; 3], [0.0; 3], 1.0, 1),
            Err(PlaneError::Degenerate)
        ));
    }
}
