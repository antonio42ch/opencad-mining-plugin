//! Geological mapping: a trace (structure, vein, contact) from a point, an
//! azimuth and a length, and the projection of a mapped trace to another
//! height along its dip.
//!
//! Dip convention here is the geological one: **dip magnitude 0..90, positive
//! downwards**, with a separate dip direction. (Drillholes use signed dip,
//! -90 = down; [`dip_to_signed`] converts between both.)

pub mod project;

use mining_core::{azimuth_to_cad_angle, normalize360, Vec3};

pub use project::{project_polyline, DipDirection, Height, ProjectError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Anchor {
    Start,
    #[default]
    Center,
    End,
}

/// Metadata stored with a trace (XDATA); it never produces drawn text.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Observation {
    pub rock_type: String,
    pub thickness: Option<f64>,
    pub mineralization: String,
    pub alteration: String,
    pub structure: String,
    pub grade: String,
    /// Dip magnitude 0..90 (positive down).
    pub dip: Option<f64>,
    /// Dip direction (azimuth) when it is known.
    pub dip_direction: Option<f64>,
    pub sample_code: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Trace {
    pub point: Vec3,
    pub azimuth: f64,
    pub length: f64,
    pub anchor: Anchor,
    pub observation: Observation,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MapError {
    InvalidLength(f64),
    InvalidDip(f64),
    /// A coordinate is not a finite number.
    InvalidPoint,
}

impl std::fmt::Display for MapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidLength(l) => write!(f, "trace length must be > 0 (got {l})"),
            Self::InvalidDip(d) => write!(f, "dip must be within 0..90 (got {d})"),
            Self::InvalidPoint => write!(f, "the coordinates must be finite numbers"),
        }
    }
}
impl std::error::Error for MapError {}

/// Plan point at `dist` from `o` along geological azimuth `az` (Z kept).
pub fn point_along(o: Vec3, az: f64, dist: f64) -> Vec3 {
    let a = azimuth_to_cad_angle(az).to_radians();
    [o[0] + dist * a.cos(), o[1] + dist * a.sin(), o[2]]
}

/// 3D point `len` metres down-dip from `o` (dip positive down).
pub fn point_down_dip(o: Vec3, dip_direction: f64, dip: f64, len: f64) -> Vec3 {
    let h = azimuth_to_cad_angle(dip_direction).to_radians();
    let v = (-dip).to_radians();
    [
        o[0] + len * v.cos() * h.cos(),
        o[1] + len * v.cos() * h.sin(),
        o[2] + len * v.sin(),
    ]
}

/// Geological dip (0..90 down) -> signed drillhole dip (-90 = down).
pub fn dip_to_signed(dip: f64) -> f64 {
    -dip
}

impl Trace {
    pub fn validate(&self) -> Result<(), MapError> {
        if self.point.iter().any(|c| !c.is_finite()) {
            return Err(MapError::InvalidPoint);
        }
        if !(self.length > 0.0 && self.length.is_finite()) {
            return Err(MapError::InvalidLength(self.length));
        }
        if let Some(d) = self.observation.dip {
            if !(0.0..=90.0).contains(&d) {
                return Err(MapError::InvalidDip(d));
            }
        }
        Ok(())
    }

    /// `(start, end)` of the trace according to the anchor.
    pub fn endpoints(&self) -> (Vec3, Vec3) {
        let back = normalize360(self.azimuth + 180.0);
        let (p, l) = (self.point, self.length);
        match self.anchor {
            Anchor::Start => (p, point_along(p, self.azimuth, l)),
            Anchor::End => (point_along(p, back, l), p),
            Anchor::Center => (point_along(p, back, l / 2.0), point_along(p, self.azimuth, l / 2.0)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Vec3, b: Vec3) -> bool {
        (0..3).all(|i| (a[i] - b[i]).abs() < 1e-9)
    }

    fn trace(anchor: Anchor) -> Trace {
        Trace {
            point: [0.0, 0.0, 5.0],
            azimuth: 90.0,
            length: 10.0,
            anchor,
            observation: Observation::default(),
        }
    }

    #[test]
    fn anchors() {
        let (s, e) = trace(Anchor::Center).endpoints();
        assert!(close(s, [-5.0, 0.0, 5.0]) && close(e, [5.0, 0.0, 5.0]));
        assert!(close(trace(Anchor::Start).endpoints().1, [10.0, 0.0, 5.0]));
        assert!(close(trace(Anchor::End).endpoints().0, [-10.0, 0.0, 5.0]));
    }

    #[test]
    fn down_dip_goes_down() {
        let p = point_down_dip([0.0; 3], 0.0, 30.0, 2.0);
        assert!(close(p, [0.0, 2.0 * 30f64.to_radians().cos(), -1.0]));
    }

    #[test]
    fn validation() {
        let mut t = trace(Anchor::Center);
        t.length = 0.0;
        assert!(t.validate().is_err());
        t.length = 2.0;
        t.observation.dip = Some(95.0);
        assert!(matches!(t.validate(), Err(MapError::InvalidDip(_))));
        t.observation.dip = Some(65.0);
        assert!(t.validate().is_ok());
        t.point[1] = f64::NAN;
        assert_eq!(t.validate(), Err(MapError::InvalidPoint));
    }
}
