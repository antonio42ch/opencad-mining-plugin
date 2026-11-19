//! Minimum-curvature desurvey, computed with tangent vectors so the dogleg is
//! always correct (the legacy C# mixed inclination-from-vertical with the
//! from-horizontal dogleg formula, which inflated the dZ by ~27 % in some cases).

use mining_core::{add, dot, scale, Vec3};

use crate::model::*;

/// Unit tangent of a station. `dip` -90 = down. Z up.
pub fn tangent(azimuth_deg: f64, dip_deg: f64) -> Vec3 {
    let inc = (dip_deg + 90.0).to_radians(); // from vertical
    let az = azimuth_deg.to_radians();
    [inc.sin() * az.sin(), inc.sin() * az.cos(), -inc.cos()]
}

/// Ratio factor of the minimum-curvature method for dogleg `beta`.
fn ratio_factor(beta: f64) -> f64 {
    if beta.abs() < 1e-10 {
        1.0
    } else if beta.abs() < 1e-4 {
        1.0 + beta * beta / 12.0 + 3.0 * beta.powi(4) / 360.0
    } else {
        (2.0 / beta) * (beta / 2.0).tan()
    }
}

/// Displacement along an arc from tangent `t1` to `t2` over `dmd` metres.
pub fn min_curvature_step(t1: Vec3, t2: Vec3, dmd: f64) -> Vec3 {
    let beta = dot(t1, t2).clamp(-1.0, 1.0).acos();
    scale(add(t1, t2), dmd / 2.0 * ratio_factor(beta))
}

/// Spherical interpolation of two unit tangents, `s` in [0, 1].
pub fn slerp(t1: Vec3, t2: Vec3, s: f64) -> Vec3 {
    let beta = dot(t1, t2).clamp(-1.0, 1.0).acos();
    if beta < 1e-10 {
        return t1;
    }
    let (a, b) = (((1.0 - s) * beta).sin() / beta.sin(), (s * beta).sin() / beta.sin());
    add(scale(t1, a), scale(t2, b))
}

/// Builds the trace of one hole. `surveys` must already belong to the hole.
/// Holes without surveys use the collar direction (default vertical).
pub fn desurvey(collar: &Collar, surveys: &[Survey]) -> Vec<SurveyNode> {
    // Non-finite stations are ignored here even if the caller skipped
    // validation, so this public function can never panic on NaN.
    let mut s: Vec<Survey> = surveys
        .iter()
        .filter(|v| v.depth.is_finite() && v.azimuth.is_finite() && v.dip.is_finite())
        .cloned()
        .collect();
    s.sort_by(|a, b| a.depth.total_cmp(&b.depth));
    if s.first().is_none_or(|f| f.depth > 0.0) {
        s.insert(
            0,
            Survey {
                hole_id: collar.hole_id.clone(),
                depth: 0.0,
                azimuth: collar.azimuth.unwrap_or(0.0),
                dip: collar.dip.unwrap_or(-90.0),
            },
        );
    }

    let mut nodes = vec![SurveyNode {
        md: s[0].depth,
        xyz: [collar.x, collar.y, collar.z],
        azimuth: s[0].azimuth,
        dip: s[0].dip,
        is_synthetic: false,
    }];
    for w in s.windows(2) {
        let (a, b) = (&w[0], &w[1]);
        let dmd = b.depth - a.depth;
        let step = min_curvature_step(tangent(a.azimuth, a.dip), tangent(b.azimuth, b.dip), dmd);
        let prev = nodes.last().unwrap().xyz;
        nodes.push(SurveyNode {
            md: b.depth,
            xyz: add(prev, step),
            azimuth: b.azimuth,
            dip: b.dip,
            is_synthetic: false,
        });
    }

    // Bottom of hole: tangent extrapolation to MaxDepth.
    let last = nodes.last().unwrap().clone();
    if collar.max_depth.is_finite() && last.md < collar.max_depth {
        let rem = collar.max_depth - last.md;
        nodes.push(SurveyNode {
            md: collar.max_depth,
            xyz: add(last.xyz, scale(tangent(last.azimuth, last.dip), rem)),
            azimuth: last.azimuth,
            dip: last.dip,
            is_synthetic: true,
        });
    }
    nodes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collar(md: f64) -> Collar {
        Collar {
            hole_id: "H".into(),
            x: 100.0,
            y: 200.0,
            z: 300.0,
            max_depth: md,
            azimuth: None,
            dip: None,
        }
    }
    fn sv(d: f64, az: f64, dip: f64) -> Survey {
        Survey {
            hole_id: "H".into(),
            depth: d,
            azimuth: az,
            dip,
        }
    }
    fn close(a: Vec3, b: Vec3, tol: f64) -> bool {
        (0..3).all(|i| (a[i] - b[i]).abs() < tol)
    }

    #[test]
    fn vertical_hole_without_surveys() {
        let n = desurvey(&collar(50.0), &[]);
        assert!(close(n.last().unwrap().xyz, [100.0, 200.0, 250.0], 1e-9));
    }

    #[test]
    fn straight_inclined_hole() {
        // dip -60, az 90 (east): I = 30 deg from vertical.
        let n = desurvey(&collar(100.0), &[sv(0.0, 90.0, -60.0), sv(100.0, 90.0, -60.0)]);
        let e = n.last().unwrap().xyz;
        assert!(close(
            e,
            [100.0 + 100.0 * 0.5, 200.0, 300.0 - 100.0 * 3f64.sqrt() / 2.0],
            1e-9
        ));
    }

    #[test]
    fn two_vertical_stations_with_different_azimuth_stay_vertical() {
        // The case the legacy dogleg got wrong: beta must be 0, no inflated dZ.
        let n = desurvey(&collar(100.0), &[sv(0.0, 0.0, -90.0), sv(100.0, 90.0, -90.0)]);
        assert!(close(n.last().unwrap().xyz, [100.0, 200.0, 200.0], 1e-9));
    }

    #[test]
    fn circular_arc_is_exact() {
        // Build-up from vertical to horizontal heading north, radius R.
        let r = 100.0;
        let md = r * std::f64::consts::FRAC_PI_2;
        let n = desurvey(&collar(md), &[sv(0.0, 0.0, -90.0), sv(md, 0.0, 0.0)]);
        let e = n.last().unwrap().xyz;
        assert!(close(e, [100.0, 200.0 + r, 300.0 - r], 1e-6), "{e:?}");
    }

    #[test]
    fn nan_surveys_do_not_panic() {
        let n = desurvey(
            &collar(10.0),
            &[sv(f64::NAN, 0.0, -90.0), sv(5.0, f64::NAN, -90.0), sv(0.0, 0.0, -90.0)],
        );
        assert!(close(n.last().unwrap().xyz, [100.0, 200.0, 290.0], 1e-9));
    }

    #[test]
    fn eoh_extrapolates_and_marks_synthetic() {
        let n = desurvey(&collar(30.0), &[sv(0.0, 0.0, -90.0), sv(10.0, 0.0, -90.0)]);
        let l = n.last().unwrap();
        assert!(l.is_synthetic && (l.md - 30.0).abs() < 1e-12);
        assert!(close(l.xyz, [100.0, 200.0, 270.0], 1e-9));
    }
}
