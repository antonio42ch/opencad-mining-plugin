//! Geological mapping: trace + annotations at a picked point.

use geomap_engine::{annotation_layout, Anchor, AnnotationLabels, Observation, Trace};
use mining_core::{azimuth_to_bearing, parse_bearing, Vec3};

use crate::cad::{CadSink, Geom, LayerDef, Prim};
use crate::config::GeoMapConfig;
use crate::i18n::{fmt, Strings};
use crate::xdata::{Class, Tag};

pub const LAYER_TRACE: &str = "MIN_GM_TRAZA";
pub const LAYER_NOTE: &str = "MIN_GM_NOTA";

fn opt_num(s: &str) -> Option<f64> {
    s.trim().replace(',', ".").parse().ok()
}

pub fn trace_from(cfg: &GeoMapConfig, point: Vec3) -> Result<Trace, String> {
    let azimuth = parse_bearing(&cfg.bearing).map_err(|e| e.to_string())?;
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
            dip: opt_num(&cfg.dip),
            sample_code: cfg.sample.clone(),
        },
    };
    t.validate().map_err(|e| e.to_string())?;
    Ok(t)
}

pub fn scene(t: &Trace, cfg: &GeoMapConfig, s: &Strings) -> Vec<Prim> {
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
    let mut out = vec![Prim {
        geom: Geom::Line(a, b),
        layer: LAYER_TRACE.into(),
        color: Some(1),
        tag,
    }];
    if cfg.annotate {
        let labels = AnnotationLabels {
            thickness: s.a_thickness,
            mineralization: s.a_mineralization,
            alteration: s.a_alteration,
            grade: s.a_grade,
            dip: s.a_dip,
            sample: s.a_sample,
        };
        let lines = t.annotation_lines(&labels);
        let (h, pts) = annotation_layout(t.point, t.length, lines.len());
        out.extend(lines.into_iter().zip(pts).map(|(value, at)| Prim {
            geom: Geom::Text { at, height: h, value },
            layer: LAYER_NOTE.into(),
            color: None,
            tag: Tag::new(Class::GeoNote).with("trace", &id),
        }));
    }
    out
}

pub fn insert(sink: &mut dyn CadSink, s: &Strings, cfg: &GeoMapConfig, point: Vec3) -> bool {
    let t = match trace_from(cfg, point) {
        Ok(t) => t,
        Err(e) => {
            sink.error(&fmt(s.m_bad_bearing, &[&e]));
            return false;
        }
    };
    sink.begin("MIN_GM_TRACE");
    sink.ensure_layers(&[LayerDef::new(LAYER_TRACE, 1), LayerDef::new(LAYER_NOTE, 7)]);
    sink.add(scene(&t, cfg, s));
    sink.end();
    sink.info(&fmt(
        s.m_trace_done,
        &[&format!("{:.1}", t.azimuth), &azimuth_to_bearing(t.azimuth)],
    ));
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cad::mock::MockSink;
    use crate::i18n::{for_lang, Lang};

    #[test]
    fn inserts_trace_and_notes() {
        let cfg = GeoMapConfig {
            bearing: "N90E".into(),
            length: 10.0,
            rock_type: "Andesita".into(),
            dip: "65".into(),
            ..Default::default()
        };
        let mut sink = MockSink::default();
        assert!(insert(&mut sink, for_lang(Lang::Es), &cfg, [0.0, 0.0, 100.0]));
        let line = &sink.entities[0];
        let Geom::Line(a, b) = line.geom else {
            panic!("expected a line")
        };
        let close = |p: [f64; 3], q: [f64; 3]| (0..3).all(|i| (p[i] - q[i]).abs() < 1e-9);
        assert!(
            close(a, [-5.0, 0.0, 100.0]) && close(b, [5.0, 0.0, 100.0]),
            "{a:?} {b:?}"
        );
        assert_eq!(line.tag.get("rock"), Some("Andesita"));
        assert_eq!(
            sink.entities.iter().filter(|p| p.tag.class == Class::GeoNote).count(),
            2
        );
    }

    #[test]
    fn rejects_bad_bearing() {
        let cfg = GeoMapConfig {
            bearing: "N95E".into(),
            ..Default::default()
        };
        let mut sink = MockSink::default();
        assert!(!insert(&mut sink, for_lang(Lang::Es), &cfg, [0.0; 3]));
        assert!(sink.entities.is_empty() && sink.errors.len() == 1);
    }
}
