//! Wedges formed by three discontinuity planes in the roof and walls of an
//! underground opening (Hoek & Brown 1980, ch. 7), with planar excavation
//! faces, and their stability by the vector method of Hoek & Bray (1981):
//! falling, sliding on one plane or on two planes, optional support pressure.
//!
//! Units: metres, kN, kPa; cohesion in kPa. The largest wedge is the one
//! whose trace on the face just fits the span across the opening (and, if
//! given, a maximum length along it).

use mining_core::{add, cross, dot, scale, sub, Vec3};

use crate::stereonet::{Line, Plane};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaceKind {
    Roof,
    LeftWall,
    RightWall,
}

impl FaceKind {
    pub const ALL: [FaceKind; 3] = [FaceKind::Roof, FaceKind::LeftWall, FaceKind::RightWall];
    pub fn key(self) -> &'static str {
        match self {
            FaceKind::Roof => "roof",
            FaceKind::LeftWall => "left_wall",
            FaceKind::RightWall => "right_wall",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Joint {
    pub plane: Plane,
    /// Friction angle (degrees).
    pub friction: f64,
    /// Cohesion (kPa).
    pub cohesion: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Opening {
    /// Axis direction (trend, plunge); the sign does not matter.
    pub axis: Line,
    pub width: f64,
    pub height: f64,
    /// Maximum wedge length along the axis (0 = unlimited).
    pub max_length: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    /// The resultant points into the rock or no motion is possible.
    Stable,
    Falling,
    /// Sliding on one plane (index 0..3).
    Sliding1(usize),
    /// Sliding on two planes along their intersection.
    Sliding2(usize, usize),
}

impl Mode {
    pub fn key(&self) -> String {
        match self {
            Mode::Stable => "stable".into(),
            Mode::Falling => "falling".into(),
            Mode::Sliding1(i) => format!("sliding_on_{}", i + 1),
            Mode::Sliding2(i, j) => format!("sliding_on_{}_{}", i + 1, j + 1),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Wedge {
    pub face: FaceKind,
    /// Apex first, then the three vertices on the face (local frame: origin on
    /// the face plane; see [`frame`]).
    pub vertices: [Vec3; 4],
    /// Height of the apex above the face.
    pub apex_height: f64,
    pub volume: f64,
    pub weight: f64,
    pub base_area: f64,
    pub face_areas: [f64; 3],
    pub mode: Mode,
    /// Factor of safety (`None` = stable, no driving force).
    pub fs: Option<f64>,
}

/// World directions of the opening: `(axis, normal into the rock of the face,
/// direction across the face)`.
pub fn frame(o: &Opening, face: FaceKind) -> (Vec3, Vec3, Vec3) {
    let a = o.axis.vector();
    let up = [0.0, 0.0, 1.0];
    let roof = norm(sub(up, scale(a, dot(up, a))));
    let right = norm(cross(a, roof));
    match face {
        FaceKind::Roof => (a, roof, right),
        FaceKind::RightWall => (a, right, roof),
        FaceKind::LeftWall => (a, scale(right, -1.0), roof),
    }
}

fn norm(v: Vec3) -> Vec3 {
    let l = dot(v, v).sqrt();
    if l < 1e-15 {
        v
    } else {
        scale(v, 1.0 / l)
    }
}

fn det(a: Vec3, b: Vec3, c: Vec3) -> f64 {
    dot(a, cross(b, c))
}

/// The largest wedge of three joints on one face.
pub fn wedge(o: &Opening, face: FaceKind, joints: &[Joint; 3], gamma: f64, support_kpa: f64) -> Option<Wedge> {
    let (a, m, e) = frame(o, face);
    let n: Vec<Vec3> = joints.iter().map(|j| j.plane.normal()).collect();
    let apex = m;
    // Vertex on the face along the intersection line of each pair of planes.
    let pairs = [(0, 1), (0, 2), (1, 2)];
    let mut v = [[0.0; 3]; 3];
    for (k, (i, j)) in pairs.iter().enumerate() {
        let mut d = cross(n[*i], n[*j]);
        let md = dot(m, d);
        if dot(d, d) < 1e-12 || md.abs() < 1e-9 {
            return None;
        }
        if md > 0.0 {
            d = scale(d, -1.0);
        }
        v[k] = add(apex, scale(d, -1.0 / dot(m, d)));
    }
    let extent = |dir: Vec3| {
        let p: Vec<f64> = v.iter().map(|q| dot(*q, dir)).collect();
        p.iter().cloned().fold(f64::MIN, f64::max) - p.iter().cloned().fold(f64::MAX, f64::min)
    };
    let span = if face == FaceKind::Roof { o.width } else { o.height };
    let (w, l) = (extent(e), extent(a));
    if w < 1e-9 {
        return None;
    }
    let mut s = span / w;
    if o.max_length > 0.0 && l > 1e-9 {
        s = s.min(o.max_length / l);
    }
    // Centre the trace across the face.
    let centre = (v.iter().map(|q| dot(*q, e)).fold(f64::MIN, f64::max)
        + v.iter().map(|q| dot(*q, e)).fold(f64::MAX, f64::min))
        / 2.0;
    let shift = scale(e, -centre * s);
    let p = |q: Vec3| add(scale(q, s), shift);
    let verts = [p(apex), p(v[0]), p(v[1]), p(v[2])];
    let volume = det(
        sub(verts[1], verts[0]),
        sub(verts[2], verts[0]),
        sub(verts[3], verts[0]),
    )
    .abs()
        / 6.0;
    let tri = |x: Vec3, y: Vec3, z: Vec3| dot(cross(sub(y, x), sub(z, x)), cross(sub(y, x), sub(z, x))).sqrt() / 2.0;
    // Plane 0 holds V01, V02; plane 1 holds V01, V12; plane 2 holds V02, V12.
    let face_areas = [
        tri(verts[0], verts[1], verts[2]),
        tri(verts[0], verts[1], verts[3]),
        tri(verts[0], verts[2], verts[3]),
    ];
    let base_area = tri(verts[1], verts[2], verts[3]);
    let weight = gamma * volume;
    let c = scale(add(add(verts[0], verts[1]), add(verts[2], verts[3])), 0.25);
    // Normals pointing out of the wedge.
    let nu: Vec<Vec3> = n
        .iter()
        .map(|ni| {
            if dot(*ni, sub(verts[0], c)) >= 0.0 {
                *ni
            } else {
                scale(*ni, -1.0)
            }
        })
        .collect();
    let r = add([0.0, 0.0, -weight], scale(m, support_kpa * base_area));
    let (mode, fs) = stability(&r, &nu, m, joints, &face_areas, weight, support_kpa * base_area);
    Some(Wedge {
        face,
        vertices: verts,
        apex_height: s,
        volume,
        weight,
        base_area,
        face_areas,
        mode,
        fs,
    })
}

fn stability(
    r: &Vec3,
    nu: &[Vec3],
    m: Vec3,
    joints: &[Joint; 3],
    areas: &[f64; 3],
    weight: f64,
    support: f64,
) -> (Mode, Option<f64>) {
    let eps = 1e-9 * (1.0 + dot(*r, *r).sqrt());
    let tanf = |i: usize| joints[i].friction.to_radians().tan();
    // Falling: separates from every plane and moves out of the rock.
    if nu.iter().all(|v| dot(*r, *v) < eps) && dot(*r, m) < -eps {
        let pull = weight * dot([0.0, 0.0, 1.0], m).max(0.0);
        return (Mode::Falling, Some(if pull > 0.0 { support / pull } else { 0.0 }));
    }
    let mut best: Option<(Mode, f64)> = None;
    fn keep(best: &mut Option<(Mode, f64)>, mode: Mode, fs: f64) {
        if best.as_ref().is_none_or(|(_, f)| fs < *f) {
            *best = Some((mode, fs));
        }
    }
    for i in 0..3 {
        let ni = dot(*r, nu[i]);
        if ni <= eps {
            continue;
        }
        let s = sub(*r, scale(nu[i], ni));
        let t = dot(s, s).sqrt();
        if t < eps || dot(s, m) >= 0.0 {
            continue;
        }
        if (0..3).filter(|&j| j != i).all(|j| dot(s, nu[j]) <= eps) {
            keep(
                &mut best,
                Mode::Sliding1(i),
                (ni * tanf(i) + joints[i].cohesion * areas[i]) / t,
            );
        }
    }
    if best.is_none() {
        for (i, j) in [(0, 1), (0, 2), (1, 2)] {
            let mut d = norm(cross(nu[i], nu[j]));
            if dot(*r, d) < 0.0 {
                d = scale(d, -1.0);
            }
            let t = dot(*r, d);
            if t <= eps || dot(d, m) >= 0.0 {
                continue;
            }
            let k = 3 - i - j;
            if dot(d, nu[k]) > eps {
                continue;
            }
            // R - t d = a nu_i + b nu_j.
            let q = sub(*r, scale(d, t));
            let (g11, g12, g22) = (dot(nu[i], nu[i]), dot(nu[i], nu[j]), dot(nu[j], nu[j]));
            let (h1, h2) = (dot(q, nu[i]), dot(q, nu[j]));
            let dt = g11 * g22 - g12 * g12;
            if dt.abs() < 1e-12 {
                continue;
            }
            let a = (h1 * g22 - h2 * g12) / dt;
            let b = (h2 * g11 - h1 * g12) / dt;
            if a > 0.0 && b > 0.0 {
                keep(
                    &mut best,
                    Mode::Sliding2(i, j),
                    (a * tanf(i) + b * tanf(j) + joints[i].cohesion * areas[i] + joints[j].cohesion * areas[j]) / t,
                );
            }
        }
    }
    match best {
        Some((mode, fs)) => (mode, Some(fs)),
        None => (Mode::Stable, None),
    }
}

/// The wedges of the roof and both walls.
pub fn analyse(o: &Opening, joints: &[Joint; 3], gamma: f64, support_kpa: f64) -> Vec<Wedge> {
    FaceKind::ALL
        .iter()
        .filter_map(|f| wedge(o, *f, joints, gamma, support_kpa))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn j(dip: f64, dd: f64, phi: f64) -> Joint {
        Joint {
            plane: Plane::new(dip, dd),
            friction: phi,
            cohesion: 0.0,
        }
    }

    const NORTH: Opening = Opening {
        axis: Line {
            trend: 0.0,
            plunge: 0.0,
        },
        width: 5.0,
        height: 4.0,
        max_length: 0.0,
    };

    #[test]
    fn roof_pyramid_falls_and_support_carries_it() {
        let joints = [j(70.0, 0.0, 30.0), j(70.0, 120.0, 30.0), j(70.0, 240.0, 30.0)];
        let w = wedge(&NORTH, FaceKind::Roof, &joints, 27.0, 0.0).unwrap();
        assert_eq!(w.mode, Mode::Falling);
        assert_eq!(w.fs, Some(0.0));
        // The trace fits the 5 m span; apex above the roof.
        let xs: Vec<f64> = w.vertices[1..].iter().map(|v| v[0]).collect();
        let span = xs.iter().cloned().fold(f64::MIN, f64::max) - xs.iter().cloned().fold(f64::MAX, f64::min);
        assert!((span - 5.0).abs() < 1e-9 && w.vertices[0][2] > 0.0 && (w.apex_height - w.vertices[0][2]).abs() < 1e-9);
        // Volume of the tetrahedron = base area * height / 3.
        assert!((w.volume - w.base_area * w.apex_height / 3.0).abs() < 1e-9);
        assert!((w.weight - 27.0 * w.volume).abs() < 1e-9);
        // A support pressure of p gives FS = p * A / W.
        let p = 2.0 * w.weight / w.base_area;
        let ws = wedge(&NORTH, FaceKind::Roof, &joints, 27.0, p).unwrap();
        assert_eq!(ws.mode, Mode::Stable, "support exceeds the weight");
        let p = 0.5 * w.weight / w.base_area;
        let ws = wedge(&NORTH, FaceKind::Roof, &joints, 27.0, p).unwrap();
        assert_eq!(ws.mode, Mode::Falling);
        assert!((ws.fs.unwrap() - 0.5).abs() < 1e-9);
    }

    #[test]
    fn single_plane_sliding_gives_tan_phi_over_tan_dip() {
        // East wall of a north tunnel: base plane dips 35 towards the opening (west),
        // two steep release planes (two vertical ones would meet in a line parallel
        // to the wall and never close a wedge).
        let joints = [j(35.0, 270.0, 40.0), j(80.0, 20.0, 30.0), j(80.0, 160.0, 30.0)];
        let w = wedge(&NORTH, FaceKind::RightWall, &joints, 26.0, 0.0).unwrap();
        assert_eq!(w.mode, Mode::Sliding1(0), "{w:?}");
        let expected = 40f64.to_radians().tan() / 35f64.to_radians().tan();
        assert!(
            (w.fs.unwrap() - expected).abs() < 1e-9,
            "{} vs {expected}",
            w.fs.unwrap()
        );
        // Cohesion raises the factor of safety by c A / T.
        let mut jc = joints;
        jc[0].cohesion = 10.0;
        let wc = wedge(&NORTH, FaceKind::RightWall, &jc, 26.0, 0.0).unwrap();
        let t = w.weight * 35f64.to_radians().sin();
        assert!((wc.fs.unwrap() - expected - 10.0 * w.face_areas[0] / t).abs() < 1e-9);
    }

    #[test]
    fn symmetric_two_plane_wedge_matches_hoek_bray() {
        // Two planes dipping 50 symmetric about the west, a steep back plane.
        let joints = [j(50.0, 230.0, 35.0), j(50.0, 310.0, 35.0), j(85.0, 90.0, 35.0)];
        let w = wedge(&NORTH, FaceKind::RightWall, &joints, 26.0, 0.0).unwrap();
        assert_eq!(w.mode, Mode::Sliding2(0, 1), "{w:?}");
        // Hoek & Bray: FS = sin(beta) / sin(xi / 2) * tan(phi) / tan(psi_i), beta = 90 (symmetric).
        let l = crate::stereonet::intersection(&joints[0].plane, &joints[1].plane).unwrap();
        let (n1, n2) = (joints[0].plane.normal(), joints[1].plane.normal());
        let xi = 180.0 - dot(n1, n2).acos().to_degrees();
        let expected = 35f64.to_radians().tan() / ((xi / 2.0).to_radians().sin() * l.plunge.to_radians().tan());
        assert!(
            (w.fs.unwrap() - expected).abs() < 1e-6,
            "{} vs {expected}",
            w.fs.unwrap()
        );
    }

    #[test]
    fn degenerate_and_all_faces() {
        // A set parallel to the axis in plan with the face: no closed wedge on that face.
        assert!(wedge(
            &NORTH,
            FaceKind::Roof,
            &[j(90.0, 90.0, 30.0), j(90.0, 270.0, 30.0), j(45.0, 0.0, 30.0)],
            27.0,
            0.0
        )
        .is_none());
        let joints = [j(70.0, 0.0, 30.0), j(70.0, 120.0, 30.0), j(70.0, 240.0, 30.0)];
        let all = analyse(&NORTH, &joints, 27.0, 0.0);
        // Planes 2 and 3 meet in a N-S line, parallel to both walls: roof wedge only.
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].face, FaceKind::Roof);
        let skew = [j(70.0, 10.0, 30.0), j(65.0, 130.0, 30.0), j(60.0, 250.0, 30.0)];
        assert_eq!(analyse(&NORTH, &skew, 27.0, 0.0).len(), 3);
        assert!(all.iter().all(|w| w.volume > 0.0));
        let limited = analyse(
            &Opening {
                max_length: 1.0,
                ..NORTH
            },
            &joints,
            27.0,
            0.0,
        );
        assert!(limited[0].volume < all[0].volume);
    }
}
