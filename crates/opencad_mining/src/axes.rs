//! Axis designer: chains of straights and curves with grade, drawn as one
//! 3D polyline per axis. The design is stored in the polyline's XDATA, so an
//! axis can be continued after reopening the drawing.

use mining_core::{azimuth_to_bearing, parse_bearing, Vec3};
use solids_engine::alignment::{AxisDesign, CurveSpec, DesignElement, Element, Grade, Station, Turn};

use crate::cad::{CadSink, Geom, LayerDef, Prim};
use crate::config::AxesConfig;
use crate::i18n::{fmt, Strings};
use crate::xdata::{Class, Tag};

pub const LAYER_AXIS: &str = "MIN_AX_EJE";

#[derive(Debug, Clone, PartialEq)]
pub struct AxisState {
    pub id: String,
    pub design: AxisDesign,
}

pub fn new_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64);
    format!("{t:x}{:02x}", N.fetch_add(1, Ordering::Relaxed) & 0xff)
}

pub fn start_from(cfg: &AxesConfig) -> Result<Station, String> {
    let az = parse_bearing(&cfg.bearing).map_err(|e| e.to_string())?;
    Ok(Station {
        point: [cfg.e, cfg.n, cfg.z],
        azimuth: az,
    })
}

fn encode_grade(g: Grade) -> String {
    match g {
        Grade::Percent(v) => format!("p;{v}"),
        Grade::Degrees(v) => format!("d;{v}"),
    }
}

fn encode_element(e: &DesignElement) -> String {
    let g = encode_grade(e.grade);
    match e.element {
        Element::Straight { length } => format!("S;{length};{g};{}", e.segment),
        Element::Curve { radius, spec, turn } => {
            let (k, v) = match spec {
                CurveSpec::Length(l) => ("L", l),
                CurveSpec::Angle(a) => ("A", a),
            };
            let t = if turn == Turn::Right { "R" } else { "L" };
            format!("C;{radius};{k};{v};{t};{g};{}", e.segment)
        }
    }
}

fn decode_element(s: &str) -> Option<DesignElement> {
    let p: Vec<&str> = s.split(';').collect();
    let f = |i: usize| p.get(i).and_then(|x| x.parse::<f64>().ok());
    let grade = |i: usize| match *p.get(i)? {
        "p" => Some(Grade::Percent(f(i + 1)?)),
        "d" => Some(Grade::Degrees(f(i + 1)?)),
        _ => None,
    };
    match *p.first()? {
        "S" => Some(DesignElement {
            element: Element::Straight { length: f(1)? },
            grade: grade(2)?,
            segment: f(4)?,
        }),
        "C" => {
            let spec = match *p.get(2)? {
                "L" => CurveSpec::Length(f(3)?),
                "A" => CurveSpec::Angle(f(3)?),
                _ => return None,
            };
            let turn = if *p.get(4)? == "R" { Turn::Right } else { Turn::Left };
            Some(DesignElement {
                element: Element::Curve {
                    radius: f(1)?,
                    spec,
                    turn,
                },
                grade: grade(5)?,
                segment: f(7)?,
            })
        }
        _ => None,
    }
}

pub fn tag(a: &AxisState) -> Tag {
    let st = a.design.start;
    let mut t = Tag::new(Class::Axis)
        .with("id", &a.id)
        .with("e", st.point[0])
        .with("n", st.point[1])
        .with("z", st.point[2])
        .with("az", st.azimuth)
        .with("length", format!("{:.3}", a.design.length()))
        .with("elements", a.design.elements.len());
    for (i, e) in a.design.elements.iter().enumerate() {
        t = t.with(&format!("e{}", i + 1), encode_element(e));
    }
    t
}

/// Rebuilds the design from an `AX_AXIS` tag.
pub fn from_tag(t: &Tag) -> Option<AxisState> {
    if t.class != Class::Axis {
        return None;
    }
    let f = |k: &str| t.get(k)?.parse::<f64>().ok();
    let n: usize = t.get("elements")?.parse().ok()?;
    let mut design = AxisDesign::new(Station {
        point: [f("e")?, f("n")?, f("z")?],
        azimuth: f("az")?,
    });
    for i in 1..=n {
        design.elements.push(decode_element(t.get(&format!("e{i}"))?)?);
    }
    Some(AxisState {
        id: t.get("id")?.to_string(),
        design,
    })
}

fn prim(a: &AxisState) -> Result<(Prim, Station), String> {
    let (pts, end) = a.design.build().map_err(|e| e.to_string())?;
    Ok((
        Prim {
            geom: Geom::Polyline3d(pts),
            layer: LAYER_AXIS.into(),
            color: None,
            tag: tag(a),
        },
        end,
    ))
}

/// Redraws `a` (replacing its previous polyline). Removes it when it has no
/// elements left.
fn redraw(sink: &mut dyn CadSink, a: &AxisState) -> Result<Option<Station>, String> {
    let id = a.id.clone();
    if a.design.elements.is_empty() {
        sink.remove_where(&|t: &Tag| t.class == Class::Axis && t.get("id") == Some(id.as_str()));
        return Ok(None);
    }
    let (p, end) = prim(a)?;
    sink.ensure_layers(&[LayerDef::new(LAYER_AXIS, 6)]);
    sink.remove_where(&|t: &Tag| t.class == Class::Axis && t.get("id") == Some(id.as_str()));
    sink.add(vec![p]);
    Ok(Some(end))
}

pub fn status(s: &Strings, a: &AxisState) -> String {
    match a.design.build() {
        Ok((_, end)) => fmt(
            s.m_axis_status,
            &[
                &a.id,
                &format!("{:.3}", end.point[0]),
                &format!("{:.3}", end.point[1]),
                &format!("{:.3}", end.point[2]),
                &format!("{:.4}", end.azimuth),
                &azimuth_to_bearing(end.azimuth),
                &format!("{:.3}", a.design.length()),
                &a.design.elements.len(),
            ],
        ),
        Err(e) => e.to_string(),
    }
}

/// Adds an element. With "update start" it extends the current axis;
/// otherwise it starts a new axis at the panel's start point. Returns the
/// end station (for refreshing the panel) or `None` on error.
pub fn add_element(
    sink: &mut dyn CadSink,
    s: &Strings,
    cfg: &AxesConfig,
    current: &mut Option<AxisState>,
    element: Element,
) -> Option<Station> {
    let mut next = match current.as_ref().filter(|_| cfg.update_start) {
        Some(a) => a.clone(),
        None => match start_from(cfg) {
            Ok(st) => AxisState {
                id: new_id(),
                design: AxisDesign::new(st),
            },
            Err(e) => {
                sink.error(&fmt(s.m_bad_bearing, &[&e]));
                return None;
            }
        },
    };
    next.design.elements.push(DesignElement {
        element,
        grade: cfg.grade(),
        segment: cfg.segment,
    });
    // Validate before touching the drawing.
    if let Err(e) = next.design.build() {
        sink.error(&e.to_string());
        return None;
    }
    sink.begin("MIN_AX_ELEMENT");
    let end = redraw(sink, &next);
    sink.end();
    match end {
        Ok(end) => {
            sink.info(&status(s, &next));
            *current = Some(next);
            end
        }
        Err(e) => {
            sink.error(&e);
            None
        }
    }
}

/// Removes the last element of the current axis. Returns the new end
/// station (or the axis start when it became empty).
pub fn undo_last(sink: &mut dyn CadSink, s: &Strings, current: &mut Option<AxisState>) -> Option<Station> {
    let Some(mut a) = current.clone() else {
        sink.error(s.m_axis_empty);
        return None;
    };
    a.design.elements.pop();
    sink.begin("MIN_AX_UNDO");
    let r = redraw(sink, &a);
    sink.end();
    match r {
        Ok(Some(end)) => {
            sink.info(&status(s, &a));
            *current = Some(a);
            Some(end)
        }
        Ok(None) => {
            let st = a.design.start;
            *current = None;
            Some(st)
        }
        Err(e) => {
            sink.error(&e);
            None
        }
    }
}

/// End point and tangent azimuth of any polyline (last segment with plan length).
pub fn polyline_end(points: &[Vec3]) -> Option<Station> {
    let end = *points.last()?;
    let prev = points
        .iter()
        .rev()
        .skip(1)
        .find(|p| (end[0] - p[0]).hypot(end[1] - p[1]) > 1e-9)?;
    Some(Station {
        point: end,
        azimuth: mining_core::azimuth_between([prev[0], prev[1]], [end[0], end[1]]),
    })
}

/// Writes a station into the panel fields of the config.
pub fn set_start(cfg: &mut AxesConfig, st: Station) {
    cfg.e = (st.point[0] * 1e6).round() / 1e6;
    cfg.n = (st.point[1] * 1e6).round() / 1e6;
    cfg.z = (st.point[2] * 1e6).round() / 1e6;
    cfg.bearing = format!("{:.6}", st.azimuth)
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cad::mock::MockSink;
    use crate::i18n::{for_lang, Lang};

    fn cfg() -> AxesConfig {
        AxesConfig {
            e: 1000.0,
            n: 2000.0,
            z: 300.0,
            bearing: "N90E".into(),
            grade: -12.0,
            length: 50.0,
            ..Default::default()
        }
    }

    #[test]
    fn series_extends_one_polyline_and_undo_shrinks_it() {
        let s = for_lang(Lang::Es);
        let mut c = cfg();
        let mut cur = None;
        let mut sink = MockSink::default();
        let end = add_element(&mut sink, s, &c, &mut cur, c.straight()).unwrap();
        assert!((end.point[0] - 1050.0).abs() < 1e-9 && (end.point[2] - 294.0).abs() < 1e-9);
        c.radius = 10.0;
        c.curve_method = 1;
        c.angle = 90.0;
        c.turn = 0;
        add_element(&mut sink, s, &c, &mut cur, c.curve()).unwrap();
        assert_eq!(sink.entities.len(), 1, "one polyline per axis");
        let a = cur.clone().unwrap();
        assert_eq!(a.design.elements.len(), 2);
        // The tag rebuilds the same design.
        assert_eq!(from_tag(&sink.entities[0].tag).unwrap(), a);
        let back = undo_last(&mut sink, s, &mut cur).unwrap();
        assert!((back.point[0] - 1050.0).abs() < 1e-9);
        assert_eq!(sink.entities.len(), 1);
        undo_last(&mut sink, s, &mut cur);
        assert!(sink.entities.is_empty() && cur.is_none());
        assert_eq!(sink.undo_depth, 0);
    }

    #[test]
    fn without_update_each_element_is_a_new_axis() {
        let s = for_lang(Lang::En);
        let mut c = cfg();
        c.update_start = false;
        let mut cur = None;
        let mut sink = MockSink::default();
        add_element(&mut sink, s, &c, &mut cur, c.straight());
        add_element(&mut sink, s, &c, &mut cur, c.straight());
        assert_eq!(sink.entities.len(), 2);
    }

    #[test]
    fn invalid_element_draws_nothing() {
        let s = for_lang(Lang::Es);
        let mut c = cfg();
        c.radius = 0.0;
        let mut cur = None;
        let mut sink = MockSink::default();
        assert!(add_element(&mut sink, s, &c, &mut cur, c.curve()).is_none());
        assert!(sink.entities.is_empty() && sink.errors.len() == 1 && cur.is_none());
    }

    #[test]
    fn polyline_end_and_start_fields() {
        let st = polyline_end(&[[0.0, 0.0, 0.0], [10.0, 10.0, 1.0], [10.0, 10.0, 5.0]]).unwrap();
        assert_eq!(st.point, [10.0, 10.0, 5.0]);
        assert!((st.azimuth - 45.0).abs() < 1e-9);
        let mut c = AxesConfig::default();
        set_start(&mut c, st);
        assert_eq!((c.e, c.z, c.bearing.as_str()), (10.0, 5.0, "45"));
    }
}
