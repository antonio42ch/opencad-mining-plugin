//! Axis design for underground workings: a chain of straights and circular
//! curves (in plan) with a constant grade per element.
//!
//! * Lengths are **horizontal** (plan) lengths; Z changes by
//!   `grade × horizontal length`.
//! * Azimuth: 0 = north, clockwise. A right-hand curve turns clockwise
//!   (azimuth increases).
//! * Curves are discretised into chords no longer than the requested
//!   segment length (the last chord ends exactly at the curve end).

use std::f64::consts::PI;
use std::fmt;

use mining_core::{normalize360, Vec3};

/// Point + tangent azimuth: where an element starts (or ended).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Station {
    pub point: Vec3,
    pub azimuth: f64,
}

/// Grade of an element; positive goes up.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Grade {
    Percent(f64),
    Degrees(f64),
}

impl Grade {
    /// Rise per horizontal metre.
    pub fn ratio(self) -> f64 {
        match self {
            Grade::Percent(p) => p / 100.0,
            Grade::Degrees(d) => d.to_radians().tan(),
        }
    }
    pub fn percent(self) -> f64 {
        self.ratio() * 100.0
    }
    pub fn degrees(self) -> f64 {
        self.ratio().atan().to_degrees()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Turn {
    /// Clockwise.
    Right,
    /// Counter-clockwise.
    Left,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CurveSpec {
    /// Horizontal arc length.
    Length(f64),
    /// Central angle in degrees.
    Angle(f64),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Element {
    Straight { length: f64 },
    Curve { radius: f64, spec: CurveSpec, turn: Turn },
}

impl Element {
    /// Horizontal length of the element.
    pub fn length(&self) -> f64 {
        match *self {
            Element::Straight { length } => length,
            Element::Curve {
                spec: CurveSpec::Length(l),
                ..
            } => l,
            Element::Curve {
                radius,
                spec: CurveSpec::Angle(a),
                ..
            } => radius * a.to_radians(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AlignmentError {
    InvalidLength(f64),
    InvalidRadius(f64),
    /// Central angle must be in (0, 360].
    InvalidAngle(f64),
    InvalidSegment(f64),
    /// |grade| too steep to be meaningful (> 89.9°).
    InvalidGrade(f64),
    /// The curve would need more chords than the safety cap.
    TooManySegments(usize),
}

impl fmt::Display for AlignmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLength(v) => write!(f, "length must be > 0 (got {v})"),
            Self::InvalidRadius(v) => write!(f, "radius must be > 0 (got {v})"),
            Self::InvalidAngle(v) => write!(f, "central angle must be within (0, 360] (got {v})"),
            Self::InvalidSegment(v) => write!(f, "segment length must be > 0 (got {v})"),
            Self::InvalidGrade(v) => write!(f, "grade is too steep (got {v})"),
            Self::TooManySegments(n) => write!(f, "the curve would need {n} segments; increase the segment length"),
        }
    }
}
impl std::error::Error for AlignmentError {}

const MAX_SEGMENTS: usize = 100_000;

fn positive(v: f64, e: fn(f64) -> AlignmentError) -> Result<f64, AlignmentError> {
    if v.is_finite() && v > 0.0 {
        Ok(v)
    } else {
        Err(e(v))
    }
}

/// Unit plan direction of an azimuth.
fn dir(az: f64) -> [f64; 2] {
    let a = az.to_radians();
    [a.sin(), a.cos()]
}

/// Builds one element from `start`. Returns its vertices (starting with
/// `start.point`) and the end station.
pub fn build_element(
    start: Station,
    grade: Grade,
    element: Element,
    segment: f64,
) -> Result<(Vec<Vec3>, Station), AlignmentError> {
    let m = grade.ratio();
    if !m.is_finite() || grade.degrees().abs() > 89.9 {
        return Err(AlignmentError::InvalidGrade(grade.degrees()));
    }
    let p0 = start.point;
    match element {
        Element::Straight { length } => {
            let l = positive(length, AlignmentError::InvalidLength)?;
            let d = dir(start.azimuth);
            let end = [p0[0] + l * d[0], p0[1] + l * d[1], p0[2] + m * l];
            Ok((
                vec![p0, end],
                Station {
                    point: end,
                    azimuth: normalize360(start.azimuth),
                },
            ))
        }
        Element::Curve { radius, spec, turn } => {
            let r = positive(radius, AlignmentError::InvalidRadius)?;
            let seg = positive(segment, AlignmentError::InvalidSegment)?;
            let delta_deg = match spec {
                CurveSpec::Length(l) => positive(l, AlignmentError::InvalidLength)? / r * 180.0 / PI,
                CurveSpec::Angle(a) => {
                    if !(a.is_finite() && a > 0.0 && a <= 360.0) {
                        return Err(AlignmentError::InvalidAngle(a));
                    }
                    a
                }
            };
            let arc = r * delta_deg.to_radians();
            let n = (arc / seg).ceil().max(1.0);
            if n > MAX_SEGMENTS as f64 {
                return Err(AlignmentError::TooManySegments(n as usize));
            }
            let n = n as usize;
            let s = if turn == Turn::Right { 1.0 } else { -1.0 };
            // Centre on the inside of the turn; the radial vector from the
            // centre to the start point has azimuth `az0 - s*90`.
            let dc = dir(start.azimuth + s * 90.0);
            let c = [p0[0] + r * dc[0], p0[1] + r * dc[1]];
            let mut pts = Vec::with_capacity(n + 1);
            pts.push(p0);
            for i in 1..=n {
                let theta = delta_deg * i as f64 / n as f64;
                let dr = dir(start.azimuth - s * 90.0 + s * theta);
                let h = r * theta.to_radians();
                pts.push([c[0] + r * dr[0], c[1] + r * dr[1], p0[2] + m * h]);
            }
            let end = *pts.last().unwrap();
            Ok((
                pts,
                Station {
                    point: end,
                    azimuth: normalize360(start.azimuth + s * delta_deg),
                },
            ))
        }
    }
}

/// One designed element with its own grade.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DesignElement {
    pub element: Element,
    pub grade: Grade,
    /// Maximum chord length used to draw this element (curves).
    pub segment: f64,
}

/// A whole axis: start station + elements, rebuilt deterministically.
#[derive(Debug, Clone, PartialEq)]
pub struct AxisDesign {
    pub start: Station,
    pub elements: Vec<DesignElement>,
}

impl AxisDesign {
    pub fn new(start: Station) -> Self {
        AxisDesign {
            start,
            elements: Vec::new(),
        }
    }

    /// Polyline vertices (shared vertices between elements not repeated)
    /// and the end station.
    pub fn build(&self) -> Result<(Vec<Vec3>, Station), AlignmentError> {
        let mut pts = vec![self.start.point];
        let mut st = self.start;
        for e in &self.elements {
            let (p, end) = build_element(st, e.grade, e.element, e.segment)?;
            pts.extend_from_slice(&p[1..]);
            st = end;
        }
        Ok((pts, st))
    }

    /// Total horizontal length.
    pub fn length(&self) -> f64 {
        self.elements.iter().map(|e| e.element.length()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Vec3, b: Vec3, tol: f64) -> bool {
        (0..3).all(|i| (a[i] - b[i]).abs() < tol)
    }

    #[test]
    fn straight_with_grade_uses_horizontal_length() {
        let st = Station {
            point: [100.0, 200.0, 50.0],
            azimuth: 90.0,
        };
        let (pts, end) = build_element(st, Grade::Percent(-12.0), Element::Straight { length: 50.0 }, 1.0).unwrap();
        assert_eq!(pts.len(), 2);
        assert!(close(end.point, [150.0, 200.0, 44.0], 1e-9));
        assert_eq!(end.azimuth, 90.0);
    }

    #[test]
    fn grade_units_agree() {
        assert!((Grade::Degrees(45.0).percent() - 100.0).abs() < 1e-9);
        assert!((Grade::Percent(12.0).degrees() - 12f64.atan2(100.0).to_degrees()).abs() < 1e-12);
    }

    #[test]
    fn right_quarter_curve() {
        // Heading north, turn right 90° with R = 10: ends at (10, 10), heading east.
        let st = Station {
            point: [0.0, 0.0, 0.0],
            azimuth: 0.0,
        };
        let el = Element::Curve {
            radius: 10.0,
            spec: CurveSpec::Angle(90.0),
            turn: Turn::Right,
        };
        let (pts, end) = build_element(st, Grade::Percent(0.0), el, 1.0).unwrap();
        assert!(close(end.point, [10.0, 10.0, 0.0], 1e-9), "{:?}", end.point);
        assert!((end.azimuth - 90.0).abs() < 1e-9);
        // 15.708 m of arc in chords <= 1 m -> 16 chords.
        assert_eq!(pts.len(), 17);
        for w in pts.windows(2) {
            assert!(mining_core::dist(w[0], w[1]) <= 1.0 + 1e-9);
        }
        // All vertices on the circle centred at (10, 0).
        for p in &pts {
            assert!(((p[0] - 10.0).hypot(p[1]) - 10.0).abs() < 1e-9);
        }
    }

    #[test]
    fn left_curve_by_length_with_grade() {
        // Heading east, turn left (counter-clockwise) a half circle, R = 5, +10 %.
        let st = Station {
            point: [0.0, 0.0, 100.0],
            azimuth: 90.0,
        };
        let l = 5.0 * PI;
        let el = Element::Curve {
            radius: 5.0,
            spec: CurveSpec::Length(l),
            turn: Turn::Left,
        };
        let (_, end) = build_element(st, Grade::Percent(10.0), el, 0.5).unwrap();
        assert!(close(end.point, [0.0, 10.0, 100.0 + 0.1 * l], 1e-9), "{:?}", end.point);
        assert!((end.azimuth - 270.0).abs() < 1e-9);
    }

    #[test]
    fn series_continues_from_previous_end() {
        let mut d = AxisDesign::new(Station {
            point: [0.0; 3],
            azimuth: 0.0,
        });
        d.elements.push(DesignElement {
            element: Element::Straight { length: 20.0 },
            grade: Grade::Percent(0.0),
            segment: 1.0,
        });
        d.elements.push(DesignElement {
            element: Element::Curve {
                radius: 10.0,
                spec: CurveSpec::Angle(90.0),
                turn: Turn::Right,
            },
            grade: Grade::Percent(0.0),
            segment: 1.0,
        });
        d.elements.push(DesignElement {
            element: Element::Straight { length: 5.0 },
            grade: Grade::Percent(-10.0),
            segment: 1.0,
        });
        let (pts, end) = d.build().unwrap();
        assert!(close(end.point, [15.0, 30.0, -0.5], 1e-9), "{:?}", end.point);
        // No duplicated vertices at element joints.
        for w in pts.windows(2) {
            assert!(mining_core::dist(w[0], w[1]) > 1e-9);
        }
        assert!((d.length() - (25.0 + 5.0 * PI)).abs() < 1e-9);
    }

    #[test]
    fn invalid_inputs() {
        let st = Station {
            point: [0.0; 3],
            azimuth: 0.0,
        };
        let g = Grade::Percent(0.0);
        assert!(matches!(
            build_element(st, g, Element::Straight { length: 0.0 }, 1.0),
            Err(AlignmentError::InvalidLength(_))
        ));
        let c = |r, a| Element::Curve {
            radius: r,
            spec: CurveSpec::Angle(a),
            turn: Turn::Left,
        };
        assert!(matches!(
            build_element(st, g, c(0.0, 10.0), 1.0),
            Err(AlignmentError::InvalidRadius(_))
        ));
        assert!(matches!(
            build_element(st, g, c(5.0, 400.0), 1.0),
            Err(AlignmentError::InvalidAngle(_))
        ));
        assert!(matches!(
            build_element(st, g, c(5.0, 90.0), 0.0),
            Err(AlignmentError::InvalidSegment(_))
        ));
        assert!(matches!(
            build_element(st, g, c(1e6, 360.0), 1e-3),
            Err(AlignmentError::TooManySegments(_))
        ));
        assert!(matches!(
            build_element(st, Grade::Degrees(90.0), Element::Straight { length: 1.0 }, 1.0),
            Err(AlignmentError::InvalidGrade(_))
        ));
    }
}
