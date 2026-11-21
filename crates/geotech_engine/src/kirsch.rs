//! Kirsch (1898): elastic stresses around a circular opening in a plane
//! strain far field `p1`, `p3` (no internal pressure). Angles from the
//! direction of `p1`. Non-circular openings use an equivalent radius: an
//! approximation, not a numerical analysis.

use crate::hoek_brown::HoekBrown;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Polar {
    pub sigma_r: f64,
    pub sigma_t: f64,
    pub tau: f64,
}

pub fn stresses(p1: f64, p3: f64, a: f64, r: f64, theta_deg: f64) -> Polar {
    let (s, d) = ((p1 + p3) / 2.0, (p1 - p3) / 2.0);
    let (q2, q4) = ((a / r).powi(2), (a / r).powi(4));
    let t = (2.0 * theta_deg).to_radians();
    Polar {
        sigma_r: s * (1.0 - q2) + d * (1.0 - 4.0 * q2 + 3.0 * q4) * t.cos(),
        sigma_t: s * (1.0 + q2) - d * (1.0 + 3.0 * q4) * t.cos(),
        tau: -d * (1.0 + 2.0 * q2 - 3.0 * q4) * t.sin(),
    }
}

/// Radius of the circle with the area of a `width x height` rectangle.
pub fn equivalent_radius(width: f64, height: f64) -> f64 {
    (width * height / std::f64::consts::PI).sqrt()
}

#[derive(Debug, Clone, PartialEq)]
pub struct Summary {
    /// Boundary tangential stress at the sides (theta = 90, max) and roof/floor
    /// direction (theta = 0, min).
    pub sigma_t_max: f64,
    pub sigma_t_min: f64,
    /// Rock mass strength over the maximum boundary stress.
    pub strength_factor: f64,
    /// Depth of the overstressed zone from the wall at theta = 90 (m; 0 none).
    pub overstressed_depth: f64,
    /// sigma_t around the boundary, every 5 degrees from p1.
    pub boundary: Vec<(f64, f64)>,
    /// (r / a, sigma_t, sigma_r) along theta = 90.
    pub profile: Vec<(f64, f64, f64)>,
}

pub fn analyse(p1: f64, p3: f64, a: f64, hb: &HoekBrown) -> Summary {
    let tmax = stresses(p1, p3, a, a, 90.0).sigma_t;
    let tmin = stresses(p1, p3, a, a, 0.0).sigma_t;
    let scm = hb.sigma_c();
    // Overstressed where sigma_t exceeds the Hoek-Brown strength at sigma_r.
    let mut depth = 0.0;
    let mut r = a;
    while r < 10.0 * a {
        let s = stresses(p1, p3, a, r, 90.0);
        if s.sigma_t <= hb.sigma1(s.sigma_r.max(0.0)) {
            break;
        }
        depth = r - a;
        r += a / 200.0;
    }
    Summary {
        sigma_t_max: tmax,
        sigma_t_min: tmin,
        strength_factor: if tmax > 0.0 { scm / tmax } else { f64::INFINITY },
        overstressed_depth: depth,
        boundary: (0..=72)
            .map(|k| (5.0 * k as f64, stresses(p1, p3, a, a, 5.0 * k as f64).sigma_t))
            .collect(),
        profile: (0..=40)
            .map(|k| {
                let q = 1.0 + k as f64 * 0.1;
                let s = stresses(p1, p3, a, q * a, 90.0);
                (q, s.sigma_t, s.sigma_r)
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kirsch_boundary_and_far_field() {
        let (p1, p3, a) = (20.0, 10.0, 2.0);
        // At the wall: sigma_t = 3 p1 - p3 at 90, 3 p3 - p1 at 0, sigma_r = tau = 0.
        let w = stresses(p1, p3, a, a, 90.0);
        assert!((w.sigma_t - 50.0).abs() < 1e-12 && w.sigma_r.abs() < 1e-12 && w.tau.abs() < 1e-12);
        assert!((stresses(p1, p3, a, a, 0.0).sigma_t - 10.0).abs() < 1e-12);
        // Hydrostatic: 2p everywhere on the boundary.
        assert!((stresses(10.0, 10.0, a, a, 37.0).sigma_t - 20.0).abs() < 1e-12);
        // Far away the field is recovered.
        let f = stresses(p1, p3, a, 1000.0 * a, 0.0);
        assert!((f.sigma_r - p1).abs() < 1e-4 && (f.sigma_t - p3).abs() < 1e-4);
        assert!((equivalent_radius(4.0, 4.0) - (16.0 / std::f64::consts::PI).sqrt()).abs() < 1e-12);
    }

    #[test]
    fn strength_factor_and_overstressed_zone() {
        let weak = HoekBrown::new(50.0, 40.0, 10.0, 0.0).unwrap();
        let s = analyse(20.0, 10.0, 2.0, &weak);
        assert!((s.sigma_t_max - 50.0).abs() < 1e-12);
        assert!(s.strength_factor < 1.0 && s.overstressed_depth > 0.0);
        let strong = HoekBrown::new(250.0, 90.0, 25.0, 0.0).unwrap();
        let s2 = analyse(5.0, 3.0, 2.0, &strong);
        assert!(s2.strength_factor > 1.0 && s2.overstressed_depth == 0.0);
        assert_eq!(s.boundary.len(), 73);
    }
}
