//! Property tests of the desurvey and interval mapping invariants.

use drillhole_engine::desurvey::desurvey;
use drillhole_engine::mapper::interpolate_xyz;
use drillhole_engine::{Collar, Survey};
use mining_core::dist;
use proptest::prelude::*;

fn collar(md: f64) -> Collar {
    Collar {
        hole_id: "P".into(),
        x: 0.0,
        y: 0.0,
        z: 0.0,
        max_depth: md,
        azimuth: None,
        dip: None,
    }
}

prop_compose! {
    fn surveys()(stations in prop::collection::vec((1.0f64..50.0, 0.0f64..360.0, -90.0f64..0.0), 1..15)) -> Vec<Survey> {
        let mut d = 0.0;
        stations.into_iter().map(|(step, az, dip)| {
            let s = Survey { hole_id: "P".into(), depth: d, azimuth: az, dip };
            d += step;
            s
        }).collect()
    }
}

proptest! {
    /// A chord never exceeds the measured length between stations, and the
    /// minimum-curvature arc length equals it, so straight-line distance is
    /// bounded by the measured depth.
    #[test]
    fn chord_is_bounded_by_measured_depth(s in surveys()) {
        let last = s.last().unwrap().depth;
        let trace = desurvey(&collar(last), &s);
        for w in trace.windows(2) {
            let dmd = w[1].md - w[0].md;
            prop_assert!(dist(w[0].xyz, w[1].xyz) <= dmd + 1e-9);
        }
    }

    /// Holes pointing downwards never go up.
    #[test]
    fn downward_holes_descend(s in surveys()) {
        let last = s.last().unwrap().depth;
        let trace = desurvey(&collar(last), &s);
        for w in trace.windows(2) {
            prop_assert!(w[1].xyz[2] <= w[0].xyz[2] + 1e-9);
        }
    }

    /// Interpolation is continuous at survey stations.
    #[test]
    fn interpolation_hits_stations(s in surveys()) {
        let last = s.last().unwrap().depth;
        let trace = desurvey(&collar(last), &s);
        for n in &trace {
            prop_assert!(dist(interpolate_xyz(&trace, n.md), n.xyz) < 1e-6);
        }
    }
}
