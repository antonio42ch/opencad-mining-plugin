//! Spatial index and ellipsoidal neighbourhood search with octant and
//! per-hole limits.

use std::collections::HashMap;

use mining_core::{dot, sub, Vec3};

use crate::model::Orientation;
use crate::Sample;

#[derive(Debug, Clone, PartialEq)]
pub struct Neighborhood {
    /// Radii along `[major, semi-major, minor]`.
    pub radii: [f64; 3],
    pub orientation: Orientation,
    pub min_samples: usize,
    pub max_samples: usize,
    /// 0 = unlimited.
    pub max_per_octant: usize,
    /// 0 = unlimited.
    pub max_per_hole: usize,
}

impl Neighborhood {
    /// Same neighbourhood with every radius multiplied by `m` (search passes).
    pub fn scaled(&self, m: f64) -> Neighborhood {
        Neighborhood {
            radii: [self.radii[0] * m, self.radii[1] * m, self.radii[2] * m],
            ..self.clone()
        }
    }
}

pub struct SearchIndex {
    cell: f64,
    cells: HashMap<(i32, i32, i32), Vec<u32>>,
}

impl SearchIndex {
    pub fn new(samples: &[Sample], cell: f64) -> Self {
        let cell = if cell.is_finite() && cell > 0.0 { cell } else { 1.0 };
        let mut cells: HashMap<(i32, i32, i32), Vec<u32>> = HashMap::new();
        for (i, s) in samples.iter().enumerate() {
            cells.entry(Self::key(s.xyz, cell)).or_default().push(i as u32);
        }
        SearchIndex { cell, cells }
    }

    fn key(p: Vec3, cell: f64) -> (i32, i32, i32) {
        (
            (p[0] / cell).floor() as i32,
            (p[1] / cell).floor() as i32,
            (p[2] / cell).floor() as i32,
        )
    }

    /// Indices of the samples selected for `center`, nearest first (by
    /// anisotropic distance), honouring the limits. `exclude` skips one
    /// sample (leave-one-out). Returns `(index, scaled distance)` pairs.
    pub fn query(
        &self,
        samples: &[Sample],
        center: Vec3,
        nb: &Neighborhood,
        exclude: Option<u32>,
        out: &mut Vec<(u32, f64)>,
    ) {
        self.query_filtered(samples, center, nb, &|i| Some(i) == exclude, out)
    }

    /// Like [`query`](Self::query) but `skip` removes candidates **before**
    /// the octant, per-hole and total limits are applied (so excluded samples
    /// never use up the quota).
    pub fn query_filtered(
        &self,
        samples: &[Sample],
        center: Vec3,
        nb: &Neighborhood,
        skip: &dyn Fn(u32) -> bool,
        out: &mut Vec<(u32, f64)>,
    ) {
        out.clear();
        let ax = nb.orientation.axes();
        // Half extents of the ellipsoid's bounding box.
        let half: [f64; 3] =
            std::array::from_fn(|i| (0..3).map(|k| (nb.radii[k] * ax[k][i]).powi(2)).sum::<f64>().sqrt());
        let lo = Self::key(
            [center[0] - half[0], center[1] - half[1], center[2] - half[2]],
            self.cell,
        );
        let hi = Self::key(
            [center[0] + half[0], center[1] + half[1], center[2] + half[2]],
            self.cell,
        );
        let mut cand: Vec<(f64, u32, u8)> = Vec::new();
        for i in lo.0..=hi.0 {
            for j in lo.1..=hi.1 {
                for k in lo.2..=hi.2 {
                    let Some(list) = self.cells.get(&(i, j, k)) else {
                        continue;
                    };
                    for &idx in list {
                        if skip(idx) {
                            continue;
                        }
                        let h = sub(samples[idx as usize].xyz, center);
                        let l = [dot(h, ax[0]), dot(h, ax[1]), dot(h, ax[2])];
                        let d2: f64 = (0..3).map(|a| (l[a] / nb.radii[a].max(1e-9)).powi(2)).sum();
                        if d2 <= 1.0 {
                            let oct =
                                u8::from(l[0] >= 0.0) | (u8::from(l[1] >= 0.0) << 1) | (u8::from(l[2] >= 0.0) << 2);
                            cand.push((d2.sqrt(), idx, oct));
                        }
                    }
                }
            }
        }
        cand.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        let mut per_oct = [0usize; 8];
        let mut per_hole: HashMap<u32, usize> = HashMap::new();
        for (d, idx, oct) in cand {
            if out.len() >= nb.max_samples {
                break;
            }
            if nb.max_per_octant > 0 && per_oct[oct as usize] >= nb.max_per_octant {
                continue;
            }
            let hole = samples[idx as usize].hole;
            if nb.max_per_hole > 0 && per_hole.get(&hole).copied().unwrap_or(0) >= nb.max_per_hole {
                continue;
            }
            per_oct[oct as usize] += 1;
            *per_hole.entry(hole).or_default() += 1;
            out.push((idx, d));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid() -> Vec<Sample> {
        let mut v = Vec::new();
        for i in -5..=5 {
            for j in -5..=5 {
                v.push(Sample {
                    xyz: [i as f64 * 10.0, j as f64 * 10.0, 0.0],
                    value: 1.0,
                    length: 1.0,
                    hole: (i + 5) as u32 * 11 + (j + 5) as u32,
                    domain: 1,
                });
            }
        }
        v
    }

    fn nb() -> Neighborhood {
        Neighborhood {
            radii: [35.0, 35.0, 35.0],
            orientation: Orientation::default(),
            min_samples: 1,
            max_samples: 100,
            max_per_octant: 0,
            max_per_hole: 0,
        }
    }

    #[test]
    fn ellipsoid_respects_radii_and_orientation() {
        let s = grid();
        let idx = SearchIndex::new(&s, 20.0);
        let mut out = Vec::new();
        idx.query(&s, [0.0, 0.0, 0.0], &nb(), None, &mut out);
        // Sphere r = 35 on a 10 m grid in the z = 0 plane: all points with i²+j² <= 12.25 -> 37.
        assert_eq!(out.len(), 37);
        assert_eq!(out[0].1, 0.0);
        assert!(out.windows(2).all(|w| w[0].1 <= w[1].1));
        // Long east-west ellipsoid (major axis east) catches more E-W neighbours than N-S.
        let el = Neighborhood {
            radii: [45.0, 15.0, 15.0],
            orientation: Orientation {
                azimuth: 90.0,
                dip: 0.0,
                rake: 0.0,
            },
            ..nb()
        };
        idx.query(&s, [0.0, 0.0, 0.0], &el, None, &mut out);
        let max_x = out.iter().map(|(i, _)| s[*i as usize].xyz[0].abs()).fold(0.0, f64::max);
        let max_y = out.iter().map(|(i, _)| s[*i as usize].xyz[1].abs()).fold(0.0, f64::max);
        assert_eq!((max_x, max_y), (40.0, 10.0));
    }

    #[test]
    fn octant_hole_and_total_limits() {
        let s = grid();
        let idx = SearchIndex::new(&s, 20.0);
        let mut out = Vec::new();
        // One per octant in a flat layout: z = 0 points fall in 4 octants (l_z >= 0 for all).
        idx.query(
            &s,
            [0.5, 0.5, 0.0],
            &Neighborhood {
                max_per_octant: 1,
                ..nb()
            },
            None,
            &mut out,
        );
        assert!(out.len() <= 4 && out.len() >= 3, "{}", out.len());
        idx.query(
            &s,
            [0.0, 0.0, 0.0],
            &Neighborhood { max_samples: 8, ..nb() },
            None,
            &mut out,
        );
        assert_eq!(out.len(), 8);
        // Same hole id for a whole row: limit 2 per hole.
        let mut rows = s.clone();
        rows.iter_mut().for_each(|p| p.hole = p.xyz[1] as u32);
        let idx2 = SearchIndex::new(&rows, 20.0);
        idx2.query(
            &rows,
            [0.0, 0.0, 0.0],
            &Neighborhood {
                max_per_hole: 2,
                ..nb()
            },
            None,
            &mut out,
        );
        let mut per: HashMap<u32, usize> = HashMap::new();
        out.iter()
            .for_each(|(i, _)| *per.entry(rows[*i as usize].hole).or_default() += 1);
        assert!(per.values().all(|&c| c <= 2));
        // Leave-one-out.
        idx.query(&s, [0.0, 0.0, 0.0], &nb(), Some(60), &mut out);
        assert!(out.iter().all(|(i, _)| *i != 60));
    }
}
