//! Kinematic analysis of a slope or excavation face (Markland test; Hoek &
//! Bray 1981; Goodman 1980 for flexural toppling).
//!
//! Direct toppling is not included: its published criteria differ between
//! sources and it is left out until verified.

use crate::stereonet::{az_diff, intersection, Line, Plane};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Face {
    /// Face dip (90 = vertical wall).
    pub dip: f64,
    pub dipdir: f64,
    /// Friction angle of the discontinuities (degrees).
    pub friction: f64,
    /// Lateral limits for planar sliding and toppling (degrees, usually 20).
    pub lateral: f64,
}

/// Apparent dip of the face in the azimuth `az`.
pub fn apparent_dip(face: &Face, az: f64) -> f64 {
    let c = (az - face.dipdir).to_radians().cos();
    if c <= 0.0 {
        return 0.0;
    }
    (face.dip.to_radians().tan() * c).atan().to_degrees()
}

/// Planar sliding: plane daylights (dip < face dip), is steeper than φ and
/// dips within the lateral limits of the face dip direction.
pub fn planar(face: &Face, p: &Plane) -> bool {
    p.dip > face.friction && p.dip < face.dip && az_diff(p.dipdir, face.dipdir) <= face.lateral
}

/// Wedge sliding along the intersection line: it plunges out of the face
/// (below the apparent face dip in its trend) and steeper than φ.
pub fn wedge(face: &Face, l: &Line) -> bool {
    az_diff(l.trend, face.dipdir) < 90.0 && l.plunge > face.friction && l.plunge < apparent_dip(face, l.trend)
}

/// Flexural toppling (Goodman): planes dipping into the face, within the
/// lateral limits, with `dip >= (90 - face dip) + φ`.
pub fn flexural_toppling(face: &Face, p: &Plane) -> bool {
    az_diff(p.dipdir, face.dipdir + 180.0) <= face.lateral && p.dip >= 90.0 - face.dip + face.friction
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Report {
    pub planes: usize,
    pub planar: usize,
    pub flexural: usize,
    pub intersections: usize,
    pub wedge: usize,
    /// Critical set pairs `(a, b)` for wedges (indices into the sets given).
    pub wedge_sets: Vec<(usize, usize)>,
    /// Critical sets for planar sliding and flexural toppling.
    pub planar_sets: Vec<usize>,
    pub flexural_sets: Vec<usize>,
}

impl Report {
    pub fn pct(n: usize, of: usize) -> f64 {
        if of == 0 {
            0.0
        } else {
            100.0 * n as f64 / of as f64
        }
    }
}

/// Runs the tests on every pole, every pair of poles (wedges; up to 400
/// poles, sampled evenly above that) and on the set mean planes.
pub fn analyse(face: &Face, poles: &[Plane], sets: &[Plane]) -> Report {
    let mut r = Report {
        planes: poles.len(),
        ..Default::default()
    };
    r.planar = poles.iter().filter(|p| planar(face, p)).count();
    r.flexural = poles.iter().filter(|p| flexural_toppling(face, p)).count();
    let step = poles.len().div_ceil(400).max(1);
    let sample: Vec<&Plane> = poles.iter().step_by(step).collect();
    for i in 0..sample.len() {
        for j in i + 1..sample.len() {
            if let Some(l) = intersection(sample[i], sample[j]) {
                r.intersections += 1;
                r.wedge += usize::from(wedge(face, &l));
            }
        }
    }
    for (i, a) in sets.iter().enumerate() {
        if planar(face, a) {
            r.planar_sets.push(i);
        }
        if flexural_toppling(face, a) {
            r.flexural_sets.push(i);
        }
        for (j, b) in sets.iter().enumerate().skip(i + 1) {
            if intersection(a, b).is_some_and(|l| wedge(face, &l)) {
                r.wedge_sets.push((i, j));
            }
        }
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    const FACE: Face = Face {
        dip: 60.0,
        dipdir: 90.0,
        friction: 30.0,
        lateral: 20.0,
    };

    #[test]
    fn markland_planar_and_wedge() {
        assert!(planar(&FACE, &Plane::new(45.0, 95.0)));
        assert!(!planar(&FACE, &Plane::new(25.0, 90.0)), "flatter than friction");
        assert!(!planar(&FACE, &Plane::new(70.0, 90.0)), "does not daylight");
        assert!(!planar(&FACE, &Plane::new(45.0, 120.0)), "outside the lateral limits");
        // Two planes whose intersection plunges 40 toward 90: wedge.
        let l = intersection(&Plane::new(55.0, 50.0), &Plane::new(55.0, 130.0)).unwrap();
        assert!(
            (l.trend - 90.0).abs() < 1e-9 && l.plunge > 30.0 && l.plunge < 60.0,
            "{l:?}"
        );
        assert!(wedge(&FACE, &l));
        assert!(
            !wedge(
                &FACE,
                &Line {
                    trend: 270.0,
                    plunge: 40.0
                }
            ),
            "plunges into the slope"
        );
        assert!((apparent_dip(&FACE, 90.0) - 60.0).abs() < 1e-9);
        assert!(apparent_dip(&FACE, 135.0) < 60.0 && apparent_dip(&FACE, 200.0) == 0.0);
    }

    #[test]
    fn flexural_toppling_and_report() {
        // Steep planes dipping into the face: dip >= 90 - 60 + 30 = 60.
        assert!(flexural_toppling(&FACE, &Plane::new(75.0, 270.0)));
        assert!(!flexural_toppling(&FACE, &Plane::new(50.0, 270.0)));
        assert!(!flexural_toppling(&FACE, &Plane::new(75.0, 90.0)));
        let poles = vec![
            Plane::new(45.0, 90.0),
            Plane::new(75.0, 270.0),
            Plane::new(10.0, 0.0),
            Plane::new(55.0, 50.0),
            Plane::new(55.0, 130.0),
        ];
        let sets = vec![Plane::new(55.0, 50.0), Plane::new(55.0, 130.0), Plane::new(75.0, 270.0)];
        let r = analyse(&FACE, &poles, &sets);
        assert_eq!((r.planes, r.planar, r.flexural), (5, 1, 1));
        assert_eq!(r.intersections, 10);
        assert!(r.wedge >= 1);
        assert_eq!(r.wedge_sets, vec![(0, 1)]);
        assert_eq!(r.flexural_sets, vec![2]);
        assert_eq!(Report::pct(1, 4), 25.0);
    }
}
