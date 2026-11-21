//! Discontinuity sets: automatic grouping of poles (axial data) and Fisher
//! statistics of each set.

use mining_core::{dot, Vec3};

use crate::stereonet::{axial_angle, density, Line, Plane};

#[derive(Debug, Clone, PartialEq)]
pub struct SetStats {
    pub plane: Plane,
    pub count: usize,
    /// Fisher concentration (N - 1) / (N - R).
    pub k: f64,
    /// 95 % confidence cone half-angle (degrees).
    pub alpha95: f64,
    /// Indices of the poles in the set.
    pub members: Vec<usize>,
}

/// Mean axis of unit vectors: dominant eigenvector of the orientation tensor.
pub fn mean_axis(v: &[Vec3]) -> Option<Vec3> {
    if v.is_empty() {
        return None;
    }
    let mut t = [[0.0; 3]; 3];
    for p in v {
        for i in 0..3 {
            for j in 0..3 {
                t[i][j] += p[i] * p[j];
            }
        }
    }
    // Power iteration from the first vector (never orthogonal to its own set).
    let mut x = v[0];
    for _ in 0..100 {
        let y = [0, 1, 2].map(|i| t[i][0] * x[0] + t[i][1] * x[1] + t[i][2] * x[2]);
        let l = dot(y, y).sqrt();
        if l < 1e-15 {
            return None;
        }
        x = [y[0] / l, y[1] / l, y[2] / l];
    }
    Some(x)
}

/// Fisher statistics of axial data around `axis` (vectors flipped to it).
pub fn fisher(v: &[Vec3], axis: Vec3) -> (f64, f64) {
    let n = v.len() as f64;
    let mut s = [0.0; 3];
    for p in v {
        let f = if dot(*p, axis) < 0.0 { -1.0 } else { 1.0 };
        for i in 0..3 {
            s[i] += f * p[i];
        }
    }
    let r = dot(s, s).sqrt();
    if n < 2.0 || n - r < 1e-12 {
        return (f64::INFINITY, 0.0);
    }
    let k = (n - 1.0) / (n - r);
    let c = 1.0 - (n - r) / r * ((1.0 / 0.05f64).powf(1.0 / (n - 1.0)) - 1.0);
    (k, c.clamp(-1.0, 1.0).acos().to_degrees())
}

/// Groups poles into sets. `n_sets` = 0 picks the density peaks at least
/// `min_angle` apart (at most 6) that hold at least 3 poles; poles farther
/// than `max_angle` from every set stay unassigned.
pub fn find_sets(poles: &[Vec3], n_sets: usize, min_angle: f64, max_angle: f64) -> Vec<SetStats> {
    if poles.is_empty() {
        return Vec::new();
    }
    // Seeds: poles by decreasing density, separated by min_angle.
    let mut order: Vec<(f64, usize)> = poles.iter().enumerate().map(|(i, p)| (density(poles, *p), i)).collect();
    order.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    let limit = if n_sets == 0 { 6 } else { n_sets };
    let mut seeds: Vec<Vec3> = Vec::new();
    for (d, i) in order {
        if seeds.len() >= limit {
            break;
        }
        if n_sets == 0 && d * poles.len() as f64 / 100.0 < 3.0 {
            break;
        }
        if seeds.iter().all(|s| axial_angle(*s, poles[i]) >= min_angle) {
            seeds.push(poles[i]);
        }
    }
    // k-means on axes.
    let mut assign = vec![usize::MAX; poles.len()];
    for _ in 0..50 {
        let mut changed = false;
        for (i, p) in poles.iter().enumerate() {
            let best = seeds
                .iter()
                .enumerate()
                .map(|(k, s)| (axial_angle(*s, *p), k))
                .filter(|(a, _)| *a <= max_angle)
                .min_by(|a, b| a.0.total_cmp(&b.0))
                .map_or(usize::MAX, |x| x.1);
            if assign[i] != best {
                assign[i] = best;
                changed = true;
            }
        }
        for (k, s) in seeds.iter_mut().enumerate() {
            let m: Vec<Vec3> = (0..poles.len()).filter(|&i| assign[i] == k).map(|i| poles[i]).collect();
            if let Some(a) = mean_axis(&m) {
                *s = a;
            }
        }
        if !changed {
            break;
        }
    }
    let mut out: Vec<SetStats> = seeds
        .iter()
        .enumerate()
        .filter_map(|(k, s)| {
            let members: Vec<usize> = (0..poles.len()).filter(|&i| assign[i] == k).collect();
            if members.is_empty() {
                return None;
            }
            let v: Vec<Vec3> = members.iter().map(|&i| poles[i]).collect();
            let (kf, a95) = fisher(&v, *s);
            let pole = Line::from_vector(*s)?;
            Some(SetStats {
                plane: Plane::new(90.0 - pole.plunge, pole.trend + 180.0),
                count: members.len(),
                k: kf,
                alpha95: a95,
                members,
            })
        })
        .collect();
    out.sort_by_key(|s| std::cmp::Reverse(s.count));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic scatter around a plane (± `spread` degrees).
    fn scatter(dip: f64, dd: f64, n: usize, spread: f64) -> Vec<Vec3> {
        (0..n)
            .map(|k| {
                let a = (k as f64 * 2.399_963).sin() * spread;
                let b = (k as f64 * 1.7).cos() * spread;
                Plane::new((dip + a).clamp(0.0, 90.0), dd + b).pole().vector()
            })
            .collect()
    }

    #[test]
    fn three_sets_are_found_with_their_orientation() {
        let mut poles = scatter(75.0, 40.0, 60, 6.0);
        poles.extend(scatter(60.0, 150.0, 40, 6.0));
        poles.extend(scatter(15.0, 260.0, 30, 5.0));
        let sets = find_sets(&poles, 0, 20.0, 30.0);
        assert_eq!(sets.len(), 3, "{sets:#?}");
        assert_eq!(sets.iter().map(|s| s.count).collect::<Vec<_>>(), vec![60, 40, 30]);
        let s0 = &sets[0].plane;
        assert!(
            (s0.dip - 75.0).abs() < 3.0 && crate::stereonet::az_diff(s0.dipdir, 40.0) < 4.0,
            "{s0:?}"
        );
        assert!((sets[2].plane.dip - 15.0).abs() < 3.0);
        assert!(sets.iter().all(|s| s.k > 20.0 && s.alpha95 < 5.0));
        // Asking for one set keeps the strongest.
        assert_eq!(find_sets(&poles, 1, 20.0, 30.0)[0].count, 60);
    }

    #[test]
    fn steep_sets_across_the_rim_and_fisher_values() {
        // Dip 88-90 on both sides of the net is one set.
        let mut poles = scatter(88.0, 0.0, 20, 1.5);
        poles.extend(scatter(88.0, 180.0, 20, 1.5));
        let sets = find_sets(&poles, 0, 20.0, 30.0);
        assert_eq!(sets.len(), 1);
        assert_eq!(sets[0].count, 40);
        // Identical vectors: infinite K, zero cone.
        let v = vec![[0.0, 0.0, -1.0]; 5];
        assert_eq!(fisher(&v, v[0]), (f64::INFINITY, 0.0));
        assert!(find_sets(&[], 0, 20.0, 30.0).is_empty());
    }
}
