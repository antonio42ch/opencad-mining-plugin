//! 2D contact surfaces (e.g. oxide/sulphide) estimated by ordinary kriging
//! of the contact elevation, and bilinear lookup for domain assignment.

use mining_core::Vec3;

use crate::kriging::{estimate, Method};
use crate::model::{Kind, Orientation, Structure, VariogramModel};
use crate::par_collect;
use crate::search::{Neighborhood, SearchIndex};
use crate::Sample;

#[derive(Debug, Clone, PartialEq)]
pub struct GridSurface {
    pub x0: f64,
    pub y0: f64,
    pub dx: f64,
    pub dy: f64,
    pub nx: usize,
    pub ny: usize,
    /// Row-major `[j * nx + i]`, NaN = no estimate.
    pub z: Vec<f64>,
}

impl GridSurface {
    pub fn node(&self, i: usize, j: usize) -> Vec3 {
        [
            self.x0 + i as f64 * self.dx,
            self.y0 + j as f64 * self.dy,
            self.z[j * self.nx + i],
        ]
    }

    /// Bilinear elevation at `(x, y)`; NaN outside the grid or next to a hole.
    pub fn value_at(&self, x: f64, y: f64) -> f64 {
        let (fx, fy) = ((x - self.x0) / self.dx, (y - self.y0) / self.dy);
        if fx < 0.0 || fy < 0.0 || fx > (self.nx - 1) as f64 || fy > (self.ny - 1) as f64 {
            return f64::NAN;
        }
        let (i, j) = (
            (fx.floor() as usize).min(self.nx.saturating_sub(2)),
            (fy.floor() as usize).min(self.ny.saturating_sub(2)),
        );
        let (tx, ty) = (fx - i as f64, fy - j as f64);
        let g = |a: usize, b: usize| self.z[b * self.nx + a];
        let (i1, j1) = ((i + 1).min(self.nx - 1), (j + 1).min(self.ny - 1));
        g(i, j) * (1.0 - tx) * (1.0 - ty)
            + g(i1, j) * tx * (1.0 - ty)
            + g(i, j1) * (1.0 - tx) * ty
            + g(i1, j1) * tx * ty
    }
}

/// 2D variogram parameters of the contact elevation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceParams {
    pub kind: Kind,
    pub nugget: f64,
    pub sill: f64,
    pub range: f64,
    pub radius: f64,
    pub min_samples: usize,
    pub max_samples: usize,
}

/// Regular grid of the surface: `nx x ny` nodes from `(x0, y0)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridSpec {
    pub x0: f64,
    pub y0: f64,
    pub dx: f64,
    pub dy: f64,
    pub nx: usize,
    pub ny: usize,
}

/// Estimates the surface on `grid` from contact points `(x, y, z)`. Nodes
/// without enough points stay NaN.
pub fn krige_surface(points: &[Vec3], grid: &GridSpec, p: &SurfaceParams) -> GridSurface {
    let GridSpec { x0, y0, dx, dy, nx, ny } = *grid;
    let samples: Vec<Sample> = points
        .iter()
        .enumerate()
        .map(|(i, q)| Sample {
            xyz: [q[0], q[1], 0.0],
            value: q[2],
            length: 1.0,
            hole: i as u32,
            domain: 1,
        })
        .collect();
    let model = VariogramModel {
        nugget: p.nugget,
        structures: vec![Structure {
            kind: p.kind,
            sill: p.sill,
            ranges: [p.range, p.range, p.range.max(1.0)],
        }],
        orientation: Orientation::default(),
    };
    let nb = Neighborhood {
        // All samples lie at z = 0: a tiny vertical radius keeps the search 2D
        // (a huge one walked thousands of empty index cells per node).
        radii: [p.radius, p.radius, 1.0],
        orientation: Orientation::default(),
        min_samples: p.min_samples.max(1),
        max_samples: p.max_samples.max(1),
        max_per_octant: 0,
        max_per_hole: 0,
    };
    let index = SearchIndex::new(&samples, p.radius.max(1.0));
    let z = par_collect(nx * ny, |k| {
        let (i, j) = (k % nx, k / nx);
        let c = [x0 + i as f64 * dx, y0 + j as f64 * dy, 0.0];
        let mut picks = Vec::new();
        index.query(&samples, c, &nb, None, &mut picks);
        let list: Vec<u32> = picks.iter().map(|p| p.0).collect();
        if list.len() < nb.min_samples {
            return f64::NAN;
        }
        estimate(Method::Ordinary, &model, &samples, &list, c, &[[0.0; 3]], 0.0).map_or(f64::NAN, |e| e.value)
    });
    GridSurface {
        x0,
        y0,
        dx,
        dy,
        nx,
        ny,
        z,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> SurfaceParams {
        SurfaceParams {
            kind: Kind::Spherical,
            nugget: 0.0,
            sill: 100.0,
            range: 400.0,
            radius: 150.0,
            min_samples: 3,
            max_samples: 12,
        }
    }

    #[test]
    fn recovers_an_inclined_plane_inside_the_data() {
        // z = 2000 - 0.1 x + 0.05 y sampled at scattered "holes".
        let plane = |x: f64, y: f64| 2000.0 - 0.1 * x + 0.05 * y;
        let mut pts = Vec::new();
        for i in 0..9 {
            for j in 0..9 {
                let (x, y) = (
                    i as f64 * 50.0 + (j % 3) as f64 * 7.0,
                    j as f64 * 50.0 + (i % 3) as f64 * 5.0,
                );
                pts.push([x, y, plane(x, y)]);
            }
        }
        let s = krige_surface(
            &pts,
            &GridSpec {
                x0: 100.0,
                y0: 100.0,
                dx: 25.0,
                dy: 25.0,
                nx: 13,
                ny: 13,
            },
            &params(),
        );
        let mut max_err: f64 = 0.0;
        for j in 0..13 {
            for i in 0..13 {
                let n = s.node(i, j);
                assert!(n[2].is_finite());
                max_err = max_err.max((n[2] - plane(n[0], n[1])).abs());
            }
        }
        assert!(max_err < 1.0, "{max_err}");
        assert!((s.value_at(150.0, 150.0) - plane(150.0, 150.0)).abs() < 1.0);
    }

    #[test]
    fn nodes_without_data_are_nan_and_lookup_outside_is_nan() {
        let pts = vec![
            [0.0, 0.0, 10.0],
            [10.0, 0.0, 10.0],
            [0.0, 10.0, 10.0],
            [10.0, 10.0, 10.0],
        ];
        let s = krige_surface(
            &pts,
            &GridSpec {
                x0: 0.0,
                y0: 0.0,
                dx: 400.0,
                dy: 400.0,
                nx: 3,
                ny: 3,
            },
            &params(),
        );
        assert!(s.z[0].is_finite());
        assert!(s.z[8].is_nan());
        assert!(s.value_at(-5.0, 0.0).is_nan());
    }
}
