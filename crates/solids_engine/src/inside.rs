//! Point-in-solid tests for closed meshes, fast enough for millions of
//! queries (block models): a +Z ray with parity counting over triangles
//! indexed in a uniform XY grid.
//!
//! Faces are triangulated as fans, which is exact for convex or planar
//! faces (quads of excavation solids, cubes, triangulated wireframes).

use mining_core::Vec3;

use crate::cut::MeshData;

pub struct MeshIndex {
    tris: Vec<[Vec3; 3]>,
    lo: Vec3,
    hi: Vec3,
    cell: [f64; 2],
    n: [usize; 2],
    cells: Vec<Vec<u32>>,
    eps: [f64; 2],
}

impl MeshIndex {
    pub fn new(mesh: &MeshData) -> Self {
        let mut tris = Vec::new();
        for f in &mesh.faces {
            if f.iter().any(|&i| i >= mesh.vertices.len()) {
                continue;
            }
            for k in 1..f.len().saturating_sub(1) {
                tris.push([mesh.vertices[f[0]], mesh.vertices[f[k]], mesh.vertices[f[k + 1]]]);
            }
        }
        let (lo, hi) = mesh.bounds().unwrap_or(([0.0; 3], [0.0; 3]));
        let side = ((tris.len() as f64).sqrt().ceil() as usize).clamp(1, 256);
        let n = [side, side];
        let cell = [
            ((hi[0] - lo[0]) / side as f64).max(1e-12),
            ((hi[1] - lo[1]) / side as f64).max(1e-12),
        ];
        let mut cells = vec![Vec::new(); side * side];
        let idx = |v: f64, l: f64, c: f64| (((v - l) / c).floor() as isize).clamp(0, side as isize - 1) as usize;
        for (t, tri) in tris.iter().enumerate() {
            let (mut x0, mut x1, mut y0, mut y1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
            for p in tri {
                x0 = x0.min(p[0]);
                x1 = x1.max(p[0]);
                y0 = y0.min(p[1]);
                y1 = y1.max(p[1]);
            }
            for j in idx(y0, lo[1], cell[1])..=idx(y1, lo[1], cell[1]) {
                for i in idx(x0, lo[0], cell[0])..=idx(x1, lo[0], cell[0]) {
                    cells[j * side + i].push(t as u32);
                }
            }
        }
        let diag = ((hi[0] - lo[0]).hypot(hi[1] - lo[1])).max(1.0);
        MeshIndex {
            tris,
            lo,
            hi,
            cell,
            n,
            cells,
            eps: [diag * 1.37e-9, diag * 0.71e-9],
        }
    }

    pub fn bounds(&self) -> (Vec3, Vec3) {
        (self.lo, self.hi)
    }

    pub fn is_empty(&self) -> bool {
        self.tris.is_empty()
    }

    pub fn contains(&self, p: Vec3) -> bool {
        if self.tris.is_empty() || p[2] > self.hi[2] || p[2] < self.lo[2] {
            return false;
        }
        // Deterministic sub-micrometre shift: avoids ties on shared edges.
        let (x, y) = (p[0] + self.eps[0], p[1] + self.eps[1]);
        if x < self.lo[0] || x > self.hi[0] || y < self.lo[1] || y > self.hi[1] {
            return false;
        }
        let i = (((x - self.lo[0]) / self.cell[0]).floor() as isize).clamp(0, self.n[0] as isize - 1) as usize;
        let j = (((y - self.lo[1]) / self.cell[1]).floor() as isize).clamp(0, self.n[1] as isize - 1) as usize;
        let mut crossings = 0usize;
        for &t in &self.cells[j * self.n[0] + i] {
            let [a, b, c] = self.tris[t as usize];
            let d = (b[1] - c[1]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[1] - c[1]);
            if d.abs() < 1e-18 {
                continue; // vertical triangle
            }
            let l1 = ((b[1] - c[1]) * (x - c[0]) + (c[0] - b[0]) * (y - c[1])) / d;
            let l2 = ((c[1] - a[1]) * (x - c[0]) + (a[0] - c[0]) * (y - c[1])) / d;
            let l3 = 1.0 - l1 - l2;
            if l1 < 0.0 || l2 < 0.0 || l3 < 0.0 {
                continue;
            }
            if l1 * a[2] + l2 * b[2] + l3 * c[2] > p[2] {
                crossings += 1;
            }
        }
        crossings % 2 == 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axis::{axis_rings, AxisParams};
    use crate::mesh::ClosedMesh;
    use crate::section::Profile;

    fn gallery() -> MeshData {
        let axis = [[0.0, 0.0, 0.0], [0.0, 40.0, 0.0]];
        let rings = axis_rings(&AxisParams {
            axis: &axis,
            profile: &Profile::Rectangular {
                width: 4.0,
                height: 3.0,
            },
            interval: 5.0,
            offset: [0.0, 0.0],
        })
        .unwrap();
        let m = ClosedMesh::from_rings(&rings).unwrap();
        MeshData {
            vertices: m.vertices,
            faces: m.faces,
        }
    }

    #[test]
    fn inside_and_outside_a_gallery() {
        let idx = MeshIndex::new(&gallery());
        assert!(idx.contains([0.0, 20.0, 0.0]));
        assert!(idx.contains([1.9, 0.5, 1.4]));
        assert!(!idx.contains([2.1, 20.0, 0.0]));
        assert!(!idx.contains([0.0, 20.0, 1.6]));
        assert!(!idx.contains([0.0, -0.5, 0.0]));
        assert!(!idx.contains([0.0, 41.0, 0.0]));
    }

    #[test]
    fn volume_by_monte_carlo_grid_matches_exact_volume() {
        let mesh = gallery();
        let idx = MeshIndex::new(&mesh);
        let (lo, hi) = idx.bounds();
        let step = 0.25;
        let mut inside = 0usize;
        let (mut x, mut count) = (lo[0] + step / 2.0, 0usize);
        while x < hi[0] {
            let mut y = lo[1] + step / 2.0;
            while y < hi[1] {
                let mut z = lo[2] + step / 2.0;
                while z < hi[2] {
                    count += 1;
                    inside += usize::from(idx.contains([x, y, z]));
                    z += step;
                }
                y += step;
            }
            x += step;
        }
        let vol = inside as f64 * step.powi(3);
        assert!((vol - 480.0).abs() < 1.0, "{vol} of {count}");
    }

    #[test]
    fn concave_u_shaped_prism() {
        // U profile extruded 10 m along Y (two quads per side, fan-safe faces).
        let u = [
            [0.0, 0.0],
            [3.0, 0.0],
            [3.0, 2.0],
            [2.0, 2.0],
            [2.0, 1.0],
            [1.0, 1.0],
            [1.0, 2.0],
            [0.0, 2.0],
        ];
        let mut vertices = Vec::new();
        for y in [0.0, 10.0] {
            vertices.extend(u.iter().map(|p| [p[0], y, p[1]]));
        }
        // Triangulated U (convex pieces): bottom slab + two legs.
        let pieces: [[usize; 4]; 3] = [[0, 1, 4, 5], [0, 5, 6, 7], [1, 2, 3, 4]];
        let mut faces: Vec<Vec<usize>> = Vec::new();
        for q in pieces {
            faces.push(q.to_vec());
            faces.push(q.iter().rev().map(|i| i + 8).collect());
        }
        // Outer walls of the prism.
        for i in 0..8 {
            let j = (i + 1) % 8;
            faces.push(vec![i, j, j + 8, i + 8]);
        }
        let idx = MeshIndex::new(&MeshData { vertices, faces });
        assert!(idx.contains([0.5, 5.0, 1.5]));
        assert!(idx.contains([1.5, 5.0, 0.5]));
        assert!(!idx.contains([1.5, 5.0, 1.5]), "the notch of the U is outside");
    }
}
