//! Nested anisotropic variogram models.

use mining_core::{cross, dot, scale, Vec3};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Spherical,
    Exponential,
    Gaussian,
}

impl Kind {
    /// Normalised structure value for reduced distance `h` (range = 1).
    pub fn shape(self, h: f64) -> f64 {
        match self {
            Kind::Spherical => {
                if h >= 1.0 {
                    1.0
                } else {
                    1.5 * h - 0.5 * h * h * h
                }
            }
            Kind::Exponential => 1.0 - (-3.0 * h).exp(),
            Kind::Gaussian => 1.0 - (-3.0 * h * h).exp(),
        }
    }
}

/// Principal axes of an anisotropic ellipsoid.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Orientation {
    /// Azimuth of the major axis, clockwise from north.
    pub azimuth: f64,
    /// Plunge of the major axis, positive downwards.
    pub dip: f64,
    /// Rotation of the semi-major / minor axes about the major axis.
    pub rake: f64,
}

impl Orientation {
    /// Unit vectors `[major, semi-major, minor]`. With dip = rake = 0 the
    /// semi-major axis is horizontal (to the right of the major axis) and the
    /// minor axis is vertical.
    pub fn axes(&self) -> [Vec3; 3] {
        let (az, dip, rake) = (self.azimuth.to_radians(), self.dip.to_radians(), self.rake.to_radians());
        let major = [az.sin() * dip.cos(), az.cos() * dip.cos(), -dip.sin()];
        let semi0 = [az.cos(), -az.sin(), 0.0];
        let minor0 = cross(semi0, major);
        let semi = [
            semi0[0] * rake.cos() + minor0[0] * rake.sin(),
            semi0[1] * rake.cos() + minor0[1] * rake.sin(),
            semi0[2] * rake.cos() + minor0[2] * rake.sin(),
        ];
        let minor = [
            -semi0[0] * rake.sin() + minor0[0] * rake.cos(),
            -semi0[1] * rake.sin() + minor0[1] * rake.cos(),
            -semi0[2] * rake.sin() + minor0[2] * rake.cos(),
        ];
        [major, semi, minor]
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Structure {
    pub kind: Kind,
    /// Partial sill (contribution).
    pub sill: f64,
    /// Ranges along `[major, semi-major, minor]`.
    pub ranges: [f64; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct VariogramModel {
    pub nugget: f64,
    pub structures: Vec<Structure>,
    pub orientation: Orientation,
}

impl VariogramModel {
    pub fn total_sill(&self) -> f64 {
        self.nugget + self.structures.iter().map(|s| s.sill).sum::<f64>()
    }

    /// Variogram at vector `h` (0 at the origin).
    pub fn gamma(&self, h: Vec3) -> f64 {
        let l2 = dot(h, h);
        if l2 < 1e-24 {
            return 0.0;
        }
        let ax = self.orientation.axes();
        let p = [dot(h, ax[0]), dot(h, ax[1]), dot(h, ax[2])];
        let mut g = self.nugget;
        for s in &self.structures {
            let r2: f64 = (0..3).map(|k| (p[k] / s.ranges[k].max(1e-9)).powi(2)).sum();
            g += s.sill * s.kind.shape(r2.sqrt());
        }
        g
    }

    /// Covariance `C(h) = total sill - gamma(h)` (total sill at the origin).
    pub fn cov(&self, h: Vec3) -> f64 {
        if dot(h, h) < 1e-24 {
            self.total_sill()
        } else {
            self.total_sill() - self.gamma(h)
        }
    }

    pub fn gamma_along(&self, dir: Vec3, dist: f64) -> f64 {
        self.gamma(scale(dir, dist))
    }

    /// Largest range of any structure (for search radii defaults).
    pub fn max_range(&self) -> f64 {
        self.structures.iter().flat_map(|s| s.ranges).fold(0.0, f64::max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model() -> VariogramModel {
        VariogramModel {
            nugget: 2.0,
            structures: vec![Structure {
                kind: Kind::Spherical,
                sill: 8.0,
                ranges: [100.0, 50.0, 20.0],
            }],
            orientation: Orientation {
                azimuth: 90.0,
                dip: 0.0,
                rake: 0.0,
            },
        }
    }

    #[test]
    fn ranges_follow_the_orientation() {
        let m = model();
        // Major axis points east (az 90): range 100 along X, 50 along Y, 20 along Z.
        assert!((m.gamma([100.0, 0.0, 0.0]) - 10.0).abs() < 1e-9);
        assert!((m.gamma([0.0, 50.0, 0.0]) - 10.0).abs() < 1e-9);
        assert!((m.gamma([0.0, 0.0, 20.0]) - 10.0).abs() < 1e-9);
        assert!(m.gamma([50.0, 0.0, 0.0]) < 10.0);
        assert_eq!(m.gamma([0.0; 3]), 0.0);
        // Nugget appears immediately away from the origin, covariance is sill - gamma.
        assert!((m.gamma([1e-3, 0.0, 0.0]) - 2.0).abs() < 0.1);
        assert!((m.cov([200.0, 0.0, 0.0]) - 0.0).abs() < 1e-9);
        assert_eq!(m.cov([0.0; 3]), 10.0);
    }

    #[test]
    fn axes_are_orthonormal_for_any_orientation() {
        for (az, dip, rake) in [
            (0.0, 0.0, 0.0),
            (325.0, 30.0, 15.0),
            (56.0, -40.0, 90.0),
            (10.0, 89.0, 0.0),
        ] {
            let a = Orientation { azimuth: az, dip, rake }.axes();
            for i in 0..3 {
                assert!((dot(a[i], a[i]) - 1.0).abs() < 1e-12);
                for j in i + 1..3 {
                    assert!(dot(a[i], a[j]).abs() < 1e-12, "{az} {dip} {rake}");
                }
            }
        }
    }

    #[test]
    fn exponential_and_gaussian_reach_95_percent_at_the_range() {
        for k in [Kind::Exponential, Kind::Gaussian] {
            assert!((k.shape(1.0) - 0.9502).abs() < 1e-3);
        }
    }
}
