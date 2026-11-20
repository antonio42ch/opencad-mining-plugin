//! Leave-one-out cross-validation of a kriging plan.

use crate::kriging::{estimate, Method};
use crate::model::VariogramModel;
use crate::par_collect;
use crate::search::{Neighborhood, SearchIndex};
use crate::Sample;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CvRecord {
    pub index: usize,
    pub estimate: f64,
    pub truth: f64,
    pub variance: f64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CvStats {
    pub n: usize,
    /// Mean error (estimate - truth): bias.
    pub mean_error: f64,
    pub mse: f64,
    pub correlation: f64,
    /// Mean and variance of the standardised error (ideal 0 and 1).
    pub z_mean: f64,
    pub z_var: f64,
    pub mean_variance: f64,
}

pub struct CrossVal {
    pub records: Vec<CvRecord>,
    pub stats: CvStats,
}

/// Re-estimates every `step`-th sample from the others. `exclude_same_hole`
/// also hides the other composites of the same drillhole, which mimics
/// estimating at a new hole (the honest test).
pub fn cross_validate(
    samples: &[Sample],
    model: &VariogramModel,
    nb: &Neighborhood,
    method: Method,
    step: usize,
    exclude_same_hole: bool,
) -> CrossVal {
    let cell = (nb.radii[0] * nb.radii[1] * nb.radii[2]).cbrt().max(1.0);
    let index = SearchIndex::new(samples, cell);
    let ids: Vec<usize> = (0..samples.len()).step_by(step.max(1)).collect();
    let raw = par_collect(ids.len(), |k| {
        let i = ids[k];
        let mut picks = Vec::new();
        let hole = samples[i].hole;
        let skip = |j: u32| j as usize == i || (exclude_same_hole && samples[j as usize].hole == hole);
        index.query_filtered(samples, samples[i].xyz, nb, &skip, &mut picks);
        let list: Vec<u32> = picks.iter().map(|p| p.0).collect();
        if list.len() < nb.min_samples.max(1) {
            return None;
        }
        estimate(method, model, samples, &list, samples[i].xyz, &[[0.0; 3]], 0.0).map(|e| CvRecord {
            index: i,
            estimate: e.value,
            truth: samples[i].value,
            variance: e.variance,
        })
    });
    let records: Vec<CvRecord> = raw.into_iter().flatten().collect();
    let n = records.len();
    if n < 2 {
        return CrossVal {
            records,
            stats: CvStats {
                n,
                ..Default::default()
            },
        };
    }
    let nf = n as f64;
    let (me, mse) = (
        records.iter().map(|r| r.estimate - r.truth).sum::<f64>() / nf,
        records.iter().map(|r| (r.estimate - r.truth).powi(2)).sum::<f64>() / nf,
    );
    let (mx, my) = (
        records.iter().map(|r| r.estimate).sum::<f64>() / nf,
        records.iter().map(|r| r.truth).sum::<f64>() / nf,
    );
    let (mut sxy, mut sxx, mut syy) = (0.0, 0.0, 0.0);
    for r in &records {
        sxy += (r.estimate - mx) * (r.truth - my);
        sxx += (r.estimate - mx).powi(2);
        syy += (r.truth - my).powi(2);
    }
    let zs: Vec<f64> = records
        .iter()
        .filter(|r| r.variance.is_finite() && r.variance > 1e-12)
        .map(|r| (r.estimate - r.truth) / r.variance.sqrt())
        .collect();
    let zn = zs.len().max(1) as f64;
    let zm = zs.iter().sum::<f64>() / zn;
    let stats = CvStats {
        n,
        mean_error: me,
        mse,
        correlation: if sxx > 0.0 && syy > 0.0 {
            sxy / (sxx * syy).sqrt()
        } else {
            f64::NAN
        },
        z_mean: zm,
        z_var: zs.iter().map(|z| (z - zm).powi(2)).sum::<f64>() / zn,
        mean_variance: records
            .iter()
            .filter(|r| r.variance.is_finite())
            .map(|r| r.variance)
            .sum::<f64>()
            / nf,
    };
    CrossVal { records, stats }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Kind, Orientation, Structure};

    /// Smooth deterministic field: kriging with the right model must predict
    /// left-out samples well and with near-zero bias.
    fn field() -> Vec<Sample> {
        let mut v = Vec::new();
        for i in 0..14 {
            for j in 0..14 {
                let (x, y) = (i as f64 * 15.0, j as f64 * 15.0);
                v.push(Sample {
                    xyz: [x, y, 0.0],
                    value: 5.0 + 2.0 * (x / 60.0).sin() + 1.5 * (y / 80.0).cos(),
                    length: 1.0,
                    hole: (i * 14 + j) as u32,
                    domain: 1,
                });
            }
        }
        v
    }

    #[test]
    fn smooth_field_is_recovered_with_small_error() {
        let m = VariogramModel {
            nugget: 0.01,
            structures: vec![Structure {
                kind: Kind::Gaussian,
                sill: 3.0,
                ranges: [150.0, 150.0, 150.0],
            }],
            orientation: Orientation::default(),
        };
        let nb = Neighborhood {
            radii: [60.0, 60.0, 60.0],
            orientation: Orientation::default(),
            min_samples: 4,
            max_samples: 12,
            max_per_octant: 0,
            max_per_hole: 0,
        };
        let cv = cross_validate(&field(), &m, &nb, Method::Ordinary, 3, true);
        assert!(cv.stats.n > 40);
        assert!(cv.stats.mean_error.abs() < 0.1, "{:?}", cv.stats);
        assert!(cv.stats.mse < 0.1, "{:?}", cv.stats);
        assert!(cv.stats.correlation > 0.95, "{:?}", cv.stats);
    }

    /// Excluding the composite's own hole must not starve the neighbourhood:
    /// the nearest picks are usually from that hole, so they are skipped
    /// before the limits, not after.
    #[test]
    fn same_hole_exclusion_keeps_full_neighbourhoods() {
        let mut s = Vec::new();
        for h in 0..25u32 {
            for k in 0..30 {
                let (x, y) = ((h % 5) as f64 * 50.0, (h / 5) as f64 * 50.0);
                s.push(Sample {
                    xyz: [x, y, -(k as f64) * 4.0],
                    value: (x / 80.0).sin() + (k as f64 / 10.0).cos(),
                    length: 4.0,
                    hole: h,
                    domain: 1,
                });
            }
        }
        let m = VariogramModel {
            nugget: 0.1,
            structures: vec![Structure {
                kind: Kind::Spherical,
                sill: 1.0,
                ranges: [200.0, 200.0, 60.0],
            }],
            orientation: Orientation::default(),
        };
        let nb = Neighborhood {
            radii: [200.0, 200.0, 60.0],
            orientation: Orientation::default(),
            min_samples: 4,
            max_samples: 8,
            max_per_octant: 0,
            max_per_hole: 0,
        };
        let cv = cross_validate(&s, &m, &nb, Method::Ordinary, 1, true);
        assert_eq!(cv.stats.n, s.len(), "every composite has neighbours in other holes");
    }

    #[test]
    fn too_few_neighbours_yields_no_records() {
        let m = VariogramModel {
            nugget: 1.0,
            structures: vec![],
            orientation: Orientation::default(),
        };
        let nb = Neighborhood {
            radii: [5.0, 5.0, 5.0],
            orientation: Orientation::default(),
            min_samples: 4,
            max_samples: 8,
            max_per_octant: 0,
            max_per_hole: 0,
        };
        assert_eq!(cross_validate(&field(), &m, &nb, Method::Ordinary, 1, false).stats.n, 0);
    }
}
