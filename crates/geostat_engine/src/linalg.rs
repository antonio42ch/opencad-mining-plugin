//! Small dense solver for kriging systems (n <= ~100).

/// Solves `A x = b` with partial pivoting. `a` is row-major `n x n`.
/// Returns `None` when the matrix is (numerically) singular.
pub fn solve(a: &mut [f64], b: &mut [f64], n: usize) -> Option<()> {
    debug_assert_eq!(a.len(), n * n);
    let scale = a.iter().fold(0.0f64, |m, v| m.max(v.abs())).max(1e-300);
    for col in 0..n {
        let (mut piv, mut best) = (col, a[col * n + col].abs());
        for r in col + 1..n {
            let v = a[r * n + col].abs();
            if v > best {
                (piv, best) = (r, v);
            }
        }
        if best < 1e-13 * scale {
            return None;
        }
        if piv != col {
            for c in 0..n {
                a.swap(col * n + c, piv * n + c);
            }
            b.swap(col, piv);
        }
        let d = a[col * n + col];
        for r in col + 1..n {
            let f = a[r * n + col] / d;
            if f != 0.0 {
                for c in col..n {
                    a[r * n + c] -= f * a[col * n + c];
                }
                b[r] -= f * b[col];
            }
        }
    }
    for r in (0..n).rev() {
        let mut s = b[r];
        for c in r + 1..n {
            s -= a[r * n + c] * b[c];
        }
        b[r] = s / a[r * n + r];
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solves_and_detects_singular() {
        let mut a = [2.0, 1.0, 1.0, 3.0];
        let mut b = [3.0, 5.0];
        solve(&mut a, &mut b, 2).unwrap();
        assert!((b[0] - 0.8).abs() < 1e-12 && (b[1] - 1.4).abs() < 1e-12);
        let mut s = [1.0, 2.0, 2.0, 4.0];
        assert!(solve(&mut s, &mut [1.0, 2.0], 2).is_none());
    }
}
