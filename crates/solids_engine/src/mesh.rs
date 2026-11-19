//! Watertight mesh from rings: side quads + two n-gon caps, outward oriented.

use mining_core::{cross, dot, Vec3};

use crate::{Ring, SolidError};

#[derive(Debug, Clone, PartialEq)]
pub struct ClosedMesh {
    pub vertices: Vec<Vec3>,
    /// Polygon faces as vertex indices (quads for the sides, n-gons for caps).
    pub faces: Vec<Vec<usize>>,
}

impl ClosedMesh {
    pub fn from_rings(rings: &[Ring]) -> Result<Self, SolidError> {
        let n = rings.first().map_or(0, |r| r.len());
        if rings.len() < 2 {
            return Err(SolidError::TooFewVertices { what: "stations" });
        }
        if n < 3 || rings.iter().any(|r| r.len() != n) {
            return Err(SolidError::InvalidSection(
                "all sections need the same vertex count (>= 3)",
            ));
        }
        let vertices: Vec<Vec3> = rings.iter().flatten().copied().collect();
        let idx = |k: usize, j: usize| k * n + j % n;
        let mut faces = Vec::with_capacity((rings.len() - 1) * n + 2);
        for k in 0..rings.len() - 1 {
            for j in 0..n {
                faces.push(vec![idx(k, j), idx(k, j + 1), idx(k + 1, j + 1), idx(k + 1, j)]);
            }
        }
        faces.push((0..n).rev().map(|j| idx(0, j)).collect());
        let last = rings.len() - 1;
        faces.push((0..n).map(|j| idx(last, j)).collect());
        let mut m = ClosedMesh { vertices, faces };
        if m.volume() < 0.0 {
            m.faces.iter_mut().for_each(|f| f.reverse());
        }
        Ok(m)
    }

    /// Signed volume (positive when faces point outwards).
    pub fn volume(&self) -> f64 {
        self.faces
            .iter()
            .map(|f| {
                let a = self.vertices[f[0]];
                (1..f.len() - 1)
                    .map(|i| dot(a, cross(self.vertices[f[i]], self.vertices[f[i + 1]])))
                    .sum::<f64>()
            })
            .sum::<f64>()
            / 6.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orientation_is_outward_regardless_of_ring_order() {
        let sq = |z: f64| vec![[0.0, 0.0, z], [1.0, 0.0, z], [1.0, 1.0, z], [0.0, 1.0, z]];
        let a = ClosedMesh::from_rings(&[sq(0.0), sq(2.0)]).unwrap();
        let b = ClosedMesh::from_rings(&[sq(2.0), sq(0.0)]).unwrap();
        assert!((a.volume() - 2.0).abs() < 1e-12 && (b.volume() - 2.0).abs() < 1e-12);
    }

    #[test]
    fn mismatched_rings_rejected() {
        let r3 = vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let r4 = vec![[0.0; 3], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0], [0.0, 1.0, 0.0]];
        assert!(ClosedMesh::from_rings(&[r3, r4]).is_err());
    }
}
