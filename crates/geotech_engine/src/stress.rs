//! In situ stresses from a depth gradient and their components in the plane
//! of a working's cross-section. MPa; compression positive.

use mining_core::{dot, Vec3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InSitu {
    /// Unit weight (kN/m3).
    pub gamma: f64,
    /// Depth below surface (m).
    pub depth: f64,
    /// sigma_H / sigma_v and sigma_h / sigma_v.
    pub k_h: f64,
    pub k_hmin: f64,
    /// Azimuth of sigma_H (degrees).
    pub az_h: f64,
}

impl InSitu {
    pub fn sigma_v(&self) -> f64 {
        self.gamma * self.depth / 1000.0
    }
    /// Tensor in world axes (x east, y north, z up).
    pub fn tensor(&self) -> [[f64; 3]; 3] {
        let sv = self.sigma_v();
        let (sh, shm) = (self.k_h * sv, self.k_hmin * sv);
        let a = self.az_h.to_radians();
        let hh = [a.sin(), a.cos(), 0.0];
        let hm = [a.cos(), -a.sin(), 0.0];
        let mut t = [[0.0; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                t[i][j] = sh * hh[i] * hh[j] + shm * hm[i] * hm[j];
            }
        }
        t[2][2] += sv;
        t
    }
}

fn normal_stress(t: &[[f64; 3]; 3], a: Vec3, b: Vec3) -> f64 {
    let mut s = 0.0;
    for i in 0..3 {
        for j in 0..3 {
            s += a[i] * t[i][j] * b[j];
        }
    }
    s
}

/// Stresses in the section plane of a horizontal working with axis `trend`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SectionStress {
    /// Horizontal (across the working) and vertical normal stresses, shear.
    pub sigma_x: f64,
    pub sigma_z: f64,
    pub tau_xz: f64,
    /// In-plane principal stresses and the angle of sigma_1 from horizontal
    /// (degrees, anticlockwise looking along the axis).
    pub sigma_1: f64,
    pub sigma_3: f64,
    pub angle: f64,
    /// Stress along the axis.
    pub sigma_axial: f64,
}

pub fn section(s: &InSitu, trend: f64) -> SectionStress {
    let t = s.tensor();
    let a = trend.to_radians();
    let axis = [a.sin(), a.cos(), 0.0];
    let across = [a.cos(), -a.sin(), 0.0];
    let up = [0.0, 0.0, 1.0];
    let (sx, sz, txz) = (
        normal_stress(&t, across, across),
        normal_stress(&t, up, up),
        normal_stress(&t, across, up),
    );
    let c = (sx + sz) / 2.0;
    let r = (((sx - sz) / 2.0).powi(2) + txz * txz).sqrt();
    let angle = 0.5 * (2.0 * txz).atan2(sx - sz);
    let _ = dot;
    SectionStress {
        sigma_x: sx,
        sigma_z: sz,
        tau_xz: txz,
        sigma_1: c + r,
        sigma_3: c - r,
        angle: angle.to_degrees(),
        sigma_axial: normal_stress(&t, axis, axis),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gradient_and_section_components() {
        let s = InSitu {
            gamma: 27.0,
            depth: 500.0,
            k_h: 2.0,
            k_hmin: 1.0,
            az_h: 90.0,
        };
        assert!((s.sigma_v() - 13.5).abs() < 1e-12);
        // A north-south working sees sigma_H (east-west) across it.
        let n = section(&s, 0.0);
        assert!((n.sigma_x - 27.0).abs() < 1e-9 && (n.sigma_z - 13.5).abs() < 1e-9 && n.tau_xz.abs() < 1e-9);
        assert!((n.sigma_1 - 27.0).abs() < 1e-9 && (n.sigma_3 - 13.5).abs() < 1e-9 && n.angle.abs() < 1e-9);
        assert!((n.sigma_axial - 13.5).abs() < 1e-9);
        // An east-west working sees sigma_h across and sigma_H along it.
        let e = section(&s, 90.0);
        assert!((e.sigma_x - 13.5).abs() < 1e-9 && (e.sigma_axial - 27.0).abs() < 1e-9);
        // At 45 degrees the horizontal across stress is the mean.
        assert!((section(&s, 45.0).sigma_x - 20.25).abs() < 1e-9);
    }
}
