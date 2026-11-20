//! Cell declustering: weights that remove the bias of preferential (dense)
//! drilling. Weights are averaged over several grid origins.

use std::collections::HashMap;

use crate::Sample;

/// Weights (mean 1) for one cell size, averaged over `origins` shifted grids.
pub fn cell_weights(samples: &[Sample], cell: [f64; 3], origins: usize) -> Vec<f64> {
    let n = samples.len();
    if n == 0 || cell.iter().any(|c| c.is_nan() || *c <= 0.0) {
        return vec![1.0; n];
    }
    let origins = origins.max(1);
    let mut w = vec![0.0; n];
    for o in 0..origins {
        let shift = o as f64 / origins as f64;
        let key = |s: &Sample| {
            (
                (s.xyz[0] / cell[0] + shift).floor() as i64,
                (s.xyz[1] / cell[1] + shift).floor() as i64,
                (s.xyz[2] / cell[2] + shift).floor() as i64,
            )
        };
        let mut count: HashMap<(i64, i64, i64), usize> = HashMap::new();
        for s in samples {
            *count.entry(key(s)).or_default() += 1;
        }
        let occupied = count.len() as f64;
        for (i, s) in samples.iter().enumerate() {
            w[i] += 1.0 / (occupied * count[&key(s)] as f64);
        }
    }
    let scale = n as f64 / w.iter().sum::<f64>();
    w.iter().map(|x| x * scale / 1.0).collect()
}

/// Weighted mean of the sample values using `weights`.
pub fn weighted_mean(samples: &[Sample], weights: &[f64]) -> f64 {
    let (mut sv, mut sw) = (0.0, 0.0);
    for (s, w) in samples.iter().zip(weights) {
        sv += s.value * w * s.length.max(1e-9);
        sw += w * s.length.max(1e-9);
    }
    if sw > 0.0 {
        sv / sw
    } else {
        f64::NAN
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeclusterResult {
    /// `(cell size, declustered mean)` for every size tried.
    pub table: Vec<(f64, f64)>,
    /// Size with the lowest (or highest) declustered mean.
    pub best_size: f64,
    pub weights: Vec<f64>,
}

/// Tries `sizes` (horizontal cell size; vertical = `z_ratio` x size) and
/// keeps the one minimising the mean (`minimise = true`: high grades are
/// over-sampled, the usual case) or maximising it.
pub fn decluster(samples: &[Sample], sizes: &[f64], z_ratio: f64, minimise: bool) -> DeclusterResult {
    let mut table = Vec::new();
    let mut best: Option<(f64, f64, Vec<f64>)> = None;
    for &s in sizes {
        let w = cell_weights(samples, [s, s, s * z_ratio.max(1e-3)], 4);
        let m = weighted_mean(samples, &w);
        table.push((s, m));
        let better = best
            .as_ref()
            .is_none_or(|(_, bm, _)| if minimise { m < *bm } else { m > *bm });
        if m.is_finite() && better {
            best = Some((s, m, w));
        }
    }
    match best {
        Some((s, _, w)) => DeclusterResult {
            table,
            best_size: s,
            weights: w,
        },
        None => DeclusterResult {
            table,
            best_size: 0.0,
            weights: vec![1.0; samples.len()],
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(x: f64, v: f64) -> Sample {
        Sample {
            xyz: [x, 0.0, 0.0],
            value: v,
            length: 1.0,
            hole: 0,
            domain: 1,
        }
    }

    #[test]
    fn dense_high_grade_cluster_is_downweighted() {
        // 20 samples of grade 10 packed in 0..2 m, 5 samples of grade 1 spread out.
        let mut data: Vec<Sample> = (0..20).map(|i| s(i as f64 * 0.1, 10.0)).collect();
        data.extend((0..5).map(|i| s(50.0 + i as f64 * 25.0, 1.0)));
        let naive = weighted_mean(&data, &vec![1.0; data.len()]);
        let d = decluster(&data, &[5.0, 10.0, 25.0], 1.0, true);
        let declustered = weighted_mean(&data, &d.weights);
        assert!(declustered < naive - 1.0, "{declustered} vs {naive}");
        assert!(
            (d.weights.iter().sum::<f64>() - data.len() as f64).abs() < 1e-9,
            "weights keep mean 1"
        );
        assert!(d.weights[0] < d.weights[24]);
        assert_eq!(d.table.len(), 3);
    }

    #[test]
    fn regular_grid_gets_equal_weights() {
        let data: Vec<Sample> = (0..10).map(|i| s(i as f64 * 20.0, i as f64)).collect();
        let w = cell_weights(&data, [10.0, 10.0, 10.0], 1);
        assert!(w.iter().all(|x| (x - 1.0).abs() < 1e-9));
    }
}
