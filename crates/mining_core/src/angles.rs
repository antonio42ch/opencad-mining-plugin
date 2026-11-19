//! Azimuth / quadrant bearing conversions.

use std::fmt;

/// Wraps an angle into `[0, 360)`.
pub fn normalize360(a: f64) -> f64 {
    let r = ((a % 360.0) + 360.0) % 360.0;
    if r >= 360.0 {
        0.0
    } else {
        r
    }
}

/// Geological azimuth (0 = N, clockwise) <-> CAD angle (0 = E, counter-clockwise).
/// The formula is its own inverse.
pub fn azimuth_to_cad_angle(az: f64) -> f64 {
    normalize360(90.0 - az)
}
pub fn cad_angle_to_azimuth(ang: f64) -> f64 {
    normalize360(90.0 - ang)
}

/// Azimuth from `p1` to `p2` in plan.
pub fn azimuth_between(p1: [f64; 2], p2: [f64; 2]) -> f64 {
    normalize360(90.0 - (p2[1] - p1[1]).atan2(p2[0] - p1[0]).to_degrees())
}

/// Azimuth in degrees -> quadrant bearing ("N45E").
pub fn azimuth_to_bearing(az: f64) -> String {
    let az = normalize360(az);
    if az <= 90.0 {
        format!("N{:.0}E", az)
    } else if az <= 180.0 {
        format!("S{:.0}E", 180.0 - az)
    } else if az <= 270.0 {
        format!("S{:.0}W", az - 180.0)
    } else {
        format!("N{:.0}W", 360.0 - az)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BearingError {
    Empty,
    /// Not a number and not a `N..E` / `S..W` form.
    Malformed(String),
    /// Quadrant angle outside 0..=90.
    OutOfRange(String),
}

impl fmt::Display for BearingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "empty bearing"),
            Self::Malformed(s) => write!(f, "malformed bearing '{s}'"),
            Self::OutOfRange(s) => write!(f, "bearing angle out of 0..90 in '{s}'"),
        }
    }
}
impl std::error::Error for BearingError {}

/// Parses a plain azimuth ("135.5") or a quadrant bearing ("N45E", "S 30 W",
/// "N30O" with Spanish O = W). Invariant culture (dot decimal; a single comma
/// is also accepted as decimal separator).
pub fn parse_bearing(text: &str) -> Result<f64, BearingError> {
    let t: String = text
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_uppercase()
        .replace(',', ".");
    if t.is_empty() {
        return Err(BearingError::Empty);
    }
    if let Ok(v) = t.parse::<f64>() {
        if v.is_finite() {
            return Ok(normalize360(v));
        }
    }
    let malformed = || BearingError::Malformed(text.to_string());
    let first = t.chars().next().ok_or_else(malformed)?;
    let last = t.chars().last().ok_or_else(malformed)?;
    if !matches!(first, 'N' | 'S') || !matches!(last, 'E' | 'W' | 'O') || t.len() < 3 {
        return Err(malformed());
    }
    let g: f64 = t[1..t.len() - 1].parse().map_err(|_| malformed())?;
    if !(0.0..=90.0).contains(&g) {
        return Err(BearingError::OutOfRange(text.to_string()));
    }
    Ok(match (first, last == 'E') {
        ('N', true) => g,
        ('S', true) => 180.0 - g,
        ('S', false) => 180.0 + g,
        _ => normalize360(360.0 - g),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn azimuth_roundtrip() {
        for az in [0.0, 45.0, 90.0, 200.0, 359.0] {
            assert!((cad_angle_to_azimuth(azimuth_to_cad_angle(az)) - az).abs() < 1e-9);
        }
    }

    #[test]
    fn bearing_parse() {
        assert_eq!(parse_bearing("N45E"), Ok(45.0));
        assert_eq!(parse_bearing("S 30 E"), Ok(150.0));
        assert_eq!(parse_bearing("S30W"), Ok(210.0));
        assert_eq!(parse_bearing("N10O"), Ok(350.0));
        assert_eq!(parse_bearing("N0W"), Ok(0.0));
        assert_eq!(parse_bearing("135,5"), Ok(135.5));
        assert!(matches!(parse_bearing("N95E"), Err(BearingError::OutOfRange(_))));
        assert!(matches!(parse_bearing("xyz"), Err(BearingError::Malformed(_))));
        assert_eq!(parse_bearing("  "), Err(BearingError::Empty));
    }

    #[test]
    fn bearing_format() {
        assert_eq!(azimuth_to_bearing(45.0), "N45E");
        assert_eq!(azimuth_to_bearing(150.0), "S30E");
        assert_eq!(azimuth_to_bearing(350.0), "N10W");
    }

    #[test]
    fn azimuth_between_points() {
        assert!((azimuth_between([0.0, 0.0], [0.0, 1.0]) - 0.0).abs() < 1e-9);
        assert!((azimuth_between([0.0, 0.0], [1.0, 0.0]) - 90.0).abs() < 1e-9);
    }
}
