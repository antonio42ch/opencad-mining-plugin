//! Arc-length utilities on 3D polylines.

use mining_core::{add, dist, scale, sub, Vec3};

/// Vertices without a closing duplicate (distance < 1e-6).
pub fn clean_vertices(mut pts: Vec<Vec3>) -> Vec<Vec3> {
    pts.dedup_by(|a, b| dist(*a, *b) < 1e-10);
    if pts.len() > 2 && dist(pts[0], *pts.last().unwrap()) < 1e-6 {
        pts.pop();
    }
    pts
}

pub fn total_length(pts: &[Vec3]) -> f64 {
    pts.windows(2).map(|w| dist(w[0], w[1])).sum()
}

/// First point, one every `interval` metres of arc length, and always the last.
pub fn sample_by_distance(pts: &[Vec3], interval: f64) -> Vec<Vec3> {
    let Some(&first) = pts.first() else {
        return Vec::new();
    };
    let mut out = vec![first];
    let (mut acc, mut next) = (0.0, interval);
    for w in pts.windows(2) {
        let (a, b) = (w[0], w[1]);
        let l = dist(a, b);
        if l < 1e-10 {
            continue;
        }
        while next <= acc + l + 1e-10 {
            let t = ((next - acc) / l).clamp(0.0, 1.0);
            out.push(add(a, scale(sub(b, a), t)));
            next += interval;
        }
        acc += l;
    }
    let last = *pts.last().unwrap();
    if dist(*out.last().unwrap(), last) > 1e-6 {
        out.push(last);
    }
    out
}

/// Point at fraction `t` of the total length.
pub fn interpolate_at_param(pts: &[Vec3], t: f64) -> Vec3 {
    if t <= 0.0 || pts.len() < 2 {
        return pts[0];
    }
    if t >= 1.0 {
        return *pts.last().unwrap();
    }
    let target = total_length(pts) * t;
    let mut acc = 0.0;
    for w in pts.windows(2) {
        let l = dist(w[0], w[1]);
        if acc + l >= target && l > 0.0 {
            return add(w[0], scale(sub(w[1], w[0]), (target - acc) / l));
        }
        acc += l;
    }
    *pts.last().unwrap()
}

/// `n` points equally spaced by arc length.
pub fn sample_by_count(pts: &[Vec3], n: usize) -> Vec<Vec3> {
    if n < 2 {
        return pts.first().copied().into_iter().collect();
    }
    (0..n)
        .map(|i| interpolate_at_param(pts, i as f64 / (n - 1) as f64))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sampling_keeps_ends() {
        let p = vec![[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [10.0, 5.0, 0.0]];
        let s = sample_by_distance(&p, 4.0);
        assert_eq!(s.len(), 5); // 0,4,8,12 + end(15)
        assert_eq!(*s.last().unwrap(), [10.0, 5.0, 0.0]);
        assert_eq!(sample_by_count(&p, 4)[2], [10.0, 0.0, 0.0]);
    }

    #[test]
    fn closed_duplicate_removed() {
        let p = clean_vertices(vec![[0.0; 3], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0], [0.0; 3]]);
        assert_eq!(p.len(), 3);
    }
}
