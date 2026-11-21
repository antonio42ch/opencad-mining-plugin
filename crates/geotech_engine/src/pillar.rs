//! Pillar loads by tributary area, empirical pillar strength and the scaled
//! span of crown pillars. MPa and metres.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    Square,
    Rectangular,
    /// Long rib pillars (length >> width).
    Rib,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Layout {
    pub shape: Shape,
    pub width: f64,
    /// Pillar length (rectangular only).
    pub length: f64,
    pub height: f64,
    /// Room (opening) width.
    pub room: f64,
}

impl Layout {
    /// Average pillar stress for vertical stress `sv` (tributary area).
    pub fn load(&self, sv: f64) -> f64 {
        let (w, l, c) = (self.width, self.length, self.room);
        match self.shape {
            Shape::Square => sv * (w + c).powi(2) / (w * w),
            Shape::Rectangular => sv * (w + c) * (l + c) / (w * l),
            Shape::Rib => sv * (w + c) / w,
        }
    }
    /// Extraction ratio.
    pub fn extraction(&self) -> f64 {
        let (w, l, c) = (self.width, self.length, self.room);
        match self.shape {
            Shape::Square => 1.0 - (w * w) / ((w + c) * (w + c)),
            Shape::Rectangular => 1.0 - (w * l) / ((w + c) * (l + c)),
            Shape::Rib => 1.0 - w / (w + c),
        }
    }
    /// Effective width (4 A / perimeter) for rectangular pillars.
    pub fn effective_width(&self) -> f64 {
        match self.shape {
            Shape::Rectangular => 2.0 * self.width * self.length / (self.width + self.length),
            _ => self.width,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Formula {
    /// Lunder & Pakalnis (1997), hard rock.
    LunderPakalnis,
    /// Obert & Duvall (1967).
    ObertDuvall,
    /// Bieniawski (1975).
    Bieniawski,
    /// Hedley & Grant (1972), K scaled from Elliot Lake (133 MPa for UCS 230).
    HedleyGrant,
}

/// Pillar strength (MPa) for intact rock UCS `ucs`.
pub fn strength(f: Formula, ucs: f64, w: f64, h: f64) -> f64 {
    let r = w / h;
    match f {
        Formula::LunderPakalnis => {
            let cpav = 0.46 * (r + 0.75).log10().powf(1.4 / r);
            let kappa = ((1.0 - cpav) / (1.0 + cpav)).acos().tan();
            0.44 * ucs * (0.68 + 0.52 * kappa)
        }
        Formula::ObertDuvall => ucs * (0.778 + 0.222 * r),
        Formula::Bieniawski => ucs * (0.64 + 0.36 * r),
        Formula::HedleyGrant => 133.0 * ucs / 230.0 * w.sqrt() / h.powf(0.75),
    }
}

/// Scaled span of a crown pillar (Carter 1992):
/// `Cs = S [gamma / (t (1 + SR) (1 - 0.4 cos(theta)))]^0.5`, with span `s`,
/// strike length `l` (SR = S / L), thickness `t`, specific gravity `gamma`
/// (t/m3) and dip `theta` of the orebody (degrees).
pub fn scaled_span(s: f64, l: f64, t: f64, gamma: f64, theta: f64) -> f64 {
    let sr = s / l;
    s * (gamma / (t * (1.0 + sr) * (1.0 - 0.4 * theta.to_radians().cos()))).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tributary_area() {
        let sq = Layout {
            shape: Shape::Square,
            width: 10.0,
            length: 10.0,
            height: 5.0,
            room: 5.0,
        };
        assert!((sq.load(10.0) - 22.5).abs() < 1e-12);
        assert!((sq.extraction() - (1.0 - 100.0 / 225.0)).abs() < 1e-12);
        let rib = Layout {
            shape: Shape::Rib,
            ..sq
        };
        assert!((rib.load(10.0) - 15.0).abs() < 1e-12);
        let rect = Layout {
            shape: Shape::Rectangular,
            length: 20.0,
            ..sq
        };
        assert!((rect.load(10.0) - 10.0 * 15.0 * 25.0 / 200.0).abs() < 1e-12);
        assert!((rect.effective_width() - 400.0 / 30.0).abs() < 1e-12);
    }

    #[test]
    fn strength_formulas() {
        // Lunder-Pakalnis at w/h = 1: Cpav = 0.46 log(1.75)^1.4.
        let cpav = 0.46 * 1.75f64.log10().powf(1.4);
        let kappa = ((1.0 - cpav) / (1.0 + cpav)).acos().tan();
        assert!((strength(Formula::LunderPakalnis, 100.0, 5.0, 5.0) - 44.0 * (0.68 + 0.52 * kappa)).abs() < 1e-9);
        assert!(
            (cpav - 0.0635).abs() < 1e-3 && (kappa - 0.538).abs() < 2e-3,
            "{cpav} {kappa}"
        );
        // About 0.42 UCS at w/h = 1 (the published Lunder-Pakalnis curve).
        assert!((strength(Formula::LunderPakalnis, 1.0, 5.0, 5.0) - 0.422).abs() < 2e-3);
        // Strength grows with w/h.
        assert!(
            strength(Formula::LunderPakalnis, 100.0, 10.0, 5.0) > strength(Formula::LunderPakalnis, 100.0, 5.0, 5.0)
        );
        assert!((strength(Formula::ObertDuvall, 100.0, 5.0, 5.0) - 100.0).abs() < 1e-12);
        assert!((strength(Formula::Bieniawski, 100.0, 10.0, 5.0) - 136.0).abs() < 1e-12);
        assert!((strength(Formula::HedleyGrant, 230.0, 4.0, 1.0) - 266.0).abs() < 1e-9);
    }

    #[test]
    fn carter_scaled_span() {
        // Vertical orebody (cos 90 = 0), S = 10, L = 100, t = 20, gamma 2.7.
        let cs = scaled_span(10.0, 100.0, 20.0, 2.7, 90.0);
        assert!((cs - 10.0 * (2.7f64 / (20.0 * 1.1)).sqrt()).abs() < 1e-12);
        assert!(
            scaled_span(10.0, 100.0, 20.0, 2.7, 0.0) > cs,
            "a flat orebody scales larger"
        );
    }
}
