//! Orientations and the equal-area (Schmidt) lower-hemisphere projection.
//! Axes: x = east, y = north, z = up. Angles in degrees.

use mining_core::{cross, dot, Vec3};

/// A plane by dip (0..90) and dip direction (0..360, clockwise from north).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plane {
    pub dip: f64,
    pub dipdir: f64,
}

/// A line by trend (azimuth) and plunge (positive down).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Line {
    pub trend: f64,
    pub plunge: f64,
}

fn rad(d: f64) -> f64 {
    d.to_radians()
}

pub fn norm360(a: f64) -> f64 {
    a.rem_euclid(360.0)
}

/// Smallest difference between two azimuths (0..180).
pub fn az_diff(a: f64, b: f64) -> f64 {
    let d = norm360(a - b);
    d.min(360.0 - d)
}

impl Plane {
    pub fn new(dip: f64, dipdir: f64) -> Plane {
        Plane {
            dip,
            dipdir: norm360(dipdir),
        }
    }
    /// Unit normal pointing up.
    pub fn normal(&self) -> Vec3 {
        let (d, a) = (rad(self.dip), rad(self.dipdir));
        [d.sin() * a.sin(), d.sin() * a.cos(), d.cos()]
    }
    /// Pole: the downward normal.
    pub fn pole(&self) -> Line {
        Line {
            trend: norm360(self.dipdir + 180.0),
            plunge: 90.0 - self.dip,
        }
    }
    /// Plane whose upward normal is `n` (any length; `None` for zero).
    pub fn from_normal(n: Vec3) -> Option<Plane> {
        let l = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        if l < 1e-12 {
            return None;
        }
        let mut u = [n[0] / l, n[1] / l, n[2] / l];
        if u[2] < 0.0 {
            u = [-u[0], -u[1], -u[2]];
        }
        let dip = u[2].clamp(-1.0, 1.0).acos().to_degrees();
        let dipdir = if dip < 1e-9 {
            0.0
        } else {
            norm360(u[0].atan2(u[1]).to_degrees())
        };
        Some(Plane { dip, dipdir })
    }
    /// Points of the great circle (lower hemisphere), unit-radius projection.
    pub fn great_circle(&self, n: usize) -> Vec<[f64; 2]> {
        let strike = self.dipdir - 90.0;
        (0..=n)
            .map(|k| {
                // Angle along the plane from the strike direction (0..180).
                let t = rad(180.0 * k as f64 / n as f64);
                let s = [rad(strike).sin(), rad(strike).cos(), 0.0];
                let (d, a) = (rad(self.dip), rad(self.dipdir));
                let down = [d.cos() * a.sin(), d.cos() * a.cos(), -d.sin()];
                let v = [
                    s[0] * t.cos() + down[0] * t.sin(),
                    s[1] * t.cos() + down[1] * t.sin(),
                    s[2] * t.cos() + down[2] * t.sin(),
                ];
                project(Line::from_vector(v).expect("unit vector"))
            })
            .collect()
    }
}

impl Line {
    /// Unit vector pointing down along the line.
    pub fn vector(&self) -> Vec3 {
        let (t, p) = (rad(self.trend), rad(self.plunge));
        [p.cos() * t.sin(), p.cos() * t.cos(), -p.sin()]
    }
    /// Line of a vector (oriented to the lower hemisphere).
    pub fn from_vector(v: Vec3) -> Option<Line> {
        let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        if l < 1e-12 {
            return None;
        }
        let mut u = [v[0] / l, v[1] / l, v[2] / l];
        if u[2] > 0.0 {
            u = [-u[0], -u[1], -u[2]];
        }
        let plunge = (-u[2]).clamp(-1.0, 1.0).asin().to_degrees();
        let trend = if plunge > 90.0 - 1e-9 {
            0.0
        } else {
            norm360(u[0].atan2(u[1]).to_degrees())
        };
        Some(Line { trend, plunge })
    }
}

/// Line of intersection of two planes (`None` when parallel).
pub fn intersection(a: &Plane, b: &Plane) -> Option<Line> {
    let c = cross(a.normal(), b.normal());
    if dot(c, c) < 1e-10 {
        return None;
    }
    Line::from_vector(c)
}

/// Equal-area lower-hemisphere projection on a unit-radius net
/// (`r = sqrt(2) sin((90 - plunge) / 2)`), `[x east, y north]`.
pub fn project(l: Line) -> [f64; 2] {
    let r = 2f64.sqrt() * rad((90.0 - l.plunge) / 2.0).sin();
    [r * rad(l.trend).sin(), r * rad(l.trend).cos()]
}

/// Inverse of [`project`] for a point inside the unit circle.
pub fn unproject(p: [f64; 2]) -> Option<Line> {
    let r = p[0].hypot(p[1]);
    if r > 1.0 + 1e-12 {
        return None;
    }
    let plunge = 90.0 - 2.0 * (r / 2f64.sqrt()).min(1.0).asin().to_degrees();
    let trend = if r < 1e-12 {
        0.0
    } else {
        norm360(p[0].atan2(p[1]).to_degrees())
    };
    Some(Line { trend, plunge })
}

/// Angle between two axes (0..90; a line and its opposite are the same axis).
pub fn axial_angle(a: Vec3, b: Vec3) -> f64 {
    dot(a, b).abs().min(1.0).acos().to_degrees()
}

/// Pole density (% of the poles per 1 % of the hemisphere) at `at`: poles
/// inside the cone whose area is 1 % of the hemisphere (half-angle 8.11°).
pub fn density(poles: &[Vec3], at: Vec3) -> f64 {
    if poles.is_empty() {
        return 0.0;
    }
    let cos_c = 0.99;
    let n = poles.iter().filter(|p| dot(**p, at).abs() >= cos_c).count();
    100.0 * n as f64 / poles.len() as f64
}

/// Density on a square grid over the net: `(n x n values, NaN outside)`.
pub fn density_grid(poles: &[Vec3], n: usize) -> Vec<f64> {
    let mut g = vec![f64::NAN; n * n];
    for j in 0..n {
        for i in 0..n {
            let p = [
                -1.0 + 2.0 * i as f64 / (n - 1) as f64,
                -1.0 + 2.0 * j as f64 / (n - 1) as f64,
            ];
            if let Some(l) = unproject(p) {
                g[j * n + i] = density(poles, l.vector());
            }
        }
    }
    g
}

/// Contour segments of a grid at `level` (marching squares), in net
/// coordinates.
pub fn contour(grid: &[f64], n: usize, level: f64) -> Vec<[[f64; 2]; 2]> {
    let xy = |i: f64, j: f64| [-1.0 + 2.0 * i / (n - 1) as f64, -1.0 + 2.0 * j / (n - 1) as f64];
    let mut out = Vec::new();
    for j in 0..n - 1 {
        for i in 0..n - 1 {
            let v = [
                grid[j * n + i],
                grid[j * n + i + 1],
                grid[(j + 1) * n + i + 1],
                grid[(j + 1) * n + i],
            ];
            if v.iter().any(|x| x.is_nan()) {
                continue;
            }
            let c = [
                (i as f64, j as f64),
                (i as f64 + 1.0, j as f64),
                (i as f64 + 1.0, j as f64 + 1.0),
                (i as f64, j as f64 + 1.0),
            ];
            let mut pts = Vec::new();
            for e in 0..4 {
                let (a, b) = (e, (e + 1) % 4);
                if (v[a] >= level) != (v[b] >= level) {
                    let t = (level - v[a]) / (v[b] - v[a]);
                    pts.push(xy(c[a].0 + t * (c[b].0 - c[a].0), c[a].1 + t * (c[b].1 - c[a].1)));
                }
            }
            if pts.len() >= 2 {
                out.push([pts[0], pts[1]]);
            }
            if pts.len() == 4 {
                out.push([pts[2], pts[3]]);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    #[test]
    fn projection_reference_points() {
        // Vertical line at the centre, horizontal lines on the circle.
        assert_eq!(
            project(Line {
                trend: 0.0,
                plunge: 90.0
            }),
            [0.0, 0.0]
        );
        let p = project(Line {
            trend: 90.0,
            plunge: 0.0,
        });
        assert!(close(p[0], 1.0, 1e-12) && close(p[1], 0.0, 1e-12));
        // Equal area: plunge 30 -> r = sqrt(2) sin(30) = 0.7071.
        let p = project(Line {
            trend: 180.0,
            plunge: 30.0,
        });
        assert!(close(p[1], -std::f64::consts::FRAC_1_SQRT_2, 1e-5));
        for (t, pl) in [(10.0, 20.0), (200.0, 75.0), (300.0, 5.0)] {
            let back = unproject(project(Line { trend: t, plunge: pl })).unwrap();
            assert!(close(back.trend, t, 1e-9) && close(back.plunge, pl, 1e-9));
        }
    }

    #[test]
    fn planes_poles_and_intersections() {
        let p = Plane::new(60.0, 120.0);
        let pole = p.pole();
        assert!(close(pole.trend, 300.0, 1e-9) && close(pole.plunge, 30.0, 1e-9));
        let back = Plane::from_normal(p.normal()).unwrap();
        assert!(close(back.dip, 60.0, 1e-9) && close(back.dipdir, 120.0, 1e-9));
        // Two planes dipping 45 to E and W meet along a horizontal N-S line.
        let i = intersection(&Plane::new(45.0, 90.0), &Plane::new(45.0, 270.0)).unwrap();
        assert!(close(i.plunge, 0.0, 1e-9) && (close(i.trend, 0.0, 1e-9) || close(i.trend, 180.0, 1e-9)));
        // Symmetric planes about north: intersection plunges north.
        let i = intersection(&Plane::new(60.0, 30.0), &Plane::new(60.0, 330.0)).unwrap();
        assert!(az_diff(i.trend, 0.0) < 1e-9 && i.plunge > 0.0 && i.plunge < 60.0);
        assert!(intersection(&p, &p).is_none());
        // The great circle passes through the dip vector.
        let gc = Plane::new(40.0, 90.0).great_circle(180);
        let dipv = project(Line {
            trend: 90.0,
            plunge: 40.0,
        });
        assert!(gc
            .iter()
            .any(|q| close(q[0], dipv[0], 1e-9) && close(q[1], dipv[1], 1e-9)));
        assert!(gc.iter().all(|q| q[0].hypot(q[1]) <= 1.0 + 1e-9));
    }

    #[test]
    fn density_and_contours() {
        let poles: Vec<Vec3> = (0..100)
            .map(|k| {
                Plane::new(70.0 + (k % 5) as f64 * 0.5, 45.0 + (k % 7) as f64)
                    .pole()
                    .vector()
            })
            .collect();
        let c = Plane::new(71.0, 48.0).pole().vector();
        assert!(density(&poles, c) > 90.0);
        assert_eq!(density(&poles, Plane::new(10.0, 200.0).pole().vector()), 0.0);
        let g = density_grid(&poles, 41);
        assert!(g[0].is_nan() && !g[20 * 41 + 20].is_nan());
        assert!(!contour(&g, 41, 10.0).is_empty());
        // Antipodal poles near the rim count together.
        let rim = [
            Plane::new(89.0, 0.0).pole().vector(),
            Plane::new(89.0, 180.0).pole().vector(),
        ];
        assert_eq!(density(&rim, Plane::new(89.5, 0.0).pole().vector()), 100.0);
    }
}
