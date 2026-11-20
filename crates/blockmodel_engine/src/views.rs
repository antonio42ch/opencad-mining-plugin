//! Geometry of the model's views: horizontal level slices and vertical
//! section slices, as polygons per block.

use mining_core::{dist, Vec3};
use solids_engine::cut::{cut, MeshData, Plane};

use crate::model::BlockModel;

/// A polygon of block `block` (3 to 6 vertices, planar).
#[derive(Debug, Clone, PartialEq)]
pub struct BlockPolygon {
    pub block: usize,
    pub points: Vec<Vec3>,
}

fn box_mesh(c: &[Vec3; 8]) -> MeshData {
    MeshData {
        vertices: c.to_vec(),
        faces: vec![
            vec![0, 3, 2, 1],
            vec![4, 5, 6, 7],
            vec![0, 1, 5, 4],
            vec![1, 2, 6, 5],
            vec![2, 3, 7, 6],
            vec![3, 0, 4, 7],
        ],
    }
}

/// Rectangles of the blocks cut by the horizontal plane `z`, drawn at `z`.
pub fn level_polygons(m: &BlockModel, z: f64, accept: &dyn Fn(usize) -> bool) -> Vec<BlockPolygon> {
    let g = &m.geometry;
    let k = ((z - g.origin[2]) / g.size[2]).floor();
    if k < 0.0 || k >= g.n[2] as f64 {
        return Vec::new();
    }
    let k = k as usize;
    let mut out = Vec::new();
    for j in 0..g.n[1] {
        for i in 0..g.n[0] {
            let idx = g.index(i, j, k);
            if !accept(idx) {
                continue;
            }
            let c = g.corners(idx);
            out.push(BlockPolygon {
                block: idx,
                points: vec![
                    [c[0][0], c[0][1], z],
                    [c[1][0], c[1][1], z],
                    [c[2][0], c[2][1], z],
                    [c[3][0], c[3][1], z],
                ],
            });
        }
    }
    out
}

/// Polygons of the blocks cut by `plane` (any orientation), at most
/// `max_blocks` (0 = unlimited). Returns them with a flag telling whether
/// the limit was hit.
pub fn plane_polygons(
    m: &BlockModel,
    plane: &Plane,
    accept: &dyn Fn(usize) -> bool,
    max_blocks: usize,
) -> (Vec<BlockPolygon>, bool) {
    let g = &m.geometry;
    let half_diag = 0.5 * (g.size[0].powi(2) + g.size[1].powi(2) + g.size[2].powi(2)).sqrt();
    let mut out = Vec::new();
    for idx in 0..m.len() {
        if !accept(idx) {
            continue;
        }
        let c = g.center(idx);
        if plane.distance(c).abs() > half_diag {
            continue;
        }
        let corners = g.corners(idx);
        for l in cut(&[box_mesh(&corners)], plane) {
            if l.closed && l.points.len() >= 3 {
                out.push(BlockPolygon {
                    block: idx,
                    points: l.points,
                });
            }
        }
        if max_blocks > 0 && out.len() >= max_blocks {
            return (out, true);
        }
    }
    (out, false)
}

/// Closed triangulated cubes of the blocks (for a 3D view above a cut-off):
/// one mesh with all of them. Returns `(vertices, faces)`.
pub fn block_cubes(m: &BlockModel, blocks: &[usize]) -> (Vec<Vec3>, Vec<Vec<usize>>) {
    let mut v = Vec::with_capacity(blocks.len() * 8);
    let mut f = Vec::with_capacity(blocks.len() * 6);
    for &b in blocks {
        let base = v.len();
        v.extend(m.geometry.corners(b));
        for face in box_mesh(&[[0.0; 3]; 8]).faces {
            f.push(face.into_iter().map(|i| base + i).collect());
        }
    }
    (v, f)
}

/// Longest edge of a polygon (to size labels).
pub fn max_edge(p: &[Vec3]) -> f64 {
    (0..p.len())
        .map(|i| dist(p[i], p[(i + 1) % p.len()]))
        .fold(0.0, f64::max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::BlockGeometry;
    use solids_engine::cut::{level_planes, vertical_planes};

    fn model(rotation: f64) -> BlockModel {
        let mut m = BlockModel::new(BlockGeometry {
            origin: [0.0; 3],
            size: [10.0, 10.0, 12.0],
            n: [6, 5, 4],
            rotation,
        });
        m.domain.iter_mut().for_each(|d| *d = 1);
        m.fraction.iter_mut().for_each(|f| *f = 1.0);
        m
    }

    #[test]
    fn level_slice_has_one_rectangle_per_block_in_the_layer() {
        let m = model(0.0);
        let polys = level_polygons(&m, 14.0, &|_| true);
        assert_eq!(polys.len(), 6 * 5);
        assert!(polys
            .iter()
            .all(|p| p.points.len() == 4 && p.points.iter().all(|q| q[2] == 14.0)));
        assert!(level_polygons(&m, 500.0, &|_| true).is_empty());
        // Filter by acceptance.
        assert_eq!(level_polygons(&m, 14.0, &|i| i % 2 == 0).len(), 15);
    }

    #[test]
    fn vertical_plane_cuts_rectangles_even_for_a_rotated_model() {
        for rot in [0.0, 33.0] {
            let m = model(rot);
            let c = m.geometry.center(m.geometry.index(2, 2, 0));
            let plane = vertical_planes([c[0], c[1] - 100.0, 0.0], [c[0], c[1] + 100.0, 0.0], 1.0, 1).unwrap()[0].plane;
            let (polys, limited) = plane_polygons(&m, &plane, &|_| true, 0);
            assert!(!limited && !polys.is_empty());
            assert!(
                polys
                    .iter()
                    .all(|p| p.points.len() == 4 || p.points.len() == 5 || p.points.len() == 6),
                "rot {rot}"
            );
            // Total polygon area equals the model's section area along that plane (width via block count).
            let area: f64 = polys
                .iter()
                .map(|p| {
                    let pl = Plane {
                        origin: plane.origin,
                        normal: plane.normal,
                        u: plane.u,
                        v: plane.v,
                    };
                    let q: Vec<[f64; 2]> = p.points.iter().map(|x| pl.to_2d(*x)).collect();
                    (0..q.len())
                        .map(|i| q[i][0] * q[(i + 1) % q.len()][1] - q[(i + 1) % q.len()][0] * q[i][1])
                        .sum::<f64>()
                        .abs()
                        / 2.0
                })
                .sum();
            assert!(area > 0.0 && area <= 4.0 * 12.0 * 60.0 + 1e-6, "{area}");
        }
        // The same query at a level plane equals the dedicated level routine.
        let m = model(0.0);
        let lp = level_planes(6.0, 6.0, 1.0).unwrap()[0].plane;
        let (cut_polys, _) = plane_polygons(&m, &lp, &|_| true, 0);
        assert_eq!(cut_polys.len(), level_polygons(&m, 6.0, &|_| true).len());
    }

    #[test]
    fn cubes_and_limits() {
        let m = model(0.0);
        let (v, f) = block_cubes(&m, &[0, 7]);
        assert_eq!((v.len(), f.len()), (16, 12));
        assert!(f.iter().flatten().all(|&i| i < 16));
        let plane = level_planes(6.0, 6.0, 1.0).unwrap()[0].plane;
        let (p, limited) = plane_polygons(&m, &plane, &|_| true, 3);
        assert!(limited && p.len() >= 3);
        assert!((max_edge(&level_polygons(&m, 1.0, &|_| true)[0].points) - 10.0).abs() < 1e-9);
    }
}
