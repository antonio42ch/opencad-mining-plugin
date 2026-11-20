//! RMR89 (Bieniawski 1989, Table 4.1). Ratings by class from the official
//! tables; a class lower bound belongs to the class above it is not assumed:
//! each range is `[low, high)` except where the table says ">".

/// Strength of intact rock: UCS (MPa) -> rating 0..15.
pub fn rating_ucs(ucs: f64) -> f64 {
    if ucs > 250.0 {
        15.0
    } else if ucs >= 100.0 {
        12.0
    } else if ucs >= 50.0 {
        7.0
    } else if ucs >= 25.0 {
        4.0
    } else if ucs >= 5.0 {
        2.0
    } else if ucs >= 1.0 {
        1.0
    } else {
        0.0
    }
}

/// Strength from the point load index Is50 (MPa). The table gives no point
/// load class below 1 MPa (UCS is required there): `None`.
pub fn rating_is50(is50: f64) -> Option<f64> {
    Some(if is50 > 10.0 {
        15.0
    } else if is50 >= 4.0 {
        12.0
    } else if is50 >= 2.0 {
        7.0
    } else if is50 >= 1.0 {
        4.0
    } else {
        return None;
    })
}

/// RQD (%) -> rating 3..20.
pub fn rating_rqd(rqd: f64) -> f64 {
    if rqd >= 90.0 {
        20.0
    } else if rqd >= 75.0 {
        17.0
    } else if rqd >= 50.0 {
        13.0
    } else if rqd >= 25.0 {
        8.0
    } else {
        3.0
    }
}

/// Spacing of discontinuities (m) -> rating 5..20.
pub fn rating_spacing(s: f64) -> f64 {
    if s > 2.0 {
        20.0
    } else if s >= 0.6 {
        15.0
    } else if s >= 0.2 {
        10.0
    } else if s >= 0.06 {
        8.0
    } else {
        5.0
    }
}

/// Persistence / discontinuity length (m) -> 0..6 (Table 4.1 E).
pub fn rating_persistence(l: f64) -> f64 {
    if l < 1.0 {
        6.0
    } else if l < 3.0 {
        4.0
    } else if l < 10.0 {
        2.0
    } else if l <= 20.0 {
        1.0
    } else {
        0.0
    }
}

/// Aperture / separation (mm) -> 0..6 (Table 4.1 E).
pub fn rating_aperture(mm: f64) -> f64 {
    if mm <= 0.0 {
        6.0
    } else if mm < 0.1 {
        5.0
    } else if mm <= 1.0 {
        4.0
    } else if mm <= 5.0 {
        1.0
    } else {
        0.0
    }
}

/// Roughness keyword (es/en) or rating 0..6.
pub fn rating_roughness(text: &str) -> Option<f64> {
    let t = text.trim().to_lowercase();
    if let Ok(v) = t.replace(',', ".").parse::<f64>() {
        return (0.0..=6.0).contains(&v).then_some(v);
    }
    Some(match t.as_str() {
        "very rough" | "muy rugosa" | "muy rugoso" => 6.0,
        "rough" | "rugosa" | "rugoso" => 5.0,
        "slightly rough" | "ligeramente rugosa" | "ligeramente rugoso" | "poco rugosa" => 3.0,
        "smooth" | "lisa" | "liso" => 1.0,
        "slickensided" | "espejo de falla" | "estriada" | "pulida" => 0.0,
        _ => return None,
    })
}

/// Weathering keyword (es/en) or rating 0..6.
pub fn rating_weathering(text: &str) -> Option<f64> {
    let t = text.trim().to_lowercase();
    if let Ok(v) = t.replace(',', ".").parse::<f64>() {
        return (0.0..=6.0).contains(&v).then_some(v);
    }
    Some(match t.as_str() {
        "unweathered" | "sana" | "fresca" | "no meteorizada" => 6.0,
        "slightly weathered" | "slightly" | "ligeramente meteorizada" | "levemente meteorizada" => 5.0,
        "moderately weathered" | "moderately" | "moderadamente meteorizada" => 3.0,
        "highly weathered" | "highly" | "muy meteorizada" | "altamente meteorizada" => 1.0,
        "decomposed" | "descompuesta" => 0.0,
        _ => return None,
    })
}

/// Infilling rating 0..6 (numeric only: the five table classes are
/// none 6, hard < 5 mm 4, hard > 5 mm 2, soft < 5 mm 2, soft > 5 mm 0).
pub fn rating_infilling(v: f64) -> Option<f64> {
    (0.0..=6.0).contains(&v).then_some(v)
}

/// Groundwater keyword (es/en) or rating 0..15.
pub fn rating_water(text: &str) -> Option<f64> {
    let t = text.trim().to_lowercase();
    if let Ok(v) = t.replace(',', ".").parse::<f64>() {
        return (0.0..=15.0).contains(&v).then_some(v);
    }
    Some(match t.as_str() {
        "completely dry" | "dry" | "seco" | "completamente seco" => 15.0,
        "damp" | "humedo" | "húmedo" => 10.0,
        "wet" | "mojado" => 7.0,
        "dripping" | "goteo" | "goteando" => 4.0,
        "flowing" | "flujo" | "fluyendo" => 0.0,
        _ => return None,
    })
}

/// The five RMR89 ratings (without orientation adjustment).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RmrParts {
    pub strength: f64,
    pub rqd: f64,
    pub spacing: f64,
    pub condition: f64,
    pub water: f64,
}

impl RmrParts {
    /// Basic RMR89 (0..100).
    pub fn total(&self) -> f64 {
        self.strength + self.rqd + self.spacing + self.condition + self.water
    }
    /// RMR89' used for GSI: dry (15) and no orientation adjustment.
    pub fn dry_total(&self) -> f64 {
        self.total() - self.water + 15.0
    }
}

/// Orientation adjustment for tunnels and mines (Table 4.1 B):
/// very favourable .. very unfavourable.
pub const ORIENTATION_TUNNELS: [f64; 5] = [0.0, -2.0, -5.0, -10.0, -12.0];

/// Mean spacing (m) from fractures per metre.
pub fn spacing_from_ff(ff: f64) -> Option<f64> {
    (ff > 0.0).then(|| 1.0 / ff)
}

/// RQD (%) estimated from the fracture frequency (Priest & Hudson 1976):
/// `RQD = 100 e^(-0.1 l) (0.1 l + 1)`.
pub fn rqd_from_ff(ff: f64) -> Option<f64> {
    (ff >= 0.0).then(|| 100.0 * (-0.1 * ff).exp() * (0.1 * ff + 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_ratings_and_boundaries() {
        assert_eq!(
            [
                rating_ucs(300.0),
                rating_ucs(250.0),
                rating_ucs(100.0),
                rating_ucs(99.9),
                rating_ucs(50.0)
            ],
            [15.0, 12.0, 12.0, 7.0, 7.0]
        );
        assert_eq!(
            [rating_ucs(25.0), rating_ucs(5.0), rating_ucs(1.0), rating_ucs(0.5)],
            [4.0, 2.0, 1.0, 0.0]
        );
        assert_eq!(
            [
                rating_rqd(95.0),
                rating_rqd(80.0),
                rating_rqd(60.0),
                rating_rqd(30.0),
                rating_rqd(10.0)
            ],
            [20.0, 17.0, 13.0, 8.0, 3.0]
        );
        assert_eq!(
            [
                rating_spacing(3.0),
                rating_spacing(1.0),
                rating_spacing(0.3),
                rating_spacing(0.1),
                rating_spacing(0.01)
            ],
            [20.0, 15.0, 10.0, 8.0, 5.0]
        );
        assert_eq!(
            [
                rating_is50(12.0),
                rating_is50(5.0),
                rating_is50(3.0),
                rating_is50(1.5),
                rating_is50(0.5)
            ],
            [Some(15.0), Some(12.0), Some(7.0), Some(4.0), None]
        );
        assert_eq!(
            [
                rating_persistence(0.5),
                rating_persistence(2.0),
                rating_persistence(5.0),
                rating_persistence(15.0),
                rating_persistence(25.0)
            ],
            [6.0, 4.0, 2.0, 1.0, 0.0]
        );
        assert_eq!(
            [
                rating_aperture(0.0),
                rating_aperture(0.05),
                rating_aperture(0.5),
                rating_aperture(3.0),
                rating_aperture(8.0)
            ],
            [6.0, 5.0, 4.0, 1.0, 0.0]
        );
    }

    #[test]
    fn keywords_and_numeric_ratings() {
        assert_eq!(rating_roughness("Rugosa"), Some(5.0));
        assert_eq!(rating_roughness("slickensided"), Some(0.0));
        assert_eq!(rating_roughness("3"), Some(3.0));
        assert_eq!(rating_roughness("9"), None);
        assert_eq!(rating_weathering("Moderadamente meteorizada"), Some(3.0));
        assert_eq!(rating_water("Húmedo"), Some(10.0));
        assert_eq!(rating_water("dripping"), Some(4.0));
        assert_eq!(rating_water("nublado"), None);
    }

    #[test]
    fn worked_example() {
        // UCS 150 MPa (12), RQD 70 % (13), spacing 0.5 m (10), condition 20, damp (10) -> 65, class II.
        let p = RmrParts {
            strength: rating_ucs(150.0),
            rqd: rating_rqd(70.0),
            spacing: rating_spacing(0.5),
            condition: 20.0,
            water: rating_water("damp").unwrap(),
        };
        assert_eq!(p.total(), 65.0);
        assert_eq!(p.dry_total(), 70.0);
        assert_eq!(crate::rmr_class(p.total()), 2);
    }

    #[test]
    fn derived_from_fracture_frequency() {
        assert_eq!(spacing_from_ff(4.0), Some(0.25));
        assert_eq!(spacing_from_ff(0.0), None);
        // Priest & Hudson: 6 fractures/m -> RQD ~ 87.8 %.
        assert!((rqd_from_ff(6.0).unwrap() - 87.81).abs() < 0.01);
        assert_eq!(rqd_from_ff(0.0), Some(100.0));
    }
}
