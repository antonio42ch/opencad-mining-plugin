//! Experimental variograms by the sector method (azimuth/dip tolerances,
//! bandwidths, lag tolerance). Pairs are symmetric (h and -h are the same).

use mining_core::{cross, dot, len, normalize_or, scale, sub, Vec3};

use crate::{par_collect, Sample};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VarioParams {
    /// Direction: azimuth clockwise from north, dip positive down (degrees).
    pub azimuth: f64,
    pub dip: f64,
    pub az_tol: f64,
    pub dip_tol: f64,
    /// Bandwidths (m) perpendicular to the direction, horizontal / vertical; 0 = none.
    pub bw_h: f64,
    pub bw_v: f64,
    pub lag: f64,
    pub lag_tol: f64,
    pub nlags: usize,
}

impl VarioParams {
    /// Omnidirectional (all pairs).
    pub fn omni(lag: f64, nlags: usize) -> Self {
        VarioParams {
            azimuth: 0.0,
            dip: 0.0,
            az_tol: 90.0,
            dip_tol: 90.0,
            bw_h: 0.0,
            bw_v: 0.0,
            lag,
            lag_tol: lag / 2.0,
            nlags,
        }
    }

    pub fn direction(&self) -> Vec3 {
        let (az, dip) = (self.azimuth.to_radians(), self.dip.to_radians());
        [az.sin() * dip.cos(), az.cos() * dip.cos(), -dip.sin()]
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VarioPoint {
    /// Mean distance of the pairs in the bin.
    pub lag: f64,
    pub gamma: f64,
    pub pairs: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExpVariogram {
    pub dir: Vec3,
    pub params: VarioParams,
    pub points: Vec<VarioPoint>,
}

fn ang_diff(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(180.0);
    d.min(180.0 - d)
}

/// Bin index (0-based) of the pair `h`, or `None` when rejected.
fn classify(h: Vec3, p: &VarioParams, d: Vec3, uh: Vec3, v: Vec3) -> Option<usize> {
    let dist = len(h);
    if dist < 1e-9 || p.lag <= 0.0 {
        return None;
    }
    let k = (dist / p.lag).round();
    if k < 1.0 || k as usize > p.nlags || (dist - k * p.lag).abs() > p.lag_tol + 1e-9 {
        return None;
    }
    let along = dot(h, d);
    let sgn = if along < 0.0 { -1.0 } else { 1.0 };
    let h = scale(h, sgn);
    let hh = h[0].hypot(h[1]);
    if p.az_tol < 90.0 && p.dip.abs() < 89.999 {
        if hh < 1e-6 * dist {
            return None;
        }
        let az_h = h[0].atan2(h[1]).to_degrees();
        if ang_diff(az_h, p.azimuth) > p.az_tol + 1e-9 {
            return None;
        }
    }
    if p.dip_tol < 90.0 {
        let dip_h = (-h[2]).atan2(hh).to_degrees();
        if (dip_h - p.dip).abs() > p.dip_tol + 1e-9 {
            return None;
        }
    }
    if p.bw_h > 0.0 || p.bw_v > 0.0 {
        let perp = sub(h, scale(d, dot(h, d)));
        if p.bw_h > 0.0 && dot(perp, uh).abs() > p.bw_h {
            return None;
        }
        if p.bw_v > 0.0 && dot(perp, v).abs() > p.bw_v {
            return None;
        }
    }
    Some(k as usize - 1)
}

pub fn experimental(samples: &[Sample], p: &VarioParams) -> ExpVariogram {
    let d = p.direction();
    let uh = normalize_or(
        [p.azimuth.to_radians().cos(), -p.azimuth.to_radians().sin(), 0.0],
        [1.0, 0.0, 0.0],
    );
    let v = normalize_or(cross(uh, d), [0.0, 0.0, 1.0]);
    let nb = p.nlags;
    let n = samples.len();
    // Per-row accumulation (parallel over the first sample of the pair).
    let rows: Vec<Vec<(f64, f64, u64)>> = par_collect(n, |i| {
        let mut acc = vec![(0.0, 0.0, 0u64); nb];
        let si = &samples[i];
        for sj in &samples[i + 1..] {
            let h = sub(sj.xyz, si.xyz);
            if let Some(b) = classify(h, p, d, uh, v) {
                let dv = si.value - sj.value;
                let a = &mut acc[b];
                a.0 += 0.5 * dv * dv;
                a.1 += len(h);
                a.2 += 1;
            }
        }
        acc
    });
    let mut tot = vec![(0.0, 0.0, 0u64); nb];
    for r in rows {
        for (t, x) in tot.iter_mut().zip(r) {
            t.0 += x.0;
            t.1 += x.1;
            t.2 += x.2;
        }
    }
    let points = tot
        .into_iter()
        .filter(|t| t.2 > 0)
        .map(|(g, l, c)| VarioPoint {
            lag: l / c as f64,
            gamma: g / c as f64,
            pairs: c,
        })
        .collect();
    ExpVariogram {
        dir: d,
        params: *p,
        points,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regular 1-D line along X (east) with a linear trend in value.
    fn line() -> Vec<Sample> {
        (0..50)
            .map(|i| Sample {
                xyz: [i as f64 * 10.0, 0.0, 0.0],
                value: i as f64,
                length: 1.0,
                hole: i,
                domain: 1,
            })
            .collect()
    }

    #[test]
    fn gamma_of_a_ramp_is_half_the_squared_lag_index() {
        let ev = experimental(&line(), &VarioParams::omni(10.0, 5));
        assert_eq!(ev.points.len(), 5);
        for (k, p) in ev.points.iter().enumerate() {
            let m = (k + 1) as f64;
            assert!((p.gamma - 0.5 * m * m).abs() < 1e-9, "{p:?}");
            assert!((p.lag - 10.0 * m).abs() < 1e-9);
            assert_eq!(p.pairs, 50 - (k as u64 + 1));
        }
    }

    #[test]
    fn directional_tolerance_separates_east_from_north() {
        // Two perpendicular lines crossing at the origin: values differ per line.
        let mut s: Vec<Sample> = (0..20)
            .map(|i| Sample {
                xyz: [i as f64 * 10.0, 0.0, 0.0],
                value: 0.0,
                length: 1.0,
                hole: 0,
                domain: 1,
            })
            .collect();
        s.extend((1..20).map(|i| Sample {
            xyz: [0.0, i as f64 * 10.0, 0.0],
            value: i as f64,
            length: 1.0,
            hole: 1,
            domain: 1,
        }));
        let east = VarioParams {
            azimuth: 90.0,
            dip: 0.0,
            az_tol: 10.0,
            dip_tol: 10.0,
            bw_h: 0.0,
            bw_v: 0.0,
            lag: 10.0,
            lag_tol: 5.0,
            nlags: 3,
        };
        let north = VarioParams { azimuth: 0.0, ..east };
        let ge = experimental(&s, &east);
        let gn = experimental(&s, &north);
        // East pairs are all within the zero-valued line (plus the origin shared) -> gamma 0.
        assert!(ge.points.iter().all(|p| p.gamma.abs() < 1e-9), "{ge:?}");
        // North pairs follow the ramp.
        assert!(gn.points[0].gamma > 0.4 && gn.points[0].gamma < 1.0, "{gn:?}");
    }

    #[test]
    fn dip_direction_selects_vertical_pairs() {
        let s: Vec<Sample> = (0..30)
            .map(|i| Sample {
                xyz: [0.0, 0.0, -(i as f64) * 4.0],
                value: i as f64,
                length: 4.0,
                hole: 0,
                domain: 1,
            })
            .collect();
        let vert = VarioParams {
            azimuth: 0.0,
            dip: 90.0,
            az_tol: 22.5,
            dip_tol: 22.5,
            bw_h: 0.0,
            bw_v: 0.0,
            lag: 4.0,
            lag_tol: 2.0,
            nlags: 4,
        };
        let ev = experimental(&s, &vert);
        assert_eq!(ev.points.len(), 4);
        assert!((ev.points[0].gamma - 0.5).abs() < 1e-9);
        let horiz = VarioParams { dip: 0.0, ..vert };
        assert!(experimental(&s, &horiz).points.is_empty());
    }
}
