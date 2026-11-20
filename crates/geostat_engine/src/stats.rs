//! Descriptive statistics, histograms, quantiles, Q-Q pairs and
//! grade-tonnage curves of sample sets (optionally weighted).

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Summary {
    pub n: usize,
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub variance: f64,
    pub std_dev: f64,
    pub cv: f64,
    pub p10: f64,
    pub p25: f64,
    pub median: f64,
    pub p75: f64,
    pub p90: f64,
    pub p95: f64,
    pub p99: f64,
}

/// Weighted quantile (`q` in [0, 1]) of `(value, weight)` pairs.
pub fn weighted_quantile(sorted: &[(f64, f64)], total_weight: f64, q: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    let target = q.clamp(0.0, 1.0) * total_weight;
    let mut acc = 0.0;
    for (i, &(v, w)) in sorted.iter().enumerate() {
        acc += w;
        if acc >= target {
            // Interpolate between this value and the next at the boundary.
            return match sorted.get(i + 1) {
                Some(&(nv, _)) if acc - target < 1e-12 && q < 1.0 => 0.5 * (v + nv),
                _ => v,
            };
        }
    }
    sorted.last().unwrap().0
}

/// Weighted summary; weights default to 1 when `weights` is `None`.
pub fn summarize(values: &[f64], weights: Option<&[f64]>) -> Summary {
    let mut pairs: Vec<(f64, f64)> = values
        .iter()
        .enumerate()
        .filter(|(_, v)| v.is_finite())
        .map(|(i, &v)| (v, weights.map_or(1.0, |w| w[i])))
        .filter(|&(_, w)| w > 0.0)
        .collect();
    if pairs.is_empty() {
        return Summary::default();
    }
    pairs.sort_by(|a, b| a.0.total_cmp(&b.0));
    let wsum: f64 = pairs.iter().map(|p| p.1).sum();
    let mean = pairs.iter().map(|p| p.0 * p.1).sum::<f64>() / wsum;
    let variance = pairs.iter().map(|p| p.1 * (p.0 - mean).powi(2)).sum::<f64>() / wsum;
    let q = |p| weighted_quantile(&pairs, wsum, p);
    Summary {
        n: pairs.len(),
        min: pairs[0].0,
        max: pairs.last().unwrap().0,
        mean,
        variance,
        std_dev: variance.sqrt(),
        cv: if mean.abs() > 1e-300 {
            variance.sqrt() / mean
        } else {
            f64::NAN
        },
        p10: q(0.10),
        p25: q(0.25),
        median: q(0.5),
        p75: q(0.75),
        p90: q(0.90),
        p95: q(0.95),
        p99: q(0.99),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Histogram {
    pub edges: Vec<f64>,
    /// Weight fraction per bin (sums to 1).
    pub freq: Vec<f64>,
}

pub fn histogram(values: &[f64], weights: Option<&[f64]>, bins: usize, lo: f64, hi: f64) -> Histogram {
    let bins = bins.max(1);
    let hi = if hi > lo { hi } else { lo + 1.0 };
    let w = (hi - lo) / bins as f64;
    let mut freq = vec![0.0; bins];
    let mut total = 0.0;
    for (i, &v) in values.iter().enumerate() {
        if !v.is_finite() {
            continue;
        }
        let wt = weights.map_or(1.0, |w| w[i]);
        let b = (((v - lo) / w).floor() as isize).clamp(0, bins as isize - 1) as usize;
        freq[b] += wt;
        total += wt;
    }
    if total > 0.0 {
        freq.iter_mut().for_each(|f| *f /= total);
    }
    Histogram {
        edges: (0..=bins).map(|i| lo + i as f64 * w).collect(),
        freq,
    }
}

/// Q-Q pairs at `n` evenly spaced probabilities (inner 1 %..99 %).
pub fn qq(a: &[f64], wa: Option<&[f64]>, b: &[f64], wb: Option<&[f64]>, n: usize) -> Vec<(f64, f64)> {
    let prep = |v: &[f64], w: Option<&[f64]>| {
        let mut p: Vec<(f64, f64)> = v
            .iter()
            .enumerate()
            .filter(|(_, x)| x.is_finite())
            .map(|(i, &x)| (x, w.map_or(1.0, |w| w[i])))
            .collect();
        p.sort_by(|x, y| x.0.total_cmp(&y.0));
        let t = p.iter().map(|x| x.1).sum::<f64>();
        (p, t)
    };
    let (pa, ta) = prep(a, wa);
    let (pb, tb) = prep(b, wb);
    (0..n)
        .map(|i| {
            let q = 0.01 + 0.98 * i as f64 / (n.max(2) - 1) as f64;
            (weighted_quantile(&pa, ta, q), weighted_quantile(&pb, tb, q))
        })
        .collect()
}

/// One row of a grade-tonnage curve.
#[derive(Debug, Clone, PartialEq)]
pub struct GradeTonnage {
    pub cutoff: f64,
    /// Weight (tonnes, or fraction when weights are normalised) above cutoff.
    pub weight: f64,
    pub grade: f64,
    pub metal: f64,
}

/// Grade-tonnage of `(value, weight)` data at each cut-off (>=).
pub fn grade_tonnage(values: &[f64], weights: &[f64], cutoffs: &[f64]) -> Vec<GradeTonnage> {
    cutoffs
        .iter()
        .map(|&c| {
            let (mut w, mut m) = (0.0, 0.0);
            for (v, wt) in values.iter().zip(weights) {
                if v.is_finite() && *v >= c && *wt > 0.0 {
                    w += wt;
                    m += wt * v;
                }
            }
            GradeTonnage {
                cutoff: c,
                weight: w,
                grade: if w > 0.0 { m / w } else { 0.0 },
                metal: m,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_and_weights() {
        let s = summarize(&[1.0, 2.0, 3.0, 4.0, f64::NAN], None);
        assert_eq!((s.n, s.min, s.max, s.mean), (4, 1.0, 4.0, 2.5));
        assert!((s.variance - 1.25).abs() < 1e-12);
        // A heavy weight on the last value pulls the mean.
        let w = summarize(&[1.0, 2.0, 3.0, 4.0], Some(&[1.0, 1.0, 1.0, 7.0]));
        assert!((w.mean - 3.4).abs() < 1e-12);
        assert_eq!(summarize(&[], None), Summary::default());
    }

    #[test]
    fn histogram_sums_to_one_and_clamps() {
        let h = histogram(&[0.0, 0.5, 1.0, 9.0, 100.0], None, 2, 0.0, 10.0);
        assert!((h.freq.iter().sum::<f64>() - 1.0).abs() < 1e-12);
        assert_eq!(h.freq, vec![0.6, 0.4]);
    }

    #[test]
    fn qq_of_identical_sets_is_the_identity() {
        let v: Vec<f64> = (0..100).map(|i| i as f64).collect();
        for (a, b) in qq(&v, None, &v, None, 20) {
            assert!((a - b).abs() < 1e-12);
        }
    }

    #[test]
    fn grade_tonnage_rows() {
        let g = grade_tonnage(&[1.0, 2.0, 4.0], &[10.0, 10.0, 10.0], &[0.0, 2.0, 5.0]);
        assert_eq!((g[0].weight, g[0].grade), (30.0, 7.0 / 3.0));
        assert_eq!((g[1].weight, g[1].grade), (20.0, 3.0));
        assert_eq!((g[2].weight, g[2].grade), (0.0, 0.0));
    }
}
