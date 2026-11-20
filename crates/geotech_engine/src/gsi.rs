//! GSI and RMR/Q correlations.

/// GSI from RMR89' (dry, no orientation adjustment): `GSI = RMR89' - 5`
/// (Hoek, Kaiser & Bawden 1995), valid for RMR89' > 23.
pub fn gsi_from_rmr89(rmr89_dry: f64) -> Option<f64> {
    (rmr89_dry > 23.0).then_some(rmr89_dry - 5.0)
}

/// GSI from Q': `GSI = 9 ln Q' + 44` (Hoek et al. 1995).
pub fn gsi_from_q_prime(q_prime: f64) -> Option<f64> {
    (q_prime > 0.0).then(|| (9.0 * q_prime.ln() + 44.0).clamp(0.0, 100.0))
}

/// RMR <-> Q correlation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Correlation {
    /// Bieniawski (1976): RMR = 9 ln Q + 44.
    Bieniawski,
    /// Rutledge & Preston (1978): RMR = 5.9 ln Q + 43.
    RutledgePreston,
    /// Barton (1995): RMR = 15 log10 Q + 50.
    Barton,
}

impl Correlation {
    pub const ALL: [Correlation; 3] = [
        Correlation::Bieniawski,
        Correlation::RutledgePreston,
        Correlation::Barton,
    ];

    pub fn rmr_from_q(self, q: f64) -> Option<f64> {
        (q > 0.0).then(|| {
            let r = match self {
                Correlation::Bieniawski => 9.0 * q.ln() + 44.0,
                Correlation::RutledgePreston => 5.9 * q.ln() + 43.0,
                Correlation::Barton => 15.0 * q.log10() + 50.0,
            };
            r.clamp(0.0, 100.0)
        })
    }

    pub fn q_from_rmr(self, rmr: f64) -> f64 {
        match self {
            Correlation::Bieniawski => ((rmr - 44.0) / 9.0).exp(),
            Correlation::RutledgePreston => ((rmr - 43.0) / 5.9).exp(),
            Correlation::Barton => 10f64.powf((rmr - 50.0) / 15.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gsi_correlations() {
        assert_eq!(gsi_from_rmr89(70.0), Some(65.0));
        assert_eq!(gsi_from_rmr89(20.0), None);
        assert!((gsi_from_q_prime(1.0).unwrap() - 44.0).abs() < 1e-12);
        assert!((gsi_from_q_prime(10.0).unwrap() - (9.0 * 10f64.ln() + 44.0)).abs() < 1e-12);
    }

    #[test]
    fn correlations_roundtrip() {
        for c in Correlation::ALL {
            for rmr in [30.0, 55.0, 75.0] {
                assert!((c.rmr_from_q(c.q_from_rmr(rmr)).unwrap() - rmr).abs() < 1e-9, "{c:?}");
            }
        }
        // Q = 1 -> RMR 44 (Bieniawski), 43 (R&P), 50 (Barton).
        let r: Vec<f64> = Correlation::ALL.iter().map(|c| c.rmr_from_q(1.0).unwrap()).collect();
        assert_eq!(r, vec![44.0, 43.0, 50.0]);
    }
}
