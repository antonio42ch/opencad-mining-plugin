//! Kriging (ordinary, simple), inverse distance and nearest neighbour for
//! points and blocks (regular discretisation).

use mining_core::{add, dist, Vec3};

use crate::linalg::solve;
use crate::model::VariogramModel;
use crate::Sample;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Method {
    Ordinary,
    /// Simple kriging with a known mean.
    Simple {
        mean: f64,
    },
    Idw {
        power: f64,
    },
    Nearest,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Estimate {
    pub value: f64,
    /// Kriging variance (NaN for IDW / nearest neighbour).
    pub variance: f64,
    /// Kriging efficiency (NaN unless kriging).
    pub efficiency: f64,
    /// Slope of regression of the true on the estimated block grade.
    pub slope: f64,
    pub n: usize,
    /// Mean distance to the samples used.
    pub avg_dist: f64,
}

/// Offsets of a regular `nx x ny x nz` discretisation of a block of `size`
/// (cell centres), in the block's local axes `(ex, ey, ez)`.
pub fn discretization(size: [f64; 3], n: [usize; 3], axes: [Vec3; 3]) -> Vec<Vec3> {
    let n = [n[0].max(1), n[1].max(1), n[2].max(1)];
    let mut out = Vec::with_capacity(n[0] * n[1] * n[2]);
    for i in 0..n[0] {
        for j in 0..n[1] {
            for k in 0..n[2] {
                let l = [
                    ((i as f64 + 0.5) / n[0] as f64 - 0.5) * size[0],
                    ((j as f64 + 0.5) / n[1] as f64 - 0.5) * size[1],
                    ((k as f64 + 0.5) / n[2] as f64 - 0.5) * size[2],
                ];
                out.push([
                    l[0] * axes[0][0] + l[1] * axes[1][0] + l[2] * axes[2][0],
                    l[0] * axes[0][1] + l[1] * axes[1][1] + l[2] * axes[2][1],
                    l[0] * axes[0][2] + l[1] * axes[1][2] + l[2] * axes[2][2],
                ]);
            }
        }
    }
    out
}

/// Mean variogram between all pairs of discretisation nodes (`gamma(V, V)`).
pub fn gamma_vv(model: &VariogramModel, offsets: &[Vec3]) -> f64 {
    if offsets.len() < 2 {
        return 0.0;
    }
    let mut s = 0.0;
    for a in offsets {
        for b in offsets {
            s += model.gamma([a[0] - b[0], a[1] - b[1], a[2] - b[2]]);
        }
    }
    s / (offsets.len() * offsets.len()) as f64
}

/// Estimates the block (or point, with one zero offset) at `center` from
/// the `picks` (indices into `samples`).
pub fn estimate(
    method: Method,
    model: &VariogramModel,
    samples: &[Sample],
    picks: &[u32],
    center: Vec3,
    offsets: &[Vec3],
    gamma_vv: f64,
) -> Option<Estimate> {
    let n = picks.len();
    if n == 0 {
        return None;
    }
    let pt = |k: usize| samples[picks[k] as usize].xyz;
    let avg_dist = picks
        .iter()
        .map(|&i| dist(samples[i as usize].xyz, center))
        .sum::<f64>()
        / n as f64;
    match method {
        Method::Nearest => {
            let best = picks.iter().min_by(|a, b| {
                dist(samples[**a as usize].xyz, center).total_cmp(&dist(samples[**b as usize].xyz, center))
            })?;
            Some(Estimate {
                value: samples[*best as usize].value,
                variance: f64::NAN,
                efficiency: f64::NAN,
                slope: f64::NAN,
                n,
                avg_dist,
            })
        }
        Method::Idw { power } => {
            let (mut sw, mut sv) = (0.0, 0.0);
            for &i in picks {
                let d = dist(samples[i as usize].xyz, center).max(1e-6);
                let w = 1.0 / d.powf(power.max(0.0));
                sw += w;
                sv += w * samples[i as usize].value;
            }
            Some(Estimate {
                value: sv / sw,
                variance: f64::NAN,
                efficiency: f64::NAN,
                slope: f64::NAN,
                n,
                avg_dist,
            })
        }
        Method::Ordinary | Method::Simple { .. } => {
            let simple = matches!(method, Method::Simple { .. });
            let m = if let Method::Simple { mean } = method {
                mean
            } else {
                0.0
            };
            let total = model.total_sill();
            let jitter = 1e-9 * total.max(1e-12);
            // Kriging works with covariances for SK and variograms for OK.
            let f = |h: Vec3| if simple { model.cov(h) } else { model.gamma(h) };
            let dim = if simple { n } else { n + 1 };
            let mut a = vec![0.0; dim * dim];
            let mut b = vec![0.0; dim];
            for i in 0..n {
                for j in 0..n {
                    let (pi, pj) = (pt(i), pt(j));
                    a[i * dim + j] = f([pi[0] - pj[0], pi[1] - pj[1], pi[2] - pj[2]]);
                }
                a[i * dim + i] += jitter;
                let pi = pt(i);
                b[i] = offsets
                    .iter()
                    .map(|o| {
                        let u = add(center, *o);
                        f([pi[0] - u[0], pi[1] - u[1], pi[2] - u[2]])
                    })
                    .sum::<f64>()
                    / offsets.len().max(1) as f64;
                if !simple {
                    a[i * dim + n] = 1.0;
                    a[n * dim + i] = 1.0;
                }
            }
            if !simple {
                b[n] = 1.0;
            }
            let rhs = b.clone();
            solve(&mut a, &mut b, dim)?;
            let lam = &b[..n];
            let var_v = total - gamma_vv; // variance of the block values
            let (value, variance, mu) = if simple {
                let v = m
                    + (0..n)
                        .map(|i| lam[i] * (samples[picks[i] as usize].value - m))
                        .sum::<f64>();
                let s2 = var_v - (0..n).map(|i| lam[i] * rhs[i]).sum::<f64>();
                (v, s2, 0.0)
            } else {
                let v = (0..n).map(|i| lam[i] * samples[picks[i] as usize].value).sum::<f64>();
                let s2 = (0..n).map(|i| lam[i] * rhs[i]).sum::<f64>() + b[n] - gamma_vv;
                (v, s2, b[n])
            };
            let variance = variance.max(0.0);
            let efficiency = if var_v > 1e-12 {
                (var_v - variance) / var_v
            } else {
                f64::NAN
            };
            let slope = if var_v > 1e-12 {
                let g = var_v - variance;
                (g + mu.abs()) / (g + 2.0 * mu.abs()).max(1e-12)
            } else {
                f64::NAN
            };
            Some(Estimate {
                value,
                variance,
                efficiency,
                slope,
                n,
                avg_dist,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Kind, Orientation, Structure};

    fn model(nugget: f64) -> VariogramModel {
        VariogramModel {
            nugget,
            structures: vec![Structure {
                kind: Kind::Spherical,
                sill: 10.0,
                ranges: [100.0, 100.0, 100.0],
            }],
            orientation: Orientation::default(),
        }
    }

    fn smp(x: f64, y: f64, z: f64, v: f64) -> Sample {
        Sample {
            xyz: [x, y, z],
            value: v,
            length: 1.0,
            hole: 0,
            domain: 1,
        }
    }

    const ZERO: [Vec3; 1] = [[0.0; 3]];

    #[test]
    fn point_kriging_is_an_exact_interpolator() {
        let s = vec![
            smp(0.0, 0.0, 0.0, 3.5),
            smp(30.0, 0.0, 0.0, 9.0),
            smp(0.0, 40.0, 0.0, 1.0),
            smp(-20.0, -20.0, 5.0, 4.0),
        ];
        let picks: Vec<u32> = (0..4).collect();
        let e = estimate(Method::Ordinary, &model(1.0), &s, &picks, [0.0; 3], &ZERO, 0.0).unwrap();
        assert!((e.value - 3.5).abs() < 1e-6, "{e:?}");
        assert!(e.variance < 1e-6);
    }

    #[test]
    fn weights_sum_to_one_so_a_constant_field_is_reproduced() {
        let s = vec![
            smp(10.0, 0.0, 0.0, 7.0),
            smp(-30.0, 5.0, 0.0, 7.0),
            smp(0.0, 25.0, 10.0, 7.0),
            smp(5.0, -40.0, -3.0, 7.0),
            smp(60.0, 60.0, 0.0, 7.0),
        ];
        let picks: Vec<u32> = (0..5).collect();
        for m in [Method::Ordinary, Method::Idw { power: 2.0 }, Method::Nearest] {
            let e = estimate(m, &model(2.0), &s, &picks, [0.0; 3], &ZERO, 0.0).unwrap();
            assert!((e.value - 7.0).abs() < 1e-9, "{m:?} {e:?}");
        }
        // SK with the true mean also reproduces it (all residuals are zero).
        let e = estimate(
            Method::Simple { mean: 7.0 },
            &model(2.0),
            &s,
            &picks,
            [0.0; 3],
            &ZERO,
            0.0,
        )
        .unwrap();
        assert!((e.value - 7.0).abs() < 1e-9);
    }

    #[test]
    fn symmetric_configuration_gives_the_mean_and_closed_form_variance() {
        // Two samples at +-d on X, estimate at the origin, no nugget, range >> d.
        let d = 20.0;
        let m = model(0.0);
        let s = vec![smp(-d, 0.0, 0.0, 2.0), smp(d, 0.0, 0.0, 8.0)];
        let e = estimate(Method::Ordinary, &m, &s, &[0, 1], [0.0; 3], &ZERO, 0.0).unwrap();
        assert!((e.value - 5.0).abs() < 1e-9);
        // lambda = 1/2 each: sigma2 = sum(l * g(d)) + mu with mu from the system:
        // g(2d)*0.5 + mu = g(d)  =>  mu = g(d) - 0.5*g(2d); sigma2 = g(d) + mu.
        let (g1, g2) = (m.gamma([d, 0.0, 0.0]), m.gamma([2.0 * d, 0.0, 0.0]));
        let expected = g1 + (g1 - 0.5 * g2);
        assert!((e.variance - expected).abs() < 1e-6, "{} vs {expected}", e.variance);
    }

    #[test]
    fn block_variance_is_smaller_than_point_variance_and_efficiency_is_sane() {
        let m = model(1.0);
        let s = vec![
            smp(-30.0, -30.0, 0.0, 2.0),
            smp(30.0, -30.0, 0.0, 4.0),
            smp(-30.0, 30.0, 0.0, 6.0),
            smp(30.0, 30.0, 0.0, 8.0),
        ];
        let picks: Vec<u32> = (0..4).collect();
        let point = estimate(Method::Ordinary, &m, &s, &picks, [0.0; 3], &ZERO, 0.0).unwrap();
        let ax = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let offs = discretization([10.0, 10.0, 12.0], [4, 4, 4], ax);
        assert_eq!(offs.len(), 64);
        let gv = gamma_vv(&m, &offs);
        assert!(gv > 0.0 && gv < m.total_sill());
        let block = estimate(Method::Ordinary, &m, &s, &picks, [0.0; 3], &offs, gv).unwrap();
        assert!((block.value - point.value).abs() < 0.2);
        assert!(block.variance < point.variance, "{} {}", block.variance, point.variance);
        assert!(block.efficiency > 0.0 && block.efficiency <= 1.0);
    }

    /// One sample, simple kriging at distance d: lambda = C(d)/C(0), so the
    /// estimate is m + lambda (z - m) exactly. Also shows SK weights are not
    /// forced to sum to 1 (the estimate can pass the datum when m is wrong).
    #[test]
    fn simple_kriging_matches_the_one_sample_closed_form() {
        let m = model(1.0);
        let s = vec![smp(30.0, 0.0, 0.0, 6.0)];
        let e = estimate(Method::Simple { mean: 3.0 }, &m, &s, &[0], [0.0; 3], &ZERO, 0.0).unwrap();
        let lambda = m.cov([30.0, 0.0, 0.0]) / m.cov([0.0; 3]);
        // Tolerance covers the 1e-9 * sill diagonal regularisation (keeps
        // duplicate samples from making the system singular).
        assert!(
            (e.value - (3.0 + lambda * 3.0)).abs() < 1e-6,
            "{} vs {}",
            e.value,
            3.0 + lambda * 3.0
        );
        // Residual variance C(0) - lambda C(d).
        assert!((e.variance - (m.cov([0.0; 3]) - lambda * m.cov([30.0, 0.0, 0.0]))).abs() < 1e-6);
        // Mean equal to the datum: estimate = datum for any lambda.
        let same = estimate(Method::Simple { mean: 6.0 }, &m, &s, &[0], [0.0; 3], &ZERO, 0.0).unwrap();
        assert!((same.value - 6.0).abs() < 1e-12);
    }

    #[test]
    fn idw_and_nearest_pick_the_expected_values() {
        let s = vec![smp(1.0, 0.0, 0.0, 10.0), smp(10.0, 0.0, 0.0, 0.0)];
        let nn = estimate(Method::Nearest, &model(0.0), &s, &[0, 1], [0.0; 3], &ZERO, 0.0).unwrap();
        assert_eq!(nn.value, 10.0);
        let idw = estimate(
            Method::Idw { power: 1.0 },
            &model(0.0),
            &s,
            &[0, 1],
            [0.0; 3],
            &ZERO,
            0.0,
        )
        .unwrap();
        // weights 1 and 0.1 -> (10*1 + 0*0.1)/1.1
        assert!((idw.value - 10.0 / 1.1).abs() < 1e-9);
        assert!(idw.variance.is_nan());
    }

    #[test]
    fn duplicate_samples_do_not_break_the_system() {
        let s = vec![
            smp(5.0, 5.0, 0.0, 3.0),
            smp(5.0, 5.0, 0.0, 3.0),
            smp(40.0, 0.0, 0.0, 6.0),
        ];
        let e = estimate(Method::Ordinary, &model(0.5), &s, &[0, 1, 2], [0.0; 3], &ZERO, 0.0);
        assert!(e.is_some_and(|e| e.value.is_finite()));
        assert!(estimate(Method::Ordinary, &model(0.5), &s, &[], [0.0; 3], &ZERO, 0.0).is_none());
    }
}
