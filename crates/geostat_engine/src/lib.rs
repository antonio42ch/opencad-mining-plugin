//! Geostatistics engine (no CAD, no UI).
//!
//! `bridge` turns drillholes into composites; `composite`, `stats` and
//! `decluster` prepare the data; `vario`, `model` and `fit` characterise the
//! continuity; `search` + `kriging` estimate; `crossval` validates;
//! `surface` estimates 2D contact surfaces.
//!
//! Conventions: metres, degrees at the API boundary, Z up. Orientation:
//! azimuth of the major axis clockwise from north, dip of the major axis
//! positive **downwards**, rake rotates the other two axes about the major.

pub mod bridge;
pub mod composite;
pub mod crossval;
pub mod decluster;
pub mod fit;
pub mod kriging;
pub mod linalg;
pub mod model;
pub mod search;
pub mod stats;
pub mod surface;
pub mod vario;

use mining_core::Vec3;

/// One datum for estimation (usually a composite).
#[derive(Debug, Clone, PartialEq)]
pub struct Sample {
    pub xyz: Vec3,
    pub value: f64,
    /// Support length (m), used as weight.
    pub length: f64,
    /// Index of the drillhole (for the per-hole limit of a neighbourhood).
    pub hole: u32,
    /// Estimation domain, 1-based; 0 = none.
    pub domain: u32,
}

/// Runs `f` over `0..n` in parallel when the feature is on.
pub fn par_collect<T: Send>(n: usize, f: impl Fn(usize) -> T + Sync + Send) -> Vec<T> {
    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;
        (0..n).into_par_iter().map(f).collect()
    }
    #[cfg(not(feature = "parallel"))]
    {
        (0..n).map(f).collect()
    }
}
