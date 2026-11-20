//! Geological mapping: draw a trace from coordinates + azimuth/bearing, and
//! project mapped traces to another height along their dip.

use geomap_engine::{project_polyline, Anchor, DipDirection, Height, Observation, ProjectError, Trace};
use mining_core::{azimuth_between, azimuth_to_bearing, normalize360, parse_bearing, Vec3};

use crate::cad::{CadSink, Geom, LayerDef, Prim};
use crate::config::GeoMapConfig;
use crate::i18n::{fmt, Strings};
use crate::xdata::{Class, Tag};

pub const LAYER_TRACE: &str = "MIN_GM_TRAZA";
pub const LAYER_PROJECTION: &str = "MIN_GM_PROYECCION";
const PROJECTION_COLOR: u8 = 30;

fn opt_num(s: &str) -> Option<f64> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        t.replace(',', ".").parse().ok()
    }
}

fn num(v: f64) -> String {
    let s = format!("{v:.3}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// Dip azimuth implied by the settings for a trace running along `azimuth`.
fn dip_azimuth(cfg: &GeoMapConfig, azimuth: f64) -> Option<f64> {
    match cfg.dip_dir {
        0 => Some(normalize360(azimuth + 90.0)),
        1 => Some(normalize360(azimuth - 90.0)),
        _ => parse_bearing(&cfg.dip_az).ok(),
    }
}

/// Builds the trace from the panel settings at `point`.
pub fn trace_from(cfg: &GeoMapConfig, point: Vec3) -> Result<Trace, String> {
    let azimuth = parse_bearing(&cfg.bearing).map_err(|e| e.to_string())?;
    let dip = opt_num(&cfg.dip);
    let t = Trace {
        point,
        azimuth,
        length: cfg.length,
        anchor: [Anchor::Start, Anchor::Center, Anchor::End][cfg.anchor.min(2)],
        observation: Observation {
            rock_type: cfg.rock_type.clone(),
            thickness: opt_num(&cfg.thickness),
            mineralization: cfg.mineralization.clone(),
            alteration: cfg.alteration.clone(),
            structure: cfg.structure.clone(),
            grade: cfg.grade.clone(),
            dip,
            dip_direction: dip.and_then(|_| dip_azimuth(cfg, azimuth)),
            sample_code: cfg.sample.clone(),
        },
    };
    t.validate().map_err(|e| e.to_string())?;
    Ok(t)
}

pub fn scene(t: &Trace) -> Vec<Prim> {
    let (a, b) = t.endpoints();
    let o = &t.observation;
    let id = format!("{:.3}_{:.3}_{:.3}", t.point[0], t.point[1], t.point[2]);
    let mut tag = Tag::new(Class::GeoTrace)
        .with("id", &id)
        .with("azimuth", t.azimuth)
        .with("length", t.length);
    for (k, v) in [
        ("rock", &o.rock_type),
        ("mineralization", &o.mineralization),
        ("alteration", &o.alteration),
        ("structure", &o.structure),
        ("grade", &o.grade),
        ("sample", &o.sample_code),
    ] {
        if !v.is_empty() {
            tag = tag.with(k, v);
        }
    }
    if let Some(x) = o.thickness {
        tag = tag.with("thickness", x);
    }
    if let Some(x) = o.dip {
        tag = tag.with("dip", x);
    }
    if let Some(x) = o.dip_direction {
        tag = tag.with("dipdir", format!("{x:.3}"));
    }
    vec![Prim {
        geom: Geom::Line(a, b),
        layer: LAYER_TRACE.into(),
        color: Some(1),
        tag,
    }]
}

/// Draws the trace at `point`.
pub fn insert(sink: &mut dyn CadSink, s: &Strings, cfg: &GeoMapConfig, point: Vec3) -> bool {
    if point.iter().any(|c| !c.is_finite()) {
        sink.error(&fmt(
            s.m_trace_coords,
            &[&format!("{} {} {}", point[0], point[1], point[2])],
        ));
        return false;
    }
    let t = match trace_from(cfg, point) {
        Ok(t) => t,
        Err(e) => {
            sink.error(&fmt(s.m_bad_bearing, &[&e]));
            return false;
        }
    };
    sink.begin("MIN_GM_TRACE");
    sink.ensure_layers(&[LayerDef::new(LAYER_TRACE, 1)]);
    sink.add(scene(&t));
    sink.end();
    sink.info(&fmt(s.m_trace_pt, &[&num(point[0]), &num(point[1]), &num(point[2])]));
    sink.info(&fmt(
        s.m_trace_done,
        &[&format!("{:.1}", t.azimuth), &azimuth_to_bearing(t.azimuth)],
    ));
    true
}

// ───────────────────────────── Projection ─────────────────────────────

/// One mapped trace picked for projection.
#[derive(Debug, Clone)]
pub struct Source {
    /// Hex handle of the entity (for the XDATA link).
    pub handle: String,
    pub points: Vec<Vec3>,
    pub closed: bool,
    /// Open Mining tag of the entity, if it has one (dip fallback).
    pub tag: Option<Tag>,
}

fn height_of(cfg: &GeoMapConfig) -> Result<Height, String> {
    Ok(if cfg.proj_mode == 1 {
        Height::Delta(cfg.proj_value)
    } else {
        Height::Absolute(cfg.proj_value)
    })
}

/// Dip and direction for one source: the panel wins; an empty panel dip falls
/// back to what the trace stored when it was drawn.
fn dip_for(cfg: &GeoMapConfig, src: &Source, s: &Strings) -> Result<(f64, DipDirection), String> {
    if let Some(d) = opt_num(&cfg.dip) {
        let dir = match cfg.dip_dir {
            0 => DipDirection::Right,
            1 => DipDirection::Left,
            _ => DipDirection::Fixed(parse_bearing(&cfg.dip_az).map_err(|e| fmt(s.m_bad_bearing, &[&e]))?),
        };
        return Ok((d, dir));
    }
    let stored = src.tag.as_ref().filter(|t| t.class == Class::GeoTrace);
    let dip = stored.and_then(|t| t.get("dip")).and_then(opt_num);
    match (dip, stored.and_then(|t| t.get("dipdir")).and_then(opt_num)) {
        (Some(d), Some(az)) => Ok((d, DipDirection::Fixed(az))),
        (Some(d), None) => Ok((d, DipDirection::Right)),
        _ => Err(s.m_proj_no_dip.to_string()),
    }
}

fn engine_error(e: ProjectError, s: &Strings) -> String {
    match e {
        ProjectError::InvalidDip(_) => s.m_proj_dip_range.to_string(),
        ProjectError::NoStrike => s.m_proj_no_strike.to_string(),
        ProjectError::InvalidHeight => s.m_proj_bad_height.to_string(),
        other => other.to_string(),
    }
}

/// Projection of every source (pure, so it is tested without a CAD).
pub fn projections(s: &Strings, cfg: &GeoMapConfig, sources: &[Source]) -> Result<Vec<Prim>, String> {
    let height = height_of(cfg)?;
    let mut out = Vec::new();
    for src in sources {
        let (dip, dir) = dip_for(cfg, src, s)?;
        let pts = project_polyline(&src.points, src.closed, dip, dir, height).map_err(|e| engine_error(e, s))?;
        let dip_az = match dir {
            DipDirection::Fixed(az) => normalize360(az),
            DipDirection::Right | DipDirection::Left => {
                let a = src
                    .points
                    .windows(2)
                    .find(|w| (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1]) > 1e-9);
                let along = a.map_or(0.0, |w| azimuth_between([w[0][0], w[0][1]], [w[1][0], w[1][1]]));
                normalize360(along + if dir == DipDirection::Right { 90.0 } else { -90.0 })
            }
        };
        let (mode, value) = match height {
            Height::Absolute(v) => ("height", v),
            Height::Delta(v) => ("delta", v),
        };
        let tag = Tag::new(Class::GeoProj)
            .with("src", &src.handle)
            .with("dip", dip)
            .with("dipdir", format!("{dip_az:.3}"))
            .with(mode, value);
        out.push(Prim {
            geom: if src.closed {
                Geom::Polygon3d(pts)
            } else {
                Geom::Polyline3d(pts)
            },
            layer: LAYER_PROJECTION.into(),
            color: Some(PROJECTION_COLOR),
            tag,
        });
    }
    Ok(out)
}

/// Projects the sources and draws the results (one undo step). The originals
/// are left untouched.
pub fn project(sink: &mut dyn CadSink, s: &Strings, cfg: &GeoMapConfig, sources: &[Source]) -> usize {
    if sources.is_empty() {
        sink.error(s.m_proj_none);
        return 0;
    }
    let prims = match projections(s, cfg, sources) {
        Ok(p) => p,
        Err(e) => {
            sink.error(&e);
            return 0;
        }
    };
    let first_shift = match (&prims[0].geom, sources[0].points.first()) {
        (Geom::Polyline3d(p) | Geom::Polygon3d(p), Some(a)) => {
            p.first().map_or(0.0, |q| (q[0] - a[0]).hypot(q[1] - a[1]))
        }
        _ => 0.0,
    };
    let (dip, height) = match (
        &prims[0].tag.get("dip"),
        &prims[0].tag.get("height"),
        &prims[0].tag.get("delta"),
    ) {
        (Some(d), Some(h), _) => (d.to_string(), h.to_string()),
        (Some(d), None, Some(dl)) => (d.to_string(), format!("Δ{dl}")),
        _ => (String::new(), String::new()),
    };
    sink.begin("MIN_GM_PROJECT");
    sink.ensure_layers(&[LayerDef::new(LAYER_PROJECTION, PROJECTION_COLOR)]);
    let n = sink.add(prims);
    sink.end();
    sink.info(&fmt(s.m_proj_done, &[&n, &dip, &height, &num(first_shift)]));
    n
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cad::mock::MockSink;
    use crate::i18n::{for_lang, Lang};

    fn close(a: Vec3, b: Vec3) -> bool {
        (0..3).all(|i| (a[i] - b[i]).abs() < 1e-9)
    }

    #[test]
    fn trace_is_drawn_from_coordinates_and_bearing() {
        let cfg = GeoMapConfig {
            e: 500_000.0,
            n: 8_500_000.0,
            z: 4_000.0,
            bearing: "N90E".into(),
            length: 10.0,
            ..Default::default()
        };
        let mut sink = MockSink::default();
        assert!(insert(&mut sink, for_lang(Lang::Es), &cfg, [cfg.e, cfg.n, cfg.z]));
        assert_eq!(sink.entities.len(), 1, "a line and nothing else: no text annotations");
        let Geom::Line(a, b) = sink.entities[0].geom else {
            panic!("expected a line")
        };
        assert!(
            close(a, [499_995.0, 8_500_000.0, 4_000.0]) && close(b, [500_005.0, 8_500_000.0, 4_000.0]),
            "{a:?} {b:?}"
        );
        assert!(sink.entities.iter().all(|p| !matches!(p.geom, Geom::Text { .. })));
        assert_eq!(sink.undo_depth, 0);
        // Bearing forms.
        for (txt, az) in [("S30W", 210.0), ("135.5", 135.5), ("N10O", 350.0)] {
            let c = GeoMapConfig {
                bearing: txt.into(),
                ..cfg.clone()
            };
            assert!((trace_from(&c, [0.0; 3]).unwrap().azimuth - az).abs() < 1e-9, "{txt}");
        }
    }

    #[test]
    fn bad_input_draws_nothing() {
        let s = for_lang(Lang::En);
        let mut sink = MockSink::default();
        assert!(!insert(
            &mut sink,
            s,
            &GeoMapConfig {
                bearing: "N95E".into(),
                ..Default::default()
            },
            [0.0; 3]
        ));
        assert!(!insert(&mut sink, s, &GeoMapConfig::default(), [f64::NAN, 0.0, 0.0]));
        assert!(!insert(
            &mut sink,
            s,
            &GeoMapConfig {
                length: 0.0,
                ..Default::default()
            },
            [0.0; 3]
        ));
        assert!(sink.entities.is_empty() && sink.errors.len() == 3);
    }

    #[test]
    fn dip_and_direction_are_stored_with_the_trace() {
        let right = GeoMapConfig {
            bearing: "N90E".into(),
            dip: "65".into(),
            dip_dir: 0,
            ..Default::default()
        };
        let t = trace_from(&right, [0.0; 3]).unwrap();
        assert_eq!(
            (t.observation.dip, t.observation.dip_direction),
            (Some(65.0), Some(180.0))
        );
        let left = GeoMapConfig {
            dip_dir: 1,
            ..right.clone()
        };
        assert_eq!(
            trace_from(&left, [0.0; 3]).unwrap().observation.dip_direction,
            Some(0.0)
        );
        let fixed = GeoMapConfig {
            dip_dir: 2,
            dip_az: "S45E".into(),
            ..right
        };
        let tag = &scene(&trace_from(&fixed, [0.0; 3]).unwrap())[0].tag;
        assert_eq!((tag.get("dip"), tag.get("dipdir")), (Some("65"), Some("135.000")));
        // No dip, nothing stored.
        assert!(scene(&trace_from(&GeoMapConfig::default(), [0.0; 3]).unwrap())[0]
            .tag
            .get("dip")
            .is_none());
    }

    fn source(points: Vec<Vec3>, closed: bool, tag: Option<Tag>) -> Source {
        Source {
            handle: "2A".into(),
            points,
            closed,
            tag,
        }
    }

    fn east_line() -> Source {
        source(vec![[0.0, 0.0, 100.0], [20.0, 0.0, 100.0]], false, None)
    }

    #[test]
    fn projects_to_a_target_height_or_by_a_difference() {
        let s = for_lang(Lang::Es);
        let abs = GeoMapConfig {
            dip: "45".into(),
            dip_dir: 0,
            proj_mode: 0,
            proj_value: 90.0,
            ..Default::default()
        };
        let mut sink = MockSink::default();
        assert_eq!(project(&mut sink, s, &abs, &[east_line()]), 1);
        let Geom::Polyline3d(p) = &sink.entities[0].geom else {
            panic!()
        };
        assert!(
            close(p[0], [0.0, -10.0, 90.0]) && close(p[1], [20.0, -10.0, 90.0]),
            "{p:?}"
        );
        let t = &sink.entities[0].tag;
        assert_eq!(
            (t.class, t.get("src"), t.get("dip"), t.get("height")),
            (Class::GeoProj, Some("2A"), Some("45"), Some("90"))
        );
        assert_eq!(t.get("dipdir"), Some("180.000"));
        assert_eq!(
            (sink.entities[0].layer.as_str(), sink.entities[0].color),
            (LAYER_PROJECTION, Some(30))
        );
        // A difference of +10 (up) with the same dip goes the other way.
        let delta = GeoMapConfig {
            proj_mode: 1,
            proj_value: 10.0,
            ..abs.clone()
        };
        let mut sink2 = MockSink::default();
        project(&mut sink2, s, &delta, &[east_line()]);
        let Geom::Polyline3d(q) = &sink2.entities[0].geom else {
            panic!()
        };
        assert!(close(q[0], [0.0, 10.0, 110.0]), "{q:?}");
        assert_eq!(sink2.entities[0].tag.get("delta"), Some("10"));
        assert!(
            sink.infos.last().unwrap().contains("10"),
            "reports the shift: {:?}",
            sink.infos
        );
        // The original is untouched: nothing but the projection was added.
        assert_eq!(sink.entities.len(), 1);
        assert_eq!(sink.undo_depth, 0);
    }

    #[test]
    fn projecting_twice_adds_two_levels_without_replacing() {
        let s = for_lang(Lang::En);
        let mut sink = MockSink::default();
        for z in [90.0, 80.0] {
            project(
                &mut sink,
                s,
                &GeoMapConfig {
                    dip: "60".into(),
                    proj_value: z,
                    ..Default::default()
                },
                &[east_line()],
            );
        }
        assert_eq!(sink.entities.len(), 2);
    }

    #[test]
    fn empty_panel_dip_uses_the_dip_stored_with_the_trace() {
        let s = for_lang(Lang::Es);
        // Draw a trace with dip 50 to the fixed azimuth 0 (north), then project it with an empty dip field.
        let draw = GeoMapConfig {
            bearing: "N90E".into(),
            dip: "50".into(),
            dip_dir: 2,
            dip_az: "0".into(),
            length: 20.0,
            anchor: 0,
            ..Default::default()
        };
        let prim = &scene(&trace_from(&draw, [0.0, 0.0, 100.0]).unwrap())[0];
        let Geom::Line(a, b) = prim.geom else { panic!() };
        let src = source(vec![a, b], false, Some(prim.tag.clone()));
        let cfg = GeoMapConfig {
            dip: String::new(),
            proj_mode: 1,
            proj_value: -10.0,
            ..Default::default()
        };
        let out = projections(s, &cfg, std::slice::from_ref(&src)).unwrap();
        let Geom::Polyline3d(p) = &out[0].geom else { panic!() };
        // Down 10 m at 50 degrees towards the north: 10 / tan 50 = 8.39 m north.
        assert!(
            (p[0][1] - 10.0 / 50f64.to_radians().tan()).abs() < 1e-9 && (p[0][2] - 90.0).abs() < 1e-9,
            "{p:?}"
        );
        // A panel dip overrides the stored one.
        let over = GeoMapConfig {
            dip: "45".into(),
            dip_dir: 0,
            ..cfg.clone()
        };
        let o = projections(s, &over, &[src]).unwrap();
        let Geom::Polyline3d(q) = &o[0].geom else { panic!() };
        assert!((q[0][1] + 10.0).abs() < 1e-9, "dip 45 to the right (south): {q:?}");
        // No dip anywhere: explained.
        let err = projections(s, &cfg, &[east_line()]).unwrap_err();
        assert_eq!(err, s.m_proj_no_dip);
    }

    #[test]
    fn closed_polylines_stay_closed_and_errors_are_localised() {
        let s = for_lang(Lang::Es);
        let sq = source(
            vec![
                [0.0, 0.0, 100.0],
                [0.0, 10.0, 100.0],
                [10.0, 10.0, 100.0],
                [10.0, 0.0, 100.0],
            ],
            true,
            None,
        );
        let cfg = GeoMapConfig {
            dip: "45".into(),
            proj_value: 98.0,
            ..Default::default()
        };
        let out = projections(s, &cfg, &[sq]).unwrap();
        assert!(matches!(&out[0].geom, Geom::Polygon3d(p) if p.len() == 4));
        for (dip, expected) in [("0", s.m_proj_dip_range), ("95", s.m_proj_dip_range)] {
            let c = GeoMapConfig {
                dip: dip.into(),
                ..cfg.clone()
            };
            assert_eq!(projections(s, &c, &[east_line()]).unwrap_err(), expected);
        }
        let vertical = source(vec![[0.0, 0.0, 100.0], [0.0, 0.0, 90.0]], false, None);
        assert_eq!(
            projections(s, &cfg, std::slice::from_ref(&vertical)).unwrap_err(),
            s.m_proj_no_strike
        );
        let fixed = GeoMapConfig {
            dip_dir: 2,
            dip_az: "N90E".into(),
            ..cfg.clone()
        };
        assert!(projections(s, &fixed, &[vertical]).is_ok());
        let nan = GeoMapConfig {
            proj_value: f64::NAN,
            ..cfg.clone()
        };
        assert_eq!(projections(s, &nan, &[east_line()]).unwrap_err(), s.m_proj_bad_height);
        let bad_az = GeoMapConfig {
            dip_dir: 2,
            dip_az: "xx".into(),
            ..cfg
        };
        assert!(projections(s, &bad_az, &[east_line()]).is_err());
        let mut sink = MockSink::default();
        assert_eq!(project(&mut sink, s, &GeoMapConfig::default(), &[]), 0);
        assert_eq!(sink.errors, vec![s.m_proj_none.to_string()]);
    }
}
