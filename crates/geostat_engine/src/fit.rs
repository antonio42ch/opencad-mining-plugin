//! Automatic fit of a variogram model to experimental variograms
//! (weighted least squares, weights = pairs / gamma_model², Cressie 1985),
//! optimised with Nelder-Mead.

use crate::model::{Kind, Orientation, Structure, VariogramModel};
use crate::vario::ExpVariogram;

#[derive(Debug, Clone)]
pub struct FitConfig {
    /// Kind of every structure (1..=3 structures).
    pub kinds: Vec<Kind>,
    pub orientation: Orientation,
    /// Keep the nugget fixed at this value.
    pub fixed_nugget: Option<f64>,
    pub max_iter: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FitResult {
    pub model: VariogramModel,
    /// Weighted objective at the optimum divided by the number of points.
    pub score: f64,
    pub evaluations: usize,
}

fn unpack(x: &[f64], cfg: &FitConfig, min_range: f64) -> VariogramModel {
    let nugget = cfg.fixed_nugget.unwrap_or(x[0].max(0.0));
    let structures = cfg
        .kinds
        .iter()
        .enumerate()
        .map(|(s, &kind)| {
            let b = 1 + s * 4;
            Structure {
                kind,
                sill: x[b].max(0.0),
                ranges: [
                    x[b + 1].abs().max(min_range),
                    x[b + 2].abs().max(min_range),
                    x[b + 3].abs().max(min_range),
                ],
            }
        })
        .collect();
    VariogramModel {
        nugget,
        structures,
        orientation: cfg.orientation,
    }
}

fn objective(m: &VariogramModel, exp: &[ExpVariogram]) -> (f64, usize) {
    let (mut sum, mut n) = (0.0, 0usize);
    for ev in exp {
        for p in &ev.points {
            let g = m.gamma_along(ev.dir, p.lag);
            let w = p.pairs as f64 / (g * g).max(1e-12);
            sum += w * (p.gamma - g).powi(2);
            n += 1;
        }
    }
    (sum, n)
}

/// Nelder-Mead minimiser (adaptive-free classic coefficients).
pub fn nelder_mead(
    f: &mut dyn FnMut(&[f64]) -> f64,
    x0: &[f64],
    step: &[f64],
    max_iter: usize,
) -> (Vec<f64>, f64, usize) {
    let n = x0.len();
    let mut pts: Vec<Vec<f64>> = vec![x0.to_vec()];
    for i in 0..n {
        let mut p = x0.to_vec();
        p[i] += if step[i].abs() > 1e-12 { step[i] } else { 0.1 };
        pts.push(p);
    }
    let mut vals: Vec<f64> = pts.iter().map(|p| f(p)).collect();
    let mut evals = pts.len();
    for _ in 0..max_iter {
        let mut order: Vec<usize> = (0..=n).collect();
        order.sort_by(|&a, &b| vals[a].total_cmp(&vals[b]));
        pts = order.iter().map(|&i| pts[i].clone()).collect();
        vals = order.iter().map(|&i| vals[i]).collect();
        if (vals[n] - vals[0]).abs() <= 1e-12 * (1.0 + vals[0].abs()) {
            break;
        }
        let cen: Vec<f64> = (0..n)
            .map(|d| pts[..n].iter().map(|p| p[d]).sum::<f64>() / n as f64)
            .collect();
        let along = |t: f64| -> Vec<f64> { (0..n).map(|d| cen[d] + t * (pts[n][d] - cen[d])).collect() };
        let xr = along(-1.0);
        let fr = f(&xr);
        evals += 1;
        if fr < vals[0] {
            let xe = along(-2.0);
            let fe = f(&xe);
            evals += 1;
            if fe < fr {
                (pts[n], vals[n]) = (xe, fe);
            } else {
                (pts[n], vals[n]) = (xr, fr);
            }
        } else if fr < vals[n - 1] {
            (pts[n], vals[n]) = (xr, fr);
        } else {
            let xc = if fr < vals[n] { along(-0.5) } else { along(0.5) };
            let fc = f(&xc);
            evals += 1;
            if fc < vals[n].min(fr) {
                (pts[n], vals[n]) = (xc, fc);
            } else {
                let anchor = pts[0].clone();
                for (p, v) in pts.iter_mut().zip(vals.iter_mut()).skip(1) {
                    for (c, a) in p.iter_mut().zip(&anchor) {
                        *c = a + 0.5 * (*c - a);
                    }
                    *v = f(p);
                    evals += 1;
                }
            }
        }
    }
    let best = (0..=n).min_by(|&a, &b| vals[a].total_cmp(&vals[b])).unwrap();
    (pts[best].clone(), vals[best], evals)
}

/// Fits from a starting model (its structures' sills/ranges seed the search).
pub fn fit(exp: &[ExpVariogram], start: &VariogramModel, cfg: &FitConfig) -> FitResult {
    let kinds = if cfg.kinds.is_empty() {
        vec![Kind::Spherical]
    } else {
        cfg.kinds[..cfg.kinds.len().min(3)].to_vec()
    };
    let cfg = FitConfig { kinds, ..cfg.clone() };
    // Nothing to fit: keep the starting model and say so with a NaN score.
    if exp.iter().all(|e| e.points.is_empty()) {
        return FitResult {
            model: start.clone(),
            score: f64::NAN,
            evaluations: 0,
        };
    }
    let min_range = exp
        .iter()
        .flat_map(|e| e.points.first().map(|p| p.lag))
        .fold(f64::MAX, f64::min)
        .min(1e6)
        * 0.25;
    let min_range = if min_range.is_finite() && min_range > 0.0 {
        min_range
    } else {
        1.0
    };
    let mut x0 = vec![start.nugget];
    let mut step = vec![(start.nugget.abs() * 0.3).max(0.05)];
    for (s, kind) in cfg.kinds.iter().enumerate() {
        let st = start.structures.get(s).copied().unwrap_or(Structure {
            kind: *kind,
            sill: start.total_sill() * 0.5,
            ranges: [start.max_range().max(10.0); 3],
        });
        x0.push(st.sill);
        step.push((st.sill.abs() * 0.3).max(0.05));
        for r in st.ranges {
            x0.push(r);
            step.push((r.abs() * 0.3).max(1.0));
        }
    }
    let npts = exp.iter().map(|e| e.points.len()).sum::<usize>().max(1);
    let mut total_evals = 0;
    let mut f = |x: &[f64]| objective(&unpack(x, &cfg, min_range), exp).0;
    let (mut best_x, mut best_f, e) = nelder_mead(&mut f, &x0, &step, cfg.max_iter.max(50));
    total_evals += e;
    // Restarts escape the collapsed simplex of the first run.
    for _ in 0..3 {
        let (x, fv, e) = nelder_mead(&mut f, &best_x, &step, cfg.max_iter.max(50));
        total_evals += e;
        if fv < best_f - 1e-12 {
            (best_x, best_f) = (x, fv);
        } else {
            break;
        }
    }
    FitResult {
        model: unpack(&best_x, &cfg, min_range),
        score: best_f / npts as f64,
        evaluations: total_evals,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vario::{VarioParams, VarioPoint};

    /// Synthetic experimental variograms generated from a known model along
    /// the three principal axes.
    fn synth(truth: &VariogramModel) -> Vec<ExpVariogram> {
        let ax = truth.orientation.axes();
        ax.iter()
            .map(|&d| ExpVariogram {
                dir: d,
                params: VarioParams::omni(10.0, 20),
                points: (1..=20)
                    .map(|k| VarioPoint {
                        lag: k as f64 * 10.0,
                        gamma: truth.gamma_along(d, k as f64 * 10.0),
                        pairs: 500,
                    })
                    .collect(),
            })
            .collect()
    }

    #[test]
    fn recovers_a_known_spherical_model() {
        let truth = VariogramModel {
            nugget: 1.5,
            structures: vec![Structure {
                kind: Kind::Spherical,
                sill: 6.0,
                ranges: [120.0, 70.0, 30.0],
            }],
            orientation: Orientation {
                azimuth: 56.0,
                dip: 0.0,
                rake: 0.0,
            },
        };
        let exp = synth(&truth);
        let start = VariogramModel {
            nugget: 0.5,
            structures: vec![Structure {
                kind: Kind::Spherical,
                sill: 4.0,
                ranges: [80.0, 80.0, 80.0],
            }],
            orientation: truth.orientation,
        };
        let r = fit(
            &exp,
            &start,
            &FitConfig {
                kinds: vec![Kind::Spherical],
                orientation: truth.orientation,
                fixed_nugget: None,
                max_iter: 2000,
            },
        );
        let s = r.model.structures[0];
        assert!((r.model.nugget - 1.5).abs() < 0.15, "{:?}", r.model);
        assert!((s.sill - 6.0).abs() < 0.4, "{:?}", r.model);
        for (got, want) in s.ranges.iter().zip([120.0, 70.0, 30.0]) {
            assert!((got - want).abs() / want < 0.1, "{:?}", r.model);
        }
        assert!(r.score < 0.05, "{}", r.score);
    }

    #[test]
    fn nelder_mead_minimises_a_quadratic() {
        let mut f = |x: &[f64]| (x[0] - 3.0).powi(2) + (x[1] + 1.0).powi(2);
        let (x, v, _) = nelder_mead(&mut f, &[0.0, 0.0], &[1.0, 1.0], 500);
        assert!((x[0] - 3.0).abs() < 1e-4 && (x[1] + 1.0).abs() < 1e-4 && v < 1e-8);
    }

    #[test]
    fn no_points_or_no_kinds_do_not_corrupt_the_model() {
        let start = VariogramModel {
            nugget: 1.0,
            structures: vec![Structure {
                kind: Kind::Spherical,
                sill: 5.0,
                ranges: [100.0; 3],
            }],
            orientation: Orientation::default(),
        };
        let r = fit(
            &[],
            &start,
            &FitConfig {
                kinds: vec![Kind::Spherical],
                orientation: Orientation::default(),
                fixed_nugget: None,
                max_iter: 100,
            },
        );
        assert_eq!(r.model, start);
        assert!(r.score.is_nan());
        let exp = synth(&start);
        let r2 = fit(
            &exp,
            &start,
            &FitConfig {
                kinds: vec![],
                orientation: Orientation::default(),
                fixed_nugget: None,
                max_iter: 300,
            },
        );
        assert_eq!(r2.model.structures[0].kind, Kind::Spherical);
    }

    #[test]
    fn fixed_nugget_is_respected() {
        let truth = VariogramModel {
            nugget: 2.0,
            structures: vec![Structure {
                kind: Kind::Exponential,
                sill: 5.0,
                ranges: [60.0, 60.0, 60.0],
            }],
            orientation: Orientation::default(),
        };
        let r = fit(
            &synth(&truth),
            &truth,
            &FitConfig {
                kinds: vec![Kind::Exponential],
                orientation: truth.orientation,
                fixed_nugget: Some(2.0),
                max_iter: 500,
            },
        );
        assert_eq!(r.model.nugget, 2.0);
    }
}
