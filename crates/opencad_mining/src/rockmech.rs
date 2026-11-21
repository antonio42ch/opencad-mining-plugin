//! Geotechnical phases 3 and 4 bound to the plugin: poles from the tables
//! and traces, the stereonet (SVG and DWG), wedges drawn on the working and
//! the design charts. Pure functions over `CadSink` primitives.

use geotech_engine::data::{GtField, GtTable};
use geotech_engine::stereonet::{contour, density_grid, project, Line, Plane};
use geotech_engine::wedge::{frame, FaceKind, Opening, Wedge};
use mining_core::{add, scale, LocalOrigin, Vec3};

use crate::cad::{Geom, Prim};
use crate::charts::Series;
use crate::xdata::{Class, Tag};

pub const LAYER_STEREO: &str = "MIN_ST_ESTEREO";
pub const LAYER_WEDGES: &str = "MIN_ST_CUNAS";
pub const LAYER_CHART: &str = "MIN_SM_GRAFICO";

const PALETTE: [&str; 6] = ["#ff6b4a", "#3fb950", "#d29922", "#bc6bff", "#39c5cf", "#ff4aa8"];
const ACI: [u8; 6] = [1, 3, 2, 6, 4, 30];

/// Planes of the discontinuity table (rows with a dip and a dip direction).
pub fn table_planes(t: &GtTable) -> Vec<Plane> {
    t.records
        .iter()
        .filter_map(|r| {
            let (d, a) = (r.num(GtField::Dip)?, r.num(GtField::DipDir)?);
            (0.0..=90.0).contains(&d).then(|| Plane::new(d, a))
        })
        .collect()
}

/// Planes of geological traces that store a dip and a dip direction.
pub fn trace_planes(tags: &[Tag]) -> Vec<Plane> {
    tags.iter()
        .filter(|t| t.class == Class::GeoTrace)
        .filter_map(|t| {
            let d: f64 = t.get("dip")?.replace(',', ".").parse().ok()?;
            let a: f64 = t.get("dipdir")?.replace(',', ".").parse().ok()?;
            (0.0..=90.0).contains(&d).then(|| Plane::new(d, a))
        })
        .collect()
}

/// Contour levels (% per 1 %) for a maximum density.
pub fn levels(max: f64) -> Vec<f64> {
    [1.0, 2.0, 4.0, 6.0, 8.0, 12.0, 16.0, 24.0, 32.0]
        .into_iter()
        .filter(|l| *l < max)
        .collect()
}

fn fmt1(v: f64) -> String {
    format!("{v:.1}")
}

/// The stereonet as SVG: net, contours, poles, set planes (with their
/// names) and an optional face plane (dashed).
pub fn stereonet_svg(poles: &[Plane], sets: &[(String, Plane)], with_contours: bool, face: Option<Plane>) -> String {
    let (c, r) = (150.0, 128.0);
    let pt = |p: [f64; 2]| (c + p[0] * r, c - p[1] * r);
    let mut s =
        String::from(r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 300 300" width="300" height="300">"##);
    s.push_str(&format!(
        r##"<circle cx="{c}" cy="{c}" r="{r}" fill="none" stroke="#888888" stroke-width="1"/>"##
    ));
    s.push_str(&format!(r##"<line x1="{c}" y1="{}" x2="{c}" y2="{}" stroke="#888888"/><text x="{c}" y="{}" font-size="11" fill="#888888" text-anchor="middle">N</text>"##, c - r, c - r - 6.0, c - r - 9.0));
    s.push_str(&format!(r##"<line x1="{}" y1="{c}" x2="{}" y2="{c}" stroke="#88888855"/><line x1="{c}" y1="{}" x2="{c}" y2="{}" stroke="#88888855"/>"##, c - 4.0, c + 4.0, c - 4.0, c + 4.0));
    if with_contours && !poles.is_empty() {
        let v: Vec<Vec3> = poles.iter().map(|p| p.pole().vector()).collect();
        let n = 49;
        let g = density_grid(&v, n);
        let max = g.iter().cloned().filter(|x| x.is_finite()).fold(0.0, f64::max);
        for (k, lv) in levels(max).iter().enumerate() {
            let shade = 40 + (k * 25).min(180);
            for seg in contour(&g, n, *lv) {
                let (a, b) = (pt(seg[0]), pt(seg[1]));
                s.push_str(&format!(r##"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="#4a9eff" stroke-opacity="{:.2}" stroke-width="1"/>"##, a.0, a.1, b.0, b.1, shade as f64 / 255.0));
            }
        }
    }
    for p in poles {
        let q = pt(project(p.pole()));
        s.push_str(&format!(
            r##"<circle cx="{:.1}" cy="{:.1}" r="1.6" fill="#4a9eff"/>"##,
            q.0, q.1
        ));
    }
    let poly = |pts: &[[f64; 2]], color: &str, dash: bool| {
        let d: Vec<String> = pts
            .iter()
            .map(|p| {
                let q = pt(*p);
                format!("{:.1},{:.1}", q.0, q.1)
            })
            .collect();
        format!(
            r##"<polyline points="{}" fill="none" stroke="{color}" stroke-width="1.5"{}/>"##,
            d.join(" "),
            if dash { r##" stroke-dasharray="4 3""## } else { "" }
        )
    };
    for (i, (name, p)) in sets.iter().enumerate() {
        let col = PALETTE[i % PALETTE.len()];
        s.push_str(&poly(&p.great_circle(90), col, false));
        let q = pt(project(p.pole()));
        s.push_str(&format!(r##"<rect x="{:.1}" y="{:.1}" width="6" height="6" fill="{col}"/><text x="{:.1}" y="{:.1}" font-size="10" fill="{col}">{}</text>"##, q.0 - 3.0, q.1 - 3.0, q.0 + 5.0, q.1 - 4.0, esc(name)));
    }
    if let Some(f) = face {
        s.push_str(&poly(&f.great_circle(90), "#ff4a4a", true));
    }
    s.push_str("</svg>");
    s
}

fn esc(t: &str) -> String {
    t.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// The stereonet as 2D entities centred on `centre` (world) with `radius`.
pub fn stereonet_prims(
    poles: &[Plane],
    sets: &[(String, Plane)],
    with_contours: bool,
    centre: Vec3,
    radius: f64,
    origin: LocalOrigin,
) -> Vec<Prim> {
    let tag = || Tag::new(Class::RmDraw).with("kind", "stereonet");
    let w = |p: [f64; 2]| origin.to_local([centre[0] + p[0] * radius, centre[1] + p[1] * radius, centre[2]]);
    let mut out = Vec::new();
    let mut push = |geom: Geom, color: Option<u8>| {
        out.push(Prim {
            geom,
            layer: LAYER_STEREO.into(),
            color,
            tag: tag(),
        })
    };
    let circle: Vec<Vec3> = (0..=72)
        .map(|k| {
            let a = (k as f64 * 5.0).to_radians();
            w([a.sin(), a.cos()])
        })
        .collect();
    push(Geom::Polyline3d(circle), Some(8));
    push(Geom::Line(w([0.0, 1.0]), w([0.0, 1.06])), Some(8));
    push(
        Geom::Text {
            at: w([-0.02, 1.09]),
            height: radius * 0.06,
            value: "N".into(),
        },
        Some(8),
    );
    if with_contours && !poles.is_empty() {
        let v: Vec<Vec3> = poles.iter().map(|p| p.pole().vector()).collect();
        let n = 49;
        let g = density_grid(&v, n);
        let max = g.iter().cloned().filter(|x| x.is_finite()).fold(0.0, f64::max);
        for lv in levels(max) {
            for seg in contour(&g, n, lv) {
                push(Geom::Line(w(seg[0]), w(seg[1])), Some(5));
            }
        }
    }
    let d = 0.012;
    for p in poles {
        let q = project(p.pole());
        push(Geom::Line(w([q[0] - d, q[1]]), w([q[0] + d, q[1]])), Some(5));
        push(Geom::Line(w([q[0], q[1] - d]), w([q[0], q[1] + d])), Some(5));
    }
    for (i, (name, p)) in sets.iter().enumerate() {
        let col = Some(ACI[i % ACI.len()]);
        push(Geom::Polyline3d(p.great_circle(90).into_iter().map(w).collect()), col);
        let q = project(p.pole());
        push(
            Geom::Text {
                at: w([q[0] + 0.03, q[1] + 0.03]),
                height: radius * 0.05,
                value: name.clone(),
            },
            col,
        );
    }
    out
}

/// Point of the face plane each wedge was computed for, with the axis
/// passing through `p` and the floor/centre/crown per `v_align`
/// (0 axis on the crown, 1 centre, 2 floor).
pub fn face_origin(o: &Opening, face: FaceKind, p: Vec3, v_align: usize) -> Vec3 {
    let (_, roof, _) = frame(o, FaceKind::Roof);
    let (_, right, _) = frame(o, FaceKind::RightWall);
    let crown = match v_align {
        0 => 0.0,
        1 => o.height / 2.0,
        _ => o.height,
    };
    match face {
        FaceKind::Roof => add(p, scale(roof, crown)),
        FaceKind::RightWall => add(add(p, scale(roof, crown - o.height / 2.0)), scale(right, o.width / 2.0)),
        FaceKind::LeftWall => add(
            add(p, scale(roof, crown - o.height / 2.0)),
            scale(right, -o.width / 2.0),
        ),
    }
}

/// Wedges as closed meshes (four triangles) on the working.
pub fn wedge_prims(
    o: &Opening,
    wedges: &[Wedge],
    p: Vec3,
    v_align: usize,
    sets: &str,
    origin: LocalOrigin,
) -> Vec<Prim> {
    wedges
        .iter()
        .map(|w| {
            let base = face_origin(o, w.face, p, v_align);
            let v: Vec<Vec3> = w.vertices.iter().map(|q| origin.to_local(add(base, *q))).collect();
            let mut tag = Tag::new(Class::RmWedge)
                .with("face", w.face.key())
                .with("mode", w.mode.key())
                .with("volume", format!("{:.3}", w.volume))
                .with("weight", format!("{:.1}", w.weight))
                .with("sets", sets);
            if let Some(fs) = w.fs {
                tag = tag.with("fs", format!("{fs:.3}"));
            }
            let color = match w.fs {
                Some(f) if f < 1.0 => 1,
                Some(f) if f < 1.5 => 30,
                Some(_) => 3,
                None => 3,
            };
            Prim {
                geom: Geom::Mesh {
                    vertices: v,
                    faces: vec![vec![1, 2, 3], vec![0, 1, 2], vec![0, 1, 3], vec![0, 2, 3]],
                },
                layer: LAYER_WEDGES.into(),
                color: Some(color),
                tag,
            }
        })
        .collect()
}

/// Direction of a polyline at horizontal chainage `ch`: (point, trend, plunge).
pub fn axis_at(points: &[Vec3], ch: f64) -> Option<(Vec3, f64, f64)> {
    let p = geotech_engine::locate::point_at_chainage(points, ch)?;
    let mut acc = 0.0;
    for w in points.windows(2) {
        let seg = (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1]);
        if seg > 1e-9 && (ch <= acc + seg || std::ptr::eq(w, points.windows(2).last()?)) {
            let d = [w[1][0] - w[0][0], w[1][1] - w[0][1], w[1][2] - w[0][2]];
            let l = Line::from_vector(d)?;
            // `from_vector` points down; keep the drawing direction's trend.
            let trend = d[0].atan2(d[1]).to_degrees().rem_euclid(360.0);
            let plunge = if d[2] <= 0.0 { l.plunge } else { -l.plunge };
            return Some((p, trend, plunge));
        }
        acc += seg;
    }
    None
}

/// A design chart kept to be drawn later.
#[derive(Debug, Clone, PartialEq)]
pub struct SavedChart {
    pub title: String,
    pub x_label: String,
    pub y_label: String,
    pub series: Vec<Series>,
}

/// A chart drawn with polylines: frame, the series scaled into it, title and
/// axis ranges. `corner` is the lower-left corner (world), `width` its size.
pub fn chart_prims(chart: &SavedChart, corner: Vec3, width: f64, kind: &str, origin: LocalOrigin) -> Vec<Prim> {
    let (title, x_label, y_label, series) = (
        chart.title.as_str(),
        chart.x_label.as_str(),
        chart.y_label.as_str(),
        &chart.series,
    );
    let h = width * 0.6;
    let pts: Vec<(f64, f64)> = series
        .iter()
        .flat_map(|s| s.points.iter().copied())
        .filter(|p| p.0.is_finite() && p.1.is_finite())
        .collect();
    if pts.is_empty() {
        return Vec::new();
    }
    let (x0, x1) = pts
        .iter()
        .fold((f64::MAX, f64::MIN), |a, p| (a.0.min(p.0), a.1.max(p.0)));
    let (y0, y1) = pts
        .iter()
        .fold((f64::MAX, f64::MIN), |a, p| (a.0.min(p.1), a.1.max(p.1)));
    let (y0, y1) = (y0.min(0.0), if y1 > y0 { y1 } else { y0 + 1.0 });
    let x1 = if x1 > x0 { x1 } else { x0 + 1.0 };
    let map = |x: f64, y: f64| {
        origin.to_local([
            corner[0] + (x - x0) / (x1 - x0) * width,
            corner[1] + (y - y0) / (y1 - y0) * h,
            corner[2],
        ])
    };
    let tag = || Tag::new(Class::RmDraw).with("kind", kind);
    let th = width * 0.025;
    let mut out = vec![Prim {
        geom: Geom::Polygon3d(vec![map(x0, y0), map(x1, y0), map(x1, y1), map(x0, y1)]),
        layer: LAYER_CHART.into(),
        color: Some(8),
        tag: tag(),
    }];
    let text = |at: Vec3, v: String| Prim {
        geom: Geom::Text {
            at,
            height: th,
            value: v,
        },
        layer: LAYER_CHART.into(),
        color: Some(7),
        tag: tag(),
    };
    out.push(text(map(x0, y1 + (y1 - y0) * 0.05), title.to_string()));
    out.push(text(
        map(x0, y0 - (y1 - y0) * 0.08),
        format!("{x_label}: {} – {}", fmt1(x0), fmt1(x1)),
    ));
    out.push(text(
        map(x0, y0 - (y1 - y0) * 0.16),
        format!("{y_label}: {} – {}", fmt1(y0), fmt1(y1)),
    ));
    for (i, s) in series.iter().enumerate() {
        let line: Vec<Vec3> = s
            .points
            .iter()
            .filter(|p| p.0.is_finite() && p.1.is_finite())
            .map(|p| map(p.0, p.1))
            .collect();
        if line.len() >= 2 {
            out.push(Prim {
                geom: Geom::Polyline3d(line),
                layer: LAYER_CHART.into(),
                color: Some(ACI[i % ACI.len()]),
                tag: tag().with("series", &s.name),
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use geotech_engine::wedge::Joint;

    #[test]
    fn svg_and_prims_of_a_stereonet() {
        let poles: Vec<Plane> = (0..30)
            .map(|k| Plane::new(70.0 + (k % 3) as f64, 40.0 + (k % 5) as f64))
            .collect();
        let sets = vec![("J1".to_string(), Plane::new(71.0, 42.0))];
        let svg = stereonet_svg(&poles, &sets, true, Some(Plane::new(60.0, 90.0)));
        assert!(svg.starts_with("<svg") && svg.contains("J1") && svg.contains("stroke-dasharray"));
        assert_eq!(svg.matches("r=\"1.6\"").count(), 30);
        let prims = stereonet_prims(&poles, &sets, false, [100.0, 200.0, 0.0], 10.0, LocalOrigin::default());
        // Net + N tick + N text + 2 lines per pole + set circle + set label.
        assert_eq!(prims.len(), 3 + 60 + 2);
        assert!(prims
            .iter()
            .all(|p| p.tag.class == Class::RmDraw && p.layer == LAYER_STEREO));
    }

    #[test]
    fn planes_from_tables_and_traces() {
        use geotech_engine::data::{GtKind, GtRecord};
        let t = GtTable {
            kind: GtKind::Joints,
            records: vec![
                GtRecord::default().with(GtField::Dip, 60).with(GtField::DipDir, 120),
                GtRecord::default().with(GtField::Dip, 95).with(GtField::DipDir, 10),
                GtRecord::default().with(GtField::Dip, 30),
            ],
            mapping: vec![],
            unmapped: vec![],
        };
        assert_eq!(table_planes(&t), vec![Plane::new(60.0, 120.0)]);
        let tags = vec![
            Tag::new(Class::GeoTrace).with("dip", "65").with("dipdir", "135.000"),
            Tag::new(Class::GeoTrace),
            Tag::new(Class::GeoProj).with("dip", "40").with("dipdir", "10"),
        ];
        assert_eq!(trace_planes(&tags), vec![Plane::new(65.0, 135.0)]);
    }

    #[test]
    fn wedges_are_placed_on_their_faces() {
        let o = Opening {
            axis: Line {
                trend: 0.0,
                plunge: 0.0,
            },
            width: 4.0,
            height: 3.0,
            max_length: 0.0,
        };
        let joints = [
            Joint {
                plane: Plane::new(70.0, 10.0),
                friction: 30.0,
                cohesion: 0.0,
            },
            Joint {
                plane: Plane::new(65.0, 130.0),
                friction: 30.0,
                cohesion: 0.0,
            },
            Joint {
                plane: Plane::new(60.0, 250.0),
                friction: 30.0,
                cohesion: 0.0,
            },
        ];
        let w = geotech_engine::wedge::analyse(&o, &joints, 27.0, 0.0);
        let prims = wedge_prims(&o, &w, [0.0, 0.0, 100.0], 2, "J1,J2,J3", LocalOrigin::default());
        assert_eq!(prims.len(), 3);
        // Roof wedge: base on the crown (z = 103), apex above it.
        if let Geom::Mesh { vertices, .. } = &prims[0].geom {
            assert!(vertices[1..].iter().all(|v| (v[2] - 103.0).abs() < 1e-9));
            assert!(vertices[0][2] > 103.0);
        } else {
            panic!("mesh expected");
        }
        // Right wall wedge base on x = +2.
        if let Geom::Mesh { vertices, .. } = &prims[2].geom {
            assert!(vertices[1..].iter().all(|v| (v[0] - 2.0).abs() < 1e-9), "{vertices:?}");
        }
        assert_eq!(prims[0].tag.get("face"), Some("roof"));
    }

    #[test]
    fn axis_direction_and_charts() {
        let pts = vec![[0.0, 0.0, 0.0], [0.0, 10.0, -1.0], [10.0, 10.0, -1.0]];
        let (p, t, pl) = axis_at(&pts, 5.0).unwrap();
        assert!((p[1] - 5.0).abs() < 1e-9 && t.abs() < 1e-9 && pl > 0.0);
        let (_, t2, pl2) = axis_at(&pts, 15.0).unwrap();
        assert!((t2 - 90.0).abs() < 1e-9 && pl2.abs() < 1e-9);
        let s = vec![Series::new(
            "a",
            vec![(0.0, 0.0), (10.0, 5.0)],
            crate::charts::Style::Line,
            0,
        )];
        let c = SavedChart {
            title: "t".into(),
            x_label: "x".into(),
            y_label: "y".into(),
            series: s,
        };
        let prims = chart_prims(&c, [0.0, 0.0, 0.0], 100.0, "chart_kirsch", LocalOrigin::default());
        assert_eq!(prims.len(), 5);
        let empty = SavedChart { series: vec![], ..c };
        assert!(chart_prims(&empty, [0.0; 3], 100.0, "k", LocalOrigin::default()).is_empty());
    }
}
