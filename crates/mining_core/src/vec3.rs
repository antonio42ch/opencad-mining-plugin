//! Minimal 3D vector algebra on `[f64; 3]`.

pub type Vec3 = [f64; 3];
pub type Vec2 = [f64; 2];

pub const X: Vec3 = [1.0, 0.0, 0.0];
pub const Y: Vec3 = [0.0, 1.0, 0.0];
pub const Z: Vec3 = [0.0, 0.0, 1.0];

pub fn add(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
pub fn sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
pub fn scale(a: Vec3, s: f64) -> Vec3 {
    [a[0] * s, a[1] * s, a[2] * s]
}
pub fn dot(a: Vec3, b: Vec3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
pub fn cross(a: Vec3, b: Vec3) -> Vec3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
pub fn len(a: Vec3) -> f64 {
    dot(a, a).sqrt()
}
pub fn dist(a: Vec3, b: Vec3) -> f64 {
    len(sub(b, a))
}
pub fn lerp(a: Vec3, b: Vec3, t: f64) -> Vec3 {
    add(a, scale(sub(b, a), t))
}
/// Unit vector, or `fallback` when the length is below `1e-9`.
pub fn normalize_or(a: Vec3, fallback: Vec3) -> Vec3 {
    let l = len(a);
    if l <= 1e-9 {
        fallback
    } else {
        scale(a, 1.0 / l)
    }
}
/// Arithmetic mean of points (empty -> origin).
pub fn mean(points: &[Vec3]) -> Vec3 {
    if points.is_empty() {
        return [0.0; 3];
    }
    let s = points.iter().fold([0.0; 3], |acc, p| add(acc, *p));
    scale(s, 1.0 / points.len() as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cross_is_right_handed() {
        assert_eq!(cross(X, Y), Z);
        assert_eq!(cross(Z, Y), [-1.0, 0.0, 0.0]);
    }

    #[test]
    fn normalize_fallback() {
        assert_eq!(normalize_or([0.0; 3], X), X);
        assert!((len(normalize_or([3.0, 4.0, 0.0], X)) - 1.0).abs() < 1e-12);
    }
}
