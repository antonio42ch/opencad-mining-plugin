//! Per-station frames (Arbitrary Axis Algorithm), deterministic and free of
//! accumulated twist.

use mining_core::{cross, normalize_or, Vec3, X, Y, Z};

/// `(right, up)` for tangent `t`. `right` points to the physical *left* of
/// the advance direction (it is the OCS X axis); `up` completes the frame.
pub fn aaa_frame(t: Vec3) -> (Vec3, Vec3) {
    let right = if t[0].abs() < 1.0 / 64.0 && t[1].abs() < 1.0 / 64.0 {
        normalize_or(cross(Y, t), X)
    } else {
        normalize_or(cross(Z, t), X)
    };
    let up = normalize_or(cross(t, right), Z);
    (right, up)
}

/// Unit tangents by centred differences.
pub fn tangents(stations: &[Vec3]) -> Vec<Vec3> {
    use mining_core::sub;
    let n = stations.len();
    (0..n)
        .map(|i| {
            let (a, b) = match i {
                0 => (stations[0], stations[1.min(n - 1)]),
                i if i == n - 1 => (stations[n - 2], stations[n - 1]),
                i => (stations[i - 1], stations[i + 1]),
            };
            normalize_or(sub(b, a), X)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use mining_core::dot;

    #[test]
    fn frame_is_orthonormal_for_horizontal_and_vertical() {
        for t in [[0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [0.6, 0.0, 0.8]] {
            let (r, u) = aaa_frame(t);
            assert!(dot(r, t).abs() < 1e-12 && dot(u, t).abs() < 1e-12 && dot(r, u).abs() < 1e-12);
        }
        // Heading north: right = Z x Y = -X (physical left).
        assert_eq!(aaa_frame([0.0, 1.0, 0.0]).0, [-1.0, 0.0, 0.0]);
    }
}
