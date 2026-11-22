//! The vein frame: origin, strike (`e1`, horizontal), up-dip (`e2`) and the
//! normal (`n`, towards the hanging wall). Local coordinates `(u, v, w)`.

use mining_core::{cross, dot, Vec3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VeinFrame {
    pub origin: Vec3,
    pub e1: Vec3,
    pub e2: Vec3,
    pub n: Vec3,
}

fn unit(v: Vec3) -> Vec3 {
    let l = dot(v, v).sqrt();
    [v[0] / l, v[1] / l, v[2] / l]
}

impl VeinFrame {
    /// Frame of a plane through `origin` with upward normal `n`.
    pub fn from_normal(origin: Vec3, n: Vec3) -> Option<VeinFrame> {
        let l = dot(n, n).sqrt();
        if l.is_nan() || l <= 1e-12 {
            return None;
        }
        let mut n = [n[0] / l, n[1] / l, n[2] / l];
        // The normal points up (to the hanging wall); a vertical vein faces east, or north.
        let flip = if n[2].abs() > 1e-9 {
            n[2] < 0.0
        } else if n[0].abs() > 1e-9 {
            n[0] < 0.0
        } else {
            n[1] < 0.0
        };
        if flip {
            n = [-n[0], -n[1], -n[2]];
        }
        let s = cross([0.0, 0.0, 1.0], n);
        let e1 = if dot(s, s) < 1e-18 { [1.0, 0.0, 0.0] } else { unit(s) };
        let e2 = cross(n, e1);
        Some(VeinFrame { origin, e1, e2, n })
    }

    /// Frame of a plane given by dip and dip direction (degrees).
    pub fn from_orientation(origin: Vec3, dip: f64, dipdir: f64) -> Option<VeinFrame> {
        let (d, a) = (dip.to_radians(), dipdir.to_radians());
        Self::from_normal(origin, [d.sin() * a.sin(), d.sin() * a.cos(), d.cos()])
    }

    pub fn to_local(&self, p: Vec3) -> Vec3 {
        let d = [p[0] - self.origin[0], p[1] - self.origin[1], p[2] - self.origin[2]];
        [dot(d, self.e1), dot(d, self.e2), dot(d, self.n)]
    }

    pub fn to_world(&self, l: Vec3) -> Vec3 {
        std::array::from_fn(|i| self.origin[i] + l[0] * self.e1[i] + l[1] * self.e2[i] + l[2] * self.n[i])
    }

    /// Dip (0..90) and dip direction of the plane.
    pub fn orientation(&self) -> (f64, f64) {
        let dip = self.n[2].clamp(-1.0, 1.0).acos().to_degrees();
        let dd = if dip < 1e-9 {
            0.0
        } else {
            self.n[0].atan2(self.n[1]).to_degrees().rem_euclid(360.0)
        };
        (dip, dd)
    }
}

/// Eigen-decomposition of a symmetric 3 x 3 matrix (cyclic Jacobi):
/// eigenvalues ascending with their unit eigenvectors.
pub fn eigen_sym(m: [[f64; 3]; 3]) -> [(f64, Vec3); 3] {
    let mut a = m;
    let mut v = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    for _ in 0..50 {
        let off = a[0][1].powi(2) + a[0][2].powi(2) + a[1][2].powi(2);
        if off < 1e-30 {
            break;
        }
        for (p, q) in [(0, 1), (0, 2), (1, 2)] {
            if a[p][q].abs() < 1e-300 {
                continue;
            }
            let theta = (a[q][q] - a[p][p]) / (2.0 * a[p][q]);
            let t = theta.signum() / (theta.abs() + (theta * theta + 1.0).sqrt());
            let t = if theta == 0.0 { 1.0 } else { t };
            let c = 1.0 / (t * t + 1.0).sqrt();
            let s = t * c;
            for row in a.iter_mut() {
                let (akp, akq) = (row[p], row[q]);
                row[p] = c * akp - s * akq;
                row[q] = s * akp + c * akq;
            }
            for k in 0..3 {
                let (apk, aqk) = (a[p][k], a[q][k]);
                a[p][k] = c * apk - s * aqk;
                a[q][k] = s * apk + c * aqk;
            }
            for row in v.iter_mut() {
                let (vkp, vkq) = (row[p], row[q]);
                row[p] = c * vkp - s * vkq;
                row[q] = s * vkp + c * vkq;
            }
        }
    }
    let mut out: [(f64, Vec3); 3] = std::array::from_fn(|i| (a[i][i], [v[0][i], v[1][i], v[2][i]]));
    out.sort_by(|x, y| x.0.total_cmp(&y.0));
    out
}

/// Best-fit plane of points (least squares): its frame at the centroid.
/// `None` with fewer than three points or when they are collinear.
pub fn fit(points: &[Vec3]) -> Option<VeinFrame> {
    if points.len() < 3 {
        return None;
    }
    let n = points.len() as f64;
    let c: Vec3 = std::array::from_fn(|i| points.iter().map(|p| p[i]).sum::<f64>() / n);
    let mut m = [[0.0; 3]; 3];
    for p in points {
        let d = [p[0] - c[0], p[1] - c[1], p[2] - c[2]];
        for i in 0..3 {
            for j in 0..3 {
                m[i][j] += d[i] * d[j];
            }
        }
    }
    let e = eigen_sym(m);
    // Collinear: two eigenvalues ~ 0.
    if e[1].0 <= 1e-9 * e[2].0.max(1e-12) {
        return None;
    }
    VeinFrame::from_normal(c, e[0].1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_axes_and_round_trip() {
        // Vein dipping 60 east: strike north, up-dip towards the west and up.
        let f = VeinFrame::from_orientation([0.0; 3], 60.0, 90.0).unwrap();
        assert!((f.e1[1] - 1.0).abs() < 1e-12 && f.e2[0] < 0.0 && f.e2[2] > 0.0 && f.n[2] > 0.0);
        let (dip, dd) = f.orientation();
        assert!((dip - 60.0).abs() < 1e-9 && (dd - 90.0).abs() < 1e-9);
        let p = [3.0, -2.0, 5.0];
        let back = f.to_world(f.to_local(p));
        assert!((0..3).all(|i| (back[i] - p[i]).abs() < 1e-12));
        // Vertical vein striking north: the normal faces east.
        let v = VeinFrame::from_orientation([0.0; 3], 90.0, 270.0).unwrap();
        assert!((v.n[0] - 1.0).abs() < 1e-12 && v.e2[2] > 0.999);
        // Horizontal vein.
        let h = VeinFrame::from_orientation([0.0; 3], 0.0, 0.0).unwrap();
        assert_eq!(h.e1, [1.0, 0.0, 0.0]);
    }

    #[test]
    fn plane_fit_recovers_the_orientation() {
        let f0 = VeinFrame::from_orientation([500.0, 800.0, 3000.0], 72.0, 115.0).unwrap();
        let pts: Vec<Vec3> = (0..60)
            .map(|k| {
                let (u, v) = ((k % 10) as f64 * 7.0 - 30.0, (k / 10) as f64 * 9.0 - 20.0);
                let w = ((k * 37) % 11) as f64 * 0.01 - 0.05; // a few cm of scatter
                f0.to_world([u, v, w])
            })
            .collect();
        let f = fit(&pts).unwrap();
        let (dip, dd) = f.orientation();
        assert!((dip - 72.0).abs() < 0.2 && (dd - 115.0).abs() < 0.2, "{dip} {dd}");
        assert!(fit(&pts[..2]).is_none());
        assert!(
            fit(&[[0.0; 3], [1.0, 1.0, 1.0], [2.0, 2.0, 2.0]]).is_none(),
            "collinear"
        );
    }

    #[test]
    fn jacobi_eigenvalues() {
        let e = eigen_sym([[2.0, 1.0, 0.0], [1.0, 2.0, 0.0], [0.0, 0.0, 5.0]]);
        let vals: Vec<f64> = e.iter().map(|x| x.0).collect();
        assert!((vals[0] - 1.0).abs() < 1e-12 && (vals[1] - 3.0).abs() < 1e-12 && (vals[2] - 5.0).abs() < 1e-12);
        let v = e[0].1;
        assert!((v[0].abs() - v[1].abs()).abs() < 1e-12 && v[2].abs() < 1e-12);
    }
}
