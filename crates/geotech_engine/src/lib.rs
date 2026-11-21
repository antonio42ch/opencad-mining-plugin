//! Geotechnics engine (no CAD, no UI): rock mass classification (RMR89,
//! Q-system, GSI and correlations), Hoek-Brown 2002 strength and rock mass
//! modulus, geotechnical data import and classification of records;
//! structures (stereonet, sets, kinematics, wedges) and stresses and design
//! (in situ stress, Kirsch, convergence-confinement, pillars, crown pillars).
//!
//! Every rating follows the published tables; nothing is extrapolated. A
//! record that lacks a parameter is left unclassified and says why.

pub mod classify;
pub mod convergence;
pub mod data;
pub mod gsi;
pub mod hoek_brown;
pub mod kinematic;
pub mod kirsch;
pub mod locate;
pub mod pillar;
pub mod q;
pub mod rmr;
pub mod sets;
pub mod stereonet;
pub mod stress;
pub mod wedge;

/// Rock mass class I (very good) .. V (very poor), from RMR.
pub fn rmr_class(rmr: f64) -> u8 {
    if rmr > 80.0 {
        1
    } else if rmr > 60.0 {
        2
    } else if rmr > 40.0 {
        3
    } else if rmr > 20.0 {
        4
    } else {
        5
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classes_follow_bieniawski_1989() {
        // 100-81 I, 80-61 II, 60-41 III, 40-21 IV, <21 V.
        assert_eq!(
            [
                rmr_class(85.0),
                rmr_class(81.0),
                rmr_class(80.0),
                rmr_class(61.0),
                rmr_class(60.0)
            ],
            [1, 1, 2, 2, 3]
        );
        assert_eq!(
            [
                rmr_class(41.0),
                rmr_class(40.0),
                rmr_class(21.0),
                rmr_class(20.0),
                rmr_class(5.0)
            ],
            [3, 4, 4, 5, 5]
        );
    }
}
