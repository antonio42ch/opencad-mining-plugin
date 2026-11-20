//! Generalised Hoek-Brown criterion, 2002 edition (Hoek, Carranza-Torres &
//! Corkum), equivalent Mohr-Coulomb parameters and rock mass modulus (Hoek &
//! Diederichs 2006). Stresses in MPa.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HoekBrown {
    /// Intact rock UCS (MPa).
    pub sigci: f64,
    pub mb: f64,
    pub s: f64,
    pub a: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HbError {
    Invalid(&'static str, f64),
}

impl std::fmt::Display for HbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let HbError::Invalid(p, v) = self;
        write!(f, "invalid {p}: {v}")
    }
}
impl std::error::Error for HbError {}

fn need(name: &'static str, v: f64, ok: bool) -> Result<f64, HbError> {
    if v.is_finite() && ok {
        Ok(v)
    } else {
        Err(HbError::Invalid(name, v))
    }
}

impl HoekBrown {
    /// Parameters from GSI (0..100), mi (> 0) and disturbance D (0..1).
    pub fn new(sigci: f64, gsi: f64, mi: f64, d: f64) -> Result<Self, HbError> {
        let sigci = need("sigma_ci", sigci, sigci > 0.0)?;
        let gsi = need("GSI", gsi, (0.0..=100.0).contains(&gsi))?;
        let mi = need("mi", mi, mi > 0.0)?;
        let d = need("D", d, (0.0..=1.0).contains(&d))?;
        Ok(HoekBrown {
            sigci,
            mb: mi * ((gsi - 100.0) / (28.0 - 14.0 * d)).exp(),
            s: ((gsi - 100.0) / (9.0 - 3.0 * d)).exp(),
            a: 0.5 + ((-gsi / 15.0).exp() - (-20.0f64 / 3.0).exp()) / 6.0,
        })
    }

    /// Major principal effective stress at failure for `sig3` (MPa).
    pub fn sigma1(&self, sig3: f64) -> f64 {
        let t = self.mb * sig3 / self.sigci + self.s;
        sig3 + self.sigci * t.max(0.0).powf(self.a)
    }

    /// Uniaxial compressive strength of the rock mass.
    pub fn sigma_c(&self) -> f64 {
        self.sigci * self.s.powf(self.a)
    }

    /// Tensile strength (negative).
    pub fn sigma_t(&self) -> f64 {
        -self.s * self.sigci / self.mb
    }

    /// Global rock mass strength.
    pub fn sigma_cm(&self) -> f64 {
        let (mb, s, a) = (self.mb, self.s, self.a);
        self.sigci * (mb + 4.0 * s - a * (mb - 8.0 * s)) * (mb / 4.0 + s).powf(a - 1.0) / (2.0 * (1.0 + a) * (2.0 + a))
    }

    /// Equivalent Mohr-Coulomb `(c' MPa, phi' degrees)` fitted over
    /// `0 < sig3 < sig3max`.
    pub fn mohr_coulomb(&self, sig3max: f64) -> (f64, f64) {
        let (mb, s, a) = (self.mb, self.s, self.a);
        let n = sig3max / self.sigci;
        let k = 6.0 * a * mb * (s + mb * n).powf(a - 1.0);
        let d = 2.0 * (1.0 + a) * (2.0 + a);
        let phi = (k / (d + k)).asin();
        let c = self.sigci * ((1.0 + 2.0 * a) * s + (1.0 - a) * mb * n) * (s + mb * n).powf(a - 1.0)
            / ((1.0 + a) * (2.0 + a) * (1.0 + k / ((1.0 + a) * (2.0 + a))).sqrt());
        (c, phi.to_degrees())
    }
}

/// Upper confining stress for the MC fit (Hoek et al. 2002).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Application {
    /// Tunnels: `sig3max / sigcm = 0.47 (sigcm / gamma H)^-0.94`.
    Tunnel,
    /// Slopes: `sig3max / sigcm = 0.72 (sigcm / gamma H)^-0.91`.
    Slope,
}

/// `sig3max` (MPa) for depth / slope height `h` (m) and unit weight
/// `gamma` (MN/m3).
pub fn sigma3_max(hb: &HoekBrown, app: Application, gamma: f64, h: f64) -> f64 {
    let scm = hb.sigma_cm();
    let ratio = scm / (gamma * h).max(1e-9);
    scm * match app {
        Application::Tunnel => 0.47 * ratio.powf(-0.94),
        Application::Slope => 0.72 * ratio.powf(-0.91),
    }
}

/// Rock mass modulus (MPa), Hoek & Diederichs (2006). With the intact
/// modulus `ei` (MPa) the generalised equation is used, otherwise the
/// simplified one.
pub fn rock_mass_modulus(gsi: f64, d: f64, ei: Option<f64>) -> f64 {
    match ei {
        Some(ei) => ei * (0.02 + (1.0 - d / 2.0) / (1.0 + ((60.0 + 15.0 * d - gsi) / 11.0).exp())),
        None => 100_000.0 * (1.0 - d / 2.0) / (1.0 + ((75.0 + 25.0 * d - gsi) / 11.0).exp()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parameters_match_the_published_values() {
        // GSI 50, mi 10, D 0: mb = 1.677, s = 0.0039, a = 0.506 (Hoek et al. 2002).
        let hb = HoekBrown::new(100.0, 50.0, 10.0, 0.0).unwrap();
        assert!((hb.mb - 1.6767).abs() < 1e-3, "{}", hb.mb);
        assert!((hb.s - 0.003866).abs() < 1e-5, "{}", hb.s);
        assert!((hb.a - 0.50573).abs() < 1e-4, "{}", hb.a);
        // Disturbance lowers mb and s.
        let d = HoekBrown::new(100.0, 50.0, 10.0, 0.7).unwrap();
        assert!(d.mb < hb.mb && d.s < hb.s);
    }

    #[test]
    fn strength_relations_are_consistent() {
        let hb = HoekBrown::new(100.0, 50.0, 10.0, 0.0).unwrap();
        // sigma1 at sig3 = 0 is the rock mass UCS.
        assert!((hb.sigma1(0.0) - hb.sigma_c()).abs() < 1e-12);
        assert!(hb.sigma_t() < 0.0);
        // Intact rock (GSI 100, D 0): s = 1, a ~ 0.5, sigma_c = sigci.
        let intact = HoekBrown::new(100.0, 100.0, 10.0, 0.0).unwrap();
        assert!((intact.s - 1.0).abs() < 1e-12 && (intact.sigma_c() - 100.0).abs() < 0.5);
        assert!(
            hb.sigma_cm() > hb.sigma_c(),
            "global strength exceeds the UCS of the mass"
        );
        // Mohr-Coulomb fit: positive c, phi in a sensible range, sigma1 envelope above the fit at 0.
        let (c, phi) = hb.mohr_coulomb(10.0);
        assert!(c > 0.0 && phi > 20.0 && phi < 60.0, "c {c} phi {phi}");
        let s3m = sigma3_max(&hb, Application::Tunnel, 0.027, 500.0);
        assert!(s3m > 0.0 && s3m.is_finite());
    }

    #[test]
    fn modulus() {
        // Simplified: GSI 50, D 0 -> 100000 / (1 + e^(25/11)) ~ 9 341 MPa.
        assert!((rock_mass_modulus(50.0, 0.0, None) - 9340.7).abs() < 0.5);
        // Generalised with Ei: GSI 100 tends to ~ Ei.
        assert!((rock_mass_modulus(100.0, 0.0, Some(50_000.0)) - 50_000.0).abs() / 50_000.0 < 0.03);
        assert!(rock_mass_modulus(30.0, 1.0, None) < rock_mass_modulus(30.0, 0.0, None));
    }

    #[test]
    fn invalid_inputs() {
        assert!(HoekBrown::new(0.0, 50.0, 10.0, 0.0).is_err());
        assert!(HoekBrown::new(100.0, 120.0, 10.0, 0.0).is_err());
        assert!(HoekBrown::new(100.0, 50.0, 10.0, 1.5).is_err());
    }
}
