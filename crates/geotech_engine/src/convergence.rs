//! Convergence-confinement of a circular tunnel (Hoek 1998, "Tunnels in weak
//! rock"): ground reaction curve of Duncan Fama (1993) with Mohr-Coulomb
//! parameters, longitudinal displacement profile of Vlachopoulos &
//! Diederichs (2009), shotcrete and rock bolt support (Hoek 1998) and
//! squeezing classes of Hoek & Marinos (2000). MPa and metres.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ground {
    /// Tunnel radius (m).
    pub r0: f64,
    /// Hydrostatic in situ stress (MPa).
    pub p0: f64,
    /// Rock mass modulus (MPa) and Poisson's ratio.
    pub e: f64,
    pub nu: f64,
    /// Mohr-Coulomb cohesion (MPa) and friction (degrees).
    pub c: f64,
    pub phi: f64,
}

impl Ground {
    fn k(&self) -> f64 {
        let s = self.phi.to_radians().sin();
        (1.0 + s) / (1.0 - s)
    }
    /// Rock mass strength 2c cos(phi) / (1 - sin(phi)).
    pub fn sigma_cm(&self) -> f64 {
        let f = self.phi.to_radians();
        2.0 * self.c * f.cos() / (1.0 - f.sin())
    }
    /// Critical support pressure below which a plastic zone forms.
    pub fn p_cr(&self) -> f64 {
        ((2.0 * self.p0 - self.sigma_cm()) / (1.0 + self.k())).max(0.0)
    }
    /// Plastic radius for support pressure `pi` (`r0` when elastic).
    pub fn plastic_radius(&self, pi: f64) -> f64 {
        if pi >= self.p_cr() {
            return self.r0;
        }
        let (k, scm) = (self.k(), self.sigma_cm());
        self.r0 * (2.0 * (self.p0 * (k - 1.0) + scm) / ((1.0 + k) * ((k - 1.0) * pi + scm))).powf(1.0 / (k - 1.0))
    }
    /// Radial wall displacement for support pressure `pi`.
    pub fn displacement(&self, pi: f64) -> f64 {
        let f = self.r0 * (1.0 + self.nu) / self.e;
        let pcr = self.p_cr();
        if pi >= pcr {
            return f * (self.p0 - pi);
        }
        let rp = self.plastic_radius(pi);
        f * (2.0 * (1.0 - self.nu) * (self.p0 - pcr) * (rp / self.r0).powi(2) - (1.0 - 2.0 * self.nu) * (self.p0 - pi))
    }
}

/// Displacement at distance `x` from the face (m; negative ahead of it) as a
/// fraction of the maximum (Vlachopoulos & Diederichs 2009); `rp_max` is the
/// plastic radius of the unsupported tunnel.
pub fn ldp(x: f64, r0: f64, rp_max: f64) -> f64 {
    let rs = rp_max / r0;
    let u0 = (-0.15 * rs).exp() / 3.0;
    let xs = x / r0;
    if xs <= 0.0 {
        u0 * xs.exp()
    } else {
        1.0 - (1.0 - u0) * (-1.5 * xs / rs).exp()
    }
}

/// A support element: maximum pressure (MPa) and stiffness (MPa/m).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Support {
    pub p_max: f64,
    pub k: f64,
}

/// Shotcrete or concrete lining (Hoek 1998).
pub fn shotcrete(r0: f64, t: f64, sigma_cc: f64, e_c: f64, nu_c: f64) -> Support {
    let ri = r0 - t;
    Support {
        p_max: sigma_cc / 2.0 * (1.0 - ri * ri / (r0 * r0)),
        k: e_c / ((1.0 + nu_c) * r0) * (r0 * r0 - ri * ri) / ((1.0 - 2.0 * nu_c) * r0 * r0 + ri * ri),
    }
}

/// Mechanically anchored rock bolts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bolt {
    /// Ultimate load of one bolt (MN).
    pub t_bf: f64,
    /// Circumferential and longitudinal spacing (m).
    pub sc: f64,
    pub sl: f64,
    /// Length and diameter (m).
    pub length: f64,
    pub diameter: f64,
    /// Steel modulus (MPa).
    pub e_s: f64,
    /// Deformation-load constant of anchor and plate (m/MN).
    pub q: f64,
}

/// Rock bolt support (Hoek 1998).
pub fn bolts(r0: f64, b: &Bolt) -> Support {
    let inv_k = b.sc * b.sl / r0 * (4.0 * b.length / (std::f64::consts::PI * b.diameter * b.diameter * b.e_s) + b.q);
    Support {
        p_max: b.t_bf / (b.sc * b.sl),
        k: 1.0 / inv_k,
    }
}

/// Supports acting together: stiffnesses add up to the smallest yield
/// displacement of the elements (Hoek 1998).
pub fn combine(list: &[Support]) -> Option<Support> {
    if list.is_empty() {
        return None;
    }
    let k: f64 = list.iter().map(|s| s.k).sum();
    let u = list.iter().map(|s| s.p_max / s.k).fold(f64::INFINITY, f64::min);
    Some(Support { p_max: k * u, k })
}

#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    pub p_cr: f64,
    /// Unsupported plastic radius and wall displacement.
    pub rp_max: f64,
    pub u_max: f64,
    /// Strain u / r0 (%) unsupported, and its Hoek-Marinos class.
    pub strain: f64,
    pub squeezing: &'static str,
    /// Displacement when the support is installed (m).
    pub u_install: f64,
    /// Equilibrium pressure and displacement (`None` without support or when
    /// the support yields before equilibrium).
    pub p_eq: Option<f64>,
    pub u_eq: Option<f64>,
    pub fs: Option<f64>,
    /// Ground curve (u, p) from p0 to 0, and support curve (u, p).
    pub ground_curve: Vec<(f64, f64)>,
    pub support_curve: Vec<(f64, f64)>,
}

/// Hoek & Marinos (2000) classes by tunnel strain (%).
pub fn squeezing_class(strain: f64) -> &'static str {
    match strain {
        s if s < 1.0 => "few_support_problems",
        s if s < 2.5 => "minor_squeezing",
        s if s < 5.0 => "severe_squeezing",
        s if s < 10.0 => "very_severe_squeezing",
        _ => "extreme_squeezing",
    }
}

pub fn analyse(g: &Ground, support: Option<Support>, install_distance: f64) -> Outcome {
    let rp_max = g.plastic_radius(0.0);
    let u_max = g.displacement(0.0);
    let u_install = ldp(install_distance, g.r0, rp_max) * u_max;
    let ground_curve: Vec<(f64, f64)> = (0..=50)
        .map(|k| {
            let p = g.p0 * (1.0 - k as f64 / 50.0);
            (g.displacement(p), p)
        })
        .collect();
    let mut out = Outcome {
        p_cr: g.p_cr(),
        rp_max,
        u_max,
        strain: 100.0 * u_max / g.r0,
        squeezing: squeezing_class(100.0 * u_max / g.r0),
        u_install,
        p_eq: None,
        u_eq: None,
        fs: None,
        ground_curve,
        support_curve: Vec::new(),
    };
    let Some(s) = support else { return out };
    let u_yield = u_install + s.p_max / s.k;
    out.support_curve = vec![
        (u_install, 0.0),
        (u_yield, s.p_max),
        (u_yield.max(u_max) * 1.2, s.p_max),
    ];
    // Equilibrium: ground pressure = support pressure (bisection on p).
    let f = |p: f64| p - (s.k * (g.displacement(p) - u_install)).clamp(0.0, s.p_max);
    let (mut lo, mut hi) = (0.0, g.p0);
    if f(lo) >= 0.0 {
        return out;
    }
    for _ in 0..200 {
        let mid = (lo + hi) / 2.0;
        if f(mid) < 0.0 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let p = (lo + hi) / 2.0;
    if p < s.p_max - 1e-9 {
        out.p_eq = Some(p);
        out.u_eq = Some(g.displacement(p));
        out.fs = Some(s.p_max / p);
    } else {
        // The support yields: report the pressure it carries.
        out.p_eq = Some(s.p_max);
        out.u_eq = Some(g.displacement(s.p_max));
        out.fs = Some(1.0);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const G: Ground = Ground {
        r0: 5.0,
        p0: 10.0,
        e: 5000.0,
        nu: 0.3,
        c: 1.0,
        phi: 30.0,
    };

    #[test]
    fn ground_curve_limits() {
        // sigma_cm = 2 c cos30 / (1 - sin30) = 3.464; k = 3; pcr = (20 - 3.464) / 4.
        assert!((G.sigma_cm() - 3.4641).abs() < 1e-4);
        assert!((G.p_cr() - 4.1340).abs() < 1e-4);
        // Above pcr the response is elastic: u = r0 (1 + nu) / E (p0 - pi).
        assert!((G.displacement(6.0) - 5.0 * 1.3 / 5000.0 * 4.0).abs() < 1e-12);
        // Continuous at pcr; plastic radius grows as support drops.
        let pcr = G.p_cr();
        assert!((G.displacement(pcr - 1e-9) - G.displacement(pcr + 1e-9)).abs() < 1e-9);
        assert!(G.plastic_radius(0.0) > G.plastic_radius(2.0) && G.plastic_radius(pcr) == G.r0);
        // Strong rock: no plastic zone at all.
        let strong = Ground { c: 10.0, ..G };
        assert_eq!(strong.p_cr(), 0.0);
        assert!((strong.displacement(0.0) - 5.0 * 1.3 / 5000.0 * 10.0).abs() < 1e-12);
    }

    #[test]
    fn ldp_and_supports() {
        let rp = 10.0;
        let u0 = (-0.15 * 2.0f64).exp() / 3.0;
        assert!((ldp(0.0, 5.0, rp) - u0).abs() < 1e-12);
        assert!(ldp(-20.0, 5.0, rp) < 0.01 && (ldp(500.0, 5.0, rp) - 1.0).abs() < 1e-6);
        // Shotcrete 0.2 m on r0 = 5: p_max = 30 / 2 (1 - 4.8^2 / 25).
        let sc = shotcrete(5.0, 0.2, 30.0, 30_000.0, 0.25);
        assert!((sc.p_max - 15.0 * (1.0 - 23.04 / 25.0)).abs() < 1e-12);
        let b = bolts(
            5.0,
            &Bolt {
                t_bf: 0.1,
                sc: 1.0,
                sl: 1.0,
                length: 3.0,
                diameter: 0.025,
                e_s: 200_000.0,
                q: 0.03,
            },
        );
        assert!((b.p_max - 0.1).abs() < 1e-12 && b.k > 0.0);
        let both = combine(&[sc, b]).unwrap();
        assert!((both.k - sc.k - b.k).abs() < 1e-9 && both.p_max <= sc.p_max + b.p_max);
        assert_eq!(squeezing_class(0.5), "few_support_problems");
        assert_eq!(squeezing_class(3.0), "severe_squeezing");
        assert_eq!(squeezing_class(12.0), "extreme_squeezing");
    }

    #[test]
    fn equilibrium_with_support() {
        let sc = shotcrete(5.0, 0.2, 30.0, 30_000.0, 0.25);
        let o = analyse(&G, Some(sc), 2.0);
        let (p, u) = (o.p_eq.unwrap(), o.u_eq.unwrap());
        assert!(p > 0.0 && p < sc.p_max);
        // On both curves at equilibrium.
        assert!((G.displacement(p) - u).abs() < 1e-9);
        assert!((sc.k * (u - o.u_install) - p).abs() < 1e-6);
        assert!((o.fs.unwrap() - sc.p_max / p).abs() < 1e-9);
        assert!(o.u_install < o.u_max && u < o.u_max);
        let none = analyse(&G, None, 2.0);
        assert!(none.p_eq.is_none() && none.strain > 0.0 && none.ground_curve.len() == 51);
    }
}
