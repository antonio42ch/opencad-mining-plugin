//! Section profiles in local coordinates (u = right, v = up), centred, as an
//! implicit closed polygon. Vertex order is consistent for every station.

use mining_core::Vec2;
use std::f64::consts::PI;

use crate::SolidError;

#[derive(Debug, Clone, PartialEq)]
pub enum Profile {
    Rectangular {
        width: f64,
        height: f64,
    },
    Trapezoidal {
        bottom: f64,
        top: f64,
        height: f64,
    },
    Circular {
        radius: f64,
        segments: usize,
    },
    /// Horseshoe: vertical walls + semicircular crown. Walls are vertical only
    /// when `arc_radius = width / 2` (the default).
    Horseshoe {
        width: f64,
        wall_height: f64,
        arc_radius: Option<f64>,
        arc_points: usize,
    },
    /// Closed 2D polygon, re-centred on its vertex mean.
    Custom(Vec<Vec2>),
}

impl Profile {
    pub fn points(&self) -> Result<Vec<Vec2>, SolidError> {
        let pos = |v: f64, m: &'static str| {
            if v > 0.0 && v.is_finite() {
                Ok(v)
            } else {
                Err(SolidError::InvalidSection(m))
            }
        };
        Ok(match self {
            Self::Rectangular { width, height } => {
                let (w, h) = (pos(*width, "width")? / 2.0, pos(*height, "height")? / 2.0);
                vec![[-w, -h], [w, -h], [w, h], [-w, h]]
            }
            Self::Trapezoidal { bottom, top, height } => {
                let (b, t, h) = (
                    pos(*bottom, "bottom")? / 2.0,
                    pos(*top, "top")? / 2.0,
                    pos(*height, "height")? / 2.0,
                );
                vec![[-b, -h], [b, -h], [t, h], [-t, h]]
            }
            Self::Circular { radius, segments } => {
                let r = pos(*radius, "radius")?;
                let n = (*segments).max(8);
                (0..n)
                    .map(|i| {
                        let a = 2.0 * PI * i as f64 / n as f64;
                        [r * a.cos(), r * a.sin()]
                    })
                    .collect()
            }
            Self::Horseshoe {
                width,
                wall_height,
                arc_radius,
                arc_points,
            } => {
                let w = pos(*width, "width")?;
                let walls = pos(*wall_height, "wall height")?;
                let r = pos(arc_radius.unwrap_or(w / 2.0), "arc radius")?;
                let n = (*arc_points).max(9) | 1; // odd, so the crown apex is a vertex
                let total = walls + r;
                let floor = -total / 2.0;
                let cy = (walls - r) / 2.0;
                let mut p = vec![[-w / 2.0, floor], [w / 2.0, floor]];
                p.extend((0..n).map(|i| {
                    let a = PI * i as f64 / (n - 1) as f64;
                    [r * a.cos(), cy + r * a.sin()]
                }));
                p
            }
            Self::Custom(pts) => {
                let mut pts = pts.clone();
                if pts.len() > 3 && pts.first() == pts.last() {
                    pts.pop();
                }
                if pts.len() < 3 {
                    return Err(SolidError::InvalidSection("custom profile needs >= 3 vertices"));
                }
                let n = pts.len() as f64;
                let (mx, my) = pts.iter().fold((0.0, 0.0), |(x, y), p| (x + p[0] / n, y + p[1] / n));
                pts.into_iter().map(|p| [p[0] - mx, p[1] - my]).collect()
            }
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HAlign {
    Left,
    Center,
    Right,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VAlign {
    Top,
    Center,
    Bottom,
}

/// Offset that puts the chosen point of the profile's bounding box on the
/// axis. "Left" is `max u` because the frame's `right` axis points to the
/// physical left of the advance direction (AutoCAD OCS convention).
pub fn alignment_offset(profile: &[Vec2], h: HAlign, v: VAlign) -> Vec2 {
    let (mut minu, mut maxu, mut minv, mut maxv) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
    for p in profile {
        minu = minu.min(p[0]);
        maxu = maxu.max(p[0]);
        minv = minv.min(p[1]);
        maxv = maxv.max(p[1]);
    }
    let ax = match h {
        HAlign::Left => maxu,
        HAlign::Right => minu,
        HAlign::Center => (minu + maxu) / 2.0,
    };
    let ay = match v {
        VAlign::Top => maxv,
        VAlign::Bottom => minv,
        VAlign::Center => (minv + maxv) / 2.0,
    };
    [-ax, -ay]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn horseshoe_spans_full_height() {
        let p = Profile::Horseshoe {
            width: 4.0,
            wall_height: 3.0,
            arc_radius: None,
            arc_points: 17,
        }
        .points()
        .unwrap();
        let maxv = p.iter().map(|p| p[1]).fold(f64::MIN, f64::max);
        let minv = p.iter().map(|p| p[1]).fold(f64::MAX, f64::min);
        assert!((maxv - 2.5).abs() < 1e-12 && (minv + 2.5).abs() < 1e-12);
        // Walls vertical: first arc point sits above the right floor corner.
        assert!((p[2][0] - 2.0).abs() < 1e-12);
    }

    #[test]
    fn invalid_dimensions_rejected() {
        assert!(Profile::Rectangular {
            width: 0.0,
            height: 1.0
        }
        .points()
        .is_err());
        assert!(Profile::Custom(vec![[0.0, 0.0], [1.0, 0.0]]).points().is_err());
    }

    #[test]
    fn alignment_bottom_center() {
        let p = Profile::Rectangular {
            width: 4.0,
            height: 3.0,
        }
        .points()
        .unwrap();
        assert_eq!(alignment_offset(&p, HAlign::Center, VAlign::Bottom), [0.0, 1.5]);
    }
}
