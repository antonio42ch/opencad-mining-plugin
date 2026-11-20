//! Domain regions and the assignment of blocks to domains.

use geostat_engine::kriging::discretization;
use geostat_engine::par_collect;
use geostat_engine::surface::GridSurface;
use mining_core::Vec3;
use solids_engine::cut::MeshData;
use solids_engine::inside::MeshIndex;

use crate::model::BlockModel;

pub trait Region: Sync + Send {
    fn contains(&self, p: Vec3) -> bool;
    /// World bounding box, when finite.
    fn bounds(&self) -> Option<(Vec3, Vec3)> {
        None
    }
}

/// Union of closed meshes (a domain can be several solids).
pub struct SolidsRegion {
    parts: Vec<MeshIndex>,
}

impl SolidsRegion {
    pub fn new(meshes: &[MeshData]) -> Self {
        SolidsRegion {
            parts: meshes.iter().map(MeshIndex::new).collect(),
        }
    }
}

impl Region for SolidsRegion {
    fn contains(&self, p: Vec3) -> bool {
        self.parts.iter().any(|m| m.contains(p))
    }
    fn bounds(&self) -> Option<(Vec3, Vec3)> {
        self.parts
            .iter()
            .filter(|m| !m.is_empty())
            .map(|m| m.bounds())
            .reduce(|(a, b), (c, d)| {
                (
                    [a[0].min(c[0]), a[1].min(c[1]), a[2].min(c[2])],
                    [b[0].max(d[0]), b[1].max(d[1]), b[2].max(d[2])],
                )
            })
    }
}

/// Above or below a contact surface (false where the surface has no value).
pub struct SurfaceSide {
    pub surface: GridSurface,
    pub above: bool,
}

impl Region for SurfaceSide {
    fn contains(&self, p: Vec3) -> bool {
        let z = self.surface.value_at(p[0], p[1]);
        z.is_finite() && if self.above { p[2] >= z } else { p[2] < z }
    }
}

/// Intersection of regions.
pub struct And(pub Vec<Box<dyn Region>>);

impl Region for And {
    fn contains(&self, p: Vec3) -> bool {
        self.0.iter().all(|r| r.contains(p))
    }
    fn bounds(&self) -> Option<(Vec3, Vec3)> {
        // Intersection of the boxes.
        self.0.iter().filter_map(|r| r.bounds()).reduce(|(a, b), (c, d)| {
            (
                [a[0].max(c[0]), a[1].max(c[1]), a[2].max(c[2])],
                [b[0].min(d[0]), b[1].min(d[1]), b[2].min(d[2])],
            )
        })
    }
}

pub struct Everything;

impl Region for Everything {
    fn contains(&self, _: Vec3) -> bool {
        true
    }
}

/// Assigns every block to the **first** region containing most of its
/// nodes (domains listed earlier win overlaps). `regions[d]` is the region of
/// domain id `d + 1`. Returns the number of assigned blocks.
pub fn assign_domains(model: &mut BlockModel, regions: &[&dyn Region], disc: [usize; 3]) -> usize {
    let g = model.geometry;
    let ax = g.axes();
    let offsets = discretization(g.size, disc, ax);
    let total = offsets.len() as f32;
    let first = |p: Vec3| regions.iter().position(|r| r.contains(p));
    // A block can be skipped only when its box misses the bounding box of
    // every region (all bounded). Agreement of centre and corners is not
    // enough: a solid smaller than the block, or a notch, hides inside it.
    let boxes: Option<Vec<(Vec3, Vec3)>> = regions.iter().map(|r| r.bounds()).collect();
    let rows: Vec<(u8, f32)> = par_collect(g.len(), |idx| {
        let c = g.center(idx);
        if let Some(boxes) = &boxes {
            let corners = g.corners(idx);
            let lo: Vec3 = std::array::from_fn(|a| corners.iter().map(|p| p[a]).fold(f64::MAX, f64::min));
            let hi: Vec3 = std::array::from_fn(|a| corners.iter().map(|p| p[a]).fold(f64::MIN, f64::max));
            if boxes
                .iter()
                .all(|(bl, bh)| (0..3).any(|a| hi[a] < bl[a] || lo[a] > bh[a]))
            {
                return (0, 0.0);
            }
        }
        let mut count = vec![0usize; regions.len()];
        for o in &offsets {
            let p = [c[0] + o[0], c[1] + o[1], c[2] + o[2]];
            if let Some(d) = first(p) {
                count[d] += 1;
            }
        }
        match (0..regions.len()).max_by_key(|&d| (count[d], std::cmp::Reverse(d))) {
            Some(d) if count[d] > 0 => (d as u8 + 1, count[d] as f32 / total),
            _ => (0, 0.0),
        }
    });
    let mut assigned = 0;
    for (idx, (d, f)) in rows.into_iter().enumerate() {
        model.domain[idx] = d;
        model.fraction[idx] = f;
        assigned += usize::from(d != 0);
    }
    assigned
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::BlockGeometry;
    use solids_engine::axis::{axis_rings, AxisParams};
    use solids_engine::mesh::ClosedMesh;
    use solids_engine::section::Profile;

    fn model() -> BlockModel {
        BlockModel::new(BlockGeometry {
            origin: [-10.0, 0.0, -6.0],
            size: [2.0, 5.0, 3.0],
            n: [10, 10, 4],
            rotation: 0.0,
        })
    }

    fn gallery_region() -> SolidsRegion {
        // 4 wide (x -2..2), 3 high (z -1.5..1.5), y 0..50.
        let rings = axis_rings(&AxisParams {
            axis: &[[0.0, 0.0, 0.0], [0.0, 50.0, 0.0]],
            profile: &Profile::Rectangular {
                width: 4.0,
                height: 3.0,
            },
            interval: 5.0,
            offset: [0.0, 0.0],
        })
        .unwrap();
        let m = ClosedMesh::from_rings(&rings).unwrap();
        SolidsRegion::new(&[MeshData {
            vertices: m.vertices,
            faces: m.faces,
        }])
    }

    #[test]
    fn solid_domain_gives_the_exact_inside_volume_with_partial_blocks() {
        let mut m = model();
        let r = gallery_region();
        let n = assign_domains(&mut m, &[&r], [4, 4, 4]);
        assert!(n > 0 && n < m.len());
        let vol: f64 = (0..m.len()).map(|i| m.inside_volume(i)).sum();
        // Exact volume 4*3*50 = 600; blocks are 2x5x3 so edges are partial: z -1.5..1.5
        // cuts the 3 m layers (z -3..0, 0..3) at their middle -> discretisation gives it exactly.
        assert!((vol - 600.0).abs() < 25.0, "{vol}");
        assert!(m.fraction.iter().all(|f| (0.0..=1.0).contains(f)));
        assert!(m.domain.iter().zip(&m.fraction).all(|(d, f)| (*d == 0) == (*f == 0.0)));
    }

    /// A solid smaller than one block (centre and corners all outside it)
    /// must still give the block its share.
    #[test]
    fn solid_smaller_than_a_block_is_not_missed() {
        let mut m = BlockModel::new(BlockGeometry {
            origin: [-30.0, -30.0, -30.0],
            size: [60.0, 60.0, 60.0],
            n: [1, 1, 1],
            rotation: 0.0,
        });
        let rings = axis_rings(&AxisParams {
            axis: &[[10.0, -10.0, 10.0], [10.0, 10.0, 10.0]],
            profile: &Profile::Rectangular {
                width: 8.0,
                height: 8.0,
            },
            interval: 5.0,
            offset: [0.0, 0.0],
        })
        .unwrap();
        let mesh = ClosedMesh::from_rings(&rings).unwrap();
        let r = SolidsRegion::new(&[MeshData {
            vertices: mesh.vertices,
            faces: mesh.faces,
        }]);
        assign_domains(&mut m, &[&r], [10, 10, 10]);
        assert_eq!(m.domain[0], 1);
        // 8 x 8 x 20 = 1280 m3 of a 216 000 m3 block.
        assert!((m.fraction[0] - 1280.0 / 216_000.0).abs() < 0.004, "{}", m.fraction[0]);
    }

    #[test]
    fn surface_side_and_priority_between_overlapping_domains() {
        let surf = GridSurface {
            x0: -20.0,
            y0: -10.0,
            dx: 20.0,
            dy: 30.0,
            nx: 3,
            ny: 4,
            z: vec![-3.0; 12],
        };
        let above = SurfaceSide {
            surface: surf.clone(),
            above: true,
        };
        let below = SurfaceSide {
            surface: surf,
            above: false,
        };
        let mut m = model();
        assign_domains(&mut m, &[&above, &below], [2, 2, 2]);
        // Plane at z = -3: blocks of the layer k=1 (z -3..0) are above, k=0 (-6..-3) below.
        let idx_low = m.geometry.index(5, 5, 0);
        let idx_up = m.geometry.index(5, 5, 3);
        assert_eq!(m.domain[idx_low], 2);
        assert_eq!(m.domain[idx_up], 1);
        // Everything-region listed first wins every overlap.
        let mut m2 = model();
        assign_domains(&mut m2, &[&Everything, &above], [2, 2, 2]);
        assert!(m2.domain.iter().all(|&d| d == 1));
    }
}
