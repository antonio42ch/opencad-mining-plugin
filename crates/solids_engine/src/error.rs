use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum SolidError {
    /// A polyline has fewer than 2 distinct vertices.
    TooFewVertices { what: &'static str },
    /// Sampling interval must be > 0.
    InvalidInterval(f64),
    /// The interval is longer than the axis: fewer than 2 stations.
    IntervalTooLong { interval: f64, length: f64 },
    /// A section dimension is not > 0 (or a custom profile is degenerate).
    InvalidSection(&'static str),
    /// Width or height collapsed at a station of a boundary method.
    DegenerateSection { station: usize },
}

impl fmt::Display for SolidError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooFewVertices { what } => {
                write!(f, "{what}: at least 2 distinct vertices are required")
            }
            Self::InvalidInterval(i) => write!(f, "sampling interval must be > 0 (got {i})"),
            Self::IntervalTooLong { interval, length } => {
                write!(f, "interval {interval} is longer than the axis ({length:.3})")
            }
            Self::InvalidSection(m) => write!(f, "invalid section: {m}"),
            Self::DegenerateSection { station } => {
                write!(f, "degenerate section at station {station}")
            }
        }
    }
}
impl std::error::Error for SolidError {}

/// Non-fatal issues reported alongside a successful result.
#[derive(Debug, Clone, PartialEq)]
pub enum SolidWarning {
    /// A boundary polyline does not cross the station plane; the nearest
    /// vertex was used instead (the legacy code did this silently).
    NoPlaneCrossing { station: usize, boundary: &'static str },
}
