//! Regular block geometry with a plan rotation.
//!
//! Local axes: `ex` (columns, i), `ey` (rows, j), `ez` (up, k). With
//! `rotation = 0`, `ex` = east and `ey` = north. `rotation` is the azimuth
//! (clockwise from north) of `ey`, so the model can follow the strike.

use mining_core::{add, scale, Vec3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlockGeometry {
    /// World position of the lower corner of block (0, 0, 0).
    pub origin: Vec3,
    pub size: [f64; 3],
    pub n: [usize; 3],
    /// Degrees.
    pub rotation: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GeometryError {
    InvalidSize,
    TooManyBlocks(usize),
    Empty,
}

impl std::fmt::Display for GeometryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSize => write!(f, "block size must be > 0"),
            Self::TooManyBlocks(n) => write!(f, "the model would have {n} blocks; the limit is {MAX_BLOCKS}"),
            Self::Empty => write!(f, "the extent is empty"),
        }
    }
}
impl std::error::Error for GeometryError {}

pub const MAX_BLOCKS: usize = 20_000_000;

impl BlockGeometry {
    pub fn axes(&self) -> [Vec3; 3] {
        let r = self.rotation.to_radians();
        [[r.cos(), -r.sin(), 0.0], [r.sin(), r.cos(), 0.0], [0.0, 0.0, 1.0]]
    }

    pub fn len(&self) -> usize {
        self.n[0] * self.n[1] * self.n[2]
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn volume(&self) -> f64 {
        self.size[0] * self.size[1] * self.size[2]
    }

    pub fn index(&self, i: usize, j: usize, k: usize) -> usize {
        (k * self.n[1] + j) * self.n[0] + i
    }

    pub fn ijk(&self, idx: usize) -> [usize; 3] {
        let i = idx % self.n[0];
        let j = (idx / self.n[0]) % self.n[1];
        [i, j, idx / (self.n[0] * self.n[1])]
    }

    /// World position of a local point `(x, y, z)` measured from the origin.
    pub fn local_to_world(&self, l: Vec3) -> Vec3 {
        let ax = self.axes();
        add(
            self.origin,
            add(add(scale(ax[0], l[0]), scale(ax[1], l[1])), scale(ax[2], l[2])),
        )
    }

    pub fn world_to_local(&self, p: Vec3) -> Vec3 {
        let ax = self.axes();
        let d = [p[0] - self.origin[0], p[1] - self.origin[1], p[2] - self.origin[2]];
        [
            mining_core::dot(d, ax[0]),
            mining_core::dot(d, ax[1]),
            mining_core::dot(d, ax[2]),
        ]
    }

    pub fn center(&self, idx: usize) -> Vec3 {
        let [i, j, k] = self.ijk(idx);
        self.local_to_world([
            (i as f64 + 0.5) * self.size[0],
            (j as f64 + 0.5) * self.size[1],
            (k as f64 + 0.5) * self.size[2],
        ])
    }

    /// The 8 corners: bottom (x-,y-), (x+,y-), (x+,y+), (x-,y+), then top.
    pub fn corners(&self, idx: usize) -> [Vec3; 8] {
        let [i, j, k] = self.ijk(idx);
        let (x0, y0, z0) = (
            i as f64 * self.size[0],
            j as f64 * self.size[1],
            k as f64 * self.size[2],
        );
        let (x1, y1, z1) = (x0 + self.size[0], y0 + self.size[1], z0 + self.size[2]);
        [
            (x0, y0, z0),
            (x1, y0, z0),
            (x1, y1, z0),
            (x0, y1, z0),
            (x0, y0, z1),
            (x1, y0, z1),
            (x1, y1, z1),
            (x0, y1, z1),
        ]
        .map(|(x, y, z)| self.local_to_world([x, y, z]))
    }

    /// Smallest geometry (same size and rotation) covering the world box
    /// `[lo, hi]` expanded by `pad` metres on every side.
    pub fn covering(
        lo: Vec3,
        hi: Vec3,
        size: [f64; 3],
        rotation: f64,
        pad: f64,
    ) -> Result<BlockGeometry, GeometryError> {
        if size.iter().any(|s| !(s.is_finite() && *s > 0.0)) {
            return Err(GeometryError::InvalidSize);
        }
        if hi[0] < lo[0] || hi[1] < lo[1] || hi[2] < lo[2] {
            return Err(GeometryError::Empty);
        }
        let probe = BlockGeometry {
            origin: [0.0; 3],
            size,
            n: [1, 1, 1],
            rotation,
        };
        let mut loc: Vec<Vec3> = Vec::with_capacity(8);
        for x in [lo[0], hi[0]] {
            for y in [lo[1], hi[1]] {
                for z in [lo[2], hi[2]] {
                    loc.push(probe.world_to_local([x, y, z]));
                }
            }
        }
        let min = |a: usize| loc.iter().map(|p| p[a]).fold(f64::MAX, f64::min) - pad;
        let max = |a: usize| loc.iter().map(|p| p[a]).fold(f64::MIN, f64::max) + pad;
        // Snap the lower corner to a multiple of the block size so models built
        // from different extents align.
        let snap = |a: usize| (min(a) / size[a]).floor() * size[a];
        let l0 = [snap(0), snap(1), snap(2)];
        let n = [0, 1, 2].map(|a| (((max(a) - l0[a]) / size[a]).ceil() as usize).max(1));
        let total = n[0].saturating_mul(n[1]).saturating_mul(n[2]);
        if total > MAX_BLOCKS {
            return Err(GeometryError::TooManyBlocks(total));
        }
        Ok(BlockGeometry {
            origin: probe.local_to_world(l0),
            size,
            n,
            rotation,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_roundtrip_and_unrotated_positions() {
        let g = BlockGeometry {
            origin: [1000.0, 2000.0, 100.0],
            size: [10.0, 10.0, 12.0],
            n: [4, 3, 2],
            rotation: 0.0,
        };
        assert_eq!(g.len(), 24);
        for idx in 0..g.len() {
            let [i, j, k] = g.ijk(idx);
            assert_eq!(g.index(i, j, k), idx);
        }
        assert_eq!(g.center(0), [1005.0, 2005.0, 106.0]);
        assert_eq!(g.center(g.index(3, 2, 1)), [1035.0, 2025.0, 118.0]);
        assert!((g.corners(0).iter().map(|c| c[2]).fold(f64::MAX, f64::min) - 100.0).abs() < 1e-9);
    }

    #[test]
    fn rotation_turns_the_model_and_roundtrips() {
        let g = BlockGeometry {
            origin: [0.0; 3],
            size: [10.0, 20.0, 5.0],
            n: [2, 2, 1],
            rotation: 90.0,
        };
        // ey now points east: the row direction advances along +X.
        let c0 = g.center(0);
        let c1 = g.center(g.index(0, 1, 0));
        assert!(
            (c1[0] - c0[0] - 20.0).abs() < 1e-9 && (c1[1] - c0[1]).abs() < 1e-9,
            "{c0:?} {c1:?}"
        );
        let p = [33.0, -7.0, 2.0];
        let back = g.local_to_world(g.world_to_local(p));
        assert!((0..3).all(|i| (back[i] - p[i]).abs() < 1e-9));
    }

    #[test]
    fn covering_contains_the_box_and_snaps() {
        let g = BlockGeometry::covering(
            [96_440.0, 87_550.0, 1_710.0],
            [97_070.0, 88_120.0, 2_250.0],
            [10.0, 10.0, 12.0],
            0.0,
            0.0,
        )
        .unwrap();
        assert_eq!(g.origin, [96_440.0, 87_550.0, 1_704.0]);
        assert_eq!(g.n, [63, 57, 46]);
        assert!(g.origin[0] + g.n[0] as f64 * 10.0 >= 97_070.0);
        let rotated = BlockGeometry::covering([0.0; 3], [100.0, 100.0, 10.0], [10.0, 10.0, 5.0], 45.0, 0.0).unwrap();
        for c in [[0.0, 0.0, 0.0], [100.0, 100.0, 10.0], [100.0, 0.0, 5.0]] {
            let l = rotated.world_to_local(c);
            assert!((0..3).all(|a| l[a] >= -1e-9 && l[a] <= rotated.n[a] as f64 * rotated.size[a] + 1e-9));
        }
        assert!(matches!(
            BlockGeometry::covering([0.0; 3], [1e6; 3], [1.0; 3], 0.0, 0.0),
            Err(GeometryError::TooManyBlocks(_))
        ));
        assert_eq!(
            BlockGeometry::covering([0.0; 3], [1.0; 3], [0.0, 1.0, 1.0], 0.0, 0.0),
            Err(GeometryError::InvalidSize)
        );
    }
}
