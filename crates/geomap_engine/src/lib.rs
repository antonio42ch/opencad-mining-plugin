//! Geological mapping in plan: a trace (structure, vein, contact) from a
//! point, an azimuth and a length, plus its annotation layout.
//!
//! Dip convention here is the geological one: **dip magnitude 0..90, positive
//! downwards**, with a separate dip direction. (Drillholes use signed dip,
//! -90 = down; [`dip_to_signed`] converts between both.)

use mining_core::{azimuth_to_cad_angle, normalize360, Vec3};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Anchor {
    Start,
    #[default]
    Center,
    End,
}

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
}

impl std::fmt::Display for MapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidLength(l) => write!(f, "trace length must be > 0 (got {l})"),
            Self::InvalidDip(d) => write!(f, "dip must be within 0..90 (got {d})"),
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

    /// Annotation lines (only non-empty fields), using the caller's labels.
    pub fn annotation_lines(&self, labels: &AnnotationLabels) -> Vec<String> {
        let o = &self.observation;
        let mut v = Vec::new();
        let mut push = |prefix: &str, val: &str| {
            if !val.trim().is_empty() {
                v.push(format!("{prefix}{val}"));
            }
        };
        push("", &o.rock_type);
        push(
            labels.thickness,
            &o.thickness.map(|t| format!("{t}")).unwrap_or_default(),
        );
        push(labels.mineralization, &o.mineralization);
        push(labels.alteration, &o.alteration);
        push("", &o.structure);
        push(labels.grade, &o.grade);
        push(labels.dip, &o.dip.map(|d| format!("{d:.0}")).unwrap_or_default());
        push(labels.sample, &o.sample_code);
        v
    }
}

/// Prefixes for annotation lines (localised by the UI layer).
pub struct AnnotationLabels {
    pub thickness: &'static str,
    pub mineralization: &'static str,
    pub alteration: &'static str,
    pub grade: &'static str,
    pub dip: &'static str,
    pub sample: &'static str,
}

/// Text height and insertion points for `count` lines next to `p`.
pub fn annotation_layout(p: Vec3, length: f64, count: usize) -> (f64, Vec<Vec3>) {
    let h = if length > 0.0 { length * 0.15 } else { 0.5 };
    let pts = (0..count)
        .map(|i| [p[0] + h * 0.5, p[1] - h * 0.5 - i as f64 * h * 1.5, p[2]])
        .collect();
    (h, pts)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Vec3, b: Vec3) -> bool {
        (0..3).all(|i| (a[i] - b[i]).abs() < 1e-9)
    }

    #[test]
    fn anchors() {
        let mut t = Trace {
            point: [0.0, 0.0, 5.0],
            azimuth: 90.0,
            length: 10.0,
            anchor: Anchor::Center,
            observation: Observation::default(),
        };
        let (s, e) = t.endpoints();
        assert!(close(s, [-5.0, 0.0, 5.0]) && close(e, [5.0, 0.0, 5.0]));
        t.anchor = Anchor::Start;
        assert!(close(t.endpoints().1, [10.0, 0.0, 5.0]));
        t.anchor = Anchor::End;
        assert!(close(t.endpoints().0, [-10.0, 0.0, 5.0]));
    }

    #[test]
    fn down_dip_goes_down() {
        let p = point_down_dip([0.0; 3], 0.0, 30.0, 2.0);
        assert!(close(p, [0.0, 2.0 * 30f64.to_radians().cos(), -1.0]));
    }

    #[test]
    fn validation_and_annotation() {
        let mut t = Trace {
            point: [0.0; 3],
            azimuth: 0.0,
            length: 0.0,
            anchor: Anchor::Center,
            observation: Observation::default(),
        };
        assert!(t.validate().is_err());
        t.length = 2.0;
        t.observation.rock_type = "Andesita".into();
        t.observation.thickness = Some(1.2);
        t.observation.dip = Some(65.0);
        let l = AnnotationLabels {
            thickness: "Pot: ",
            mineralization: "Min: ",
            alteration: "Alt: ",
            grade: "Ley: ",
            dip: "Bz: ",
            sample: "Mx: ",
        };
        assert_eq!(t.annotation_lines(&l), vec!["Andesita", "Pot: 1.2", "Bz: 65"]);
        let (h, pts) = annotation_layout([0.0; 3], 2.0, 3);
        assert!((h - 0.3).abs() < 1e-12 && pts.len() == 3);
    }
}
