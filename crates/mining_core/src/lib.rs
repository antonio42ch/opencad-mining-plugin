//! Shared math for the Open Mining plugin. `std` only: no CAD host, no UI.
//!
//! Conventions: metres, angles in degrees at the API boundary, `f64`, Z up.
//! Azimuth: 0 = North (+Y), clockwise. Dip: 0 = horizontal, -90 = straight down.

pub mod angles;
pub mod vec3;

pub use angles::*;
pub use vec3::*;

/// Sorts by an `f64` key without panicking on `NaN` (NaN sorts last).
pub fn sort_by_f64<T>(items: &mut [T], key: impl Fn(&T) -> f64) {
    items.sort_by(|a, b| key(a).total_cmp(&key(b)));
}

/// Translation applied to world coordinates before drawing, so large UTM
/// values (e.g. N = 8 500 000) keep full precision in the viewport.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LocalOrigin {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl LocalOrigin {
    pub fn to_local(&self, p: Vec3) -> Vec3 {
        [p[0] - self.x, p[1] - self.y, p[2] - self.z]
    }
    pub fn to_world(&self, p: Vec3) -> Vec3 {
        [p[0] + self.x, p[1] + self.y, p[2] + self.z]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_with_nan_does_not_panic() {
        let mut v = vec![3.0, f64::NAN, 1.0];
        sort_by_f64(&mut v, |x| *x);
        assert_eq!(&v[..2], &[1.0, 3.0]);
        assert!(v[2].is_nan());
    }

    #[test]
    fn local_origin_roundtrip() {
        let o = LocalOrigin {
            x: 500_000.0,
            y: 8_500_000.0,
            z: 0.0,
        };
        let p = [500_123.25, 8_500_456.5, 3_200.0];
        assert_eq!(o.to_world(o.to_local(p)), p);
    }
}
