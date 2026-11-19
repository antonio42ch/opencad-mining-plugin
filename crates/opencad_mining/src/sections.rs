//! Cross-sections: cut meshes with a set of planes, draw the 3D section
//! polylines in place and/or 2D sheets on a grid.

use mining_core::Vec3;
use solids_engine::cut::{
    cut, level_planes, plane_3_points, planes_along_axis, vertical_planes, LabeledPlane, MeshData, SectionLoop,
};
use solids_engine::sheet::{layout, Role, SheetConfig, SheetGeom, SheetInput};

use crate::cad::{CadSink, Geom, LayerDef, Prim};
use crate::config::SectionsConfig;
use crate::i18n::{fmt, Strings};
use crate::xdata::{Class, Tag};

pub const LAYER_CUT: &str = "MIN_SC_CORTE";
pub const LAYER_SHEET: &str = "MIN_SC_LAMINA";
pub const LAYER_GRID: &str = "MIN_SC_GRILLA";

/// Inputs picked in the drawing for the planes.
pub struct PlaneInput<'a> {
    pub axis: Option<&'a [Vec3]>,
    pub points: &'a [Vec3],
}

fn bounds(meshes: &[MeshData]) -> Option<(Vec3, Vec3)> {
    meshes.iter().filter_map(|m| m.bounds()).reduce(|(a, b), (c, d)| {
        (
            [a[0].min(c[0]), a[1].min(c[1]), a[2].min(c[2])],
            [b[0].max(d[0]), b[1].max(d[1]), b[2].max(d[2])],
        )
    })
}

/// Builds the planes and their sheet titles for the configured mode.
pub fn planes(
    s: &Strings,
    cfg: &SectionsConfig,
    input: &PlaneInput,
    meshes: &[MeshData],
) -> Result<Vec<(LabeledPlane, String)>, String> {
    let err = |e: solids_engine::cut::PlaneError| e.to_string();
    let titled = |v: Vec<LabeledPlane>, f: &dyn Fn(usize, &LabeledPlane) -> String| {
        v.into_iter()
            .enumerate()
            .map(|(i, p)| (f(i, &p), p))
            .map(|(t, p)| (p, t))
            .collect()
    };
    match cfg.mode {
        0 => {
            let axis = input.axis.ok_or("axis missing")?;
            let v = planes_along_axis(axis, cfg.interval, cfg.ch_from, cfg.ch_to).map_err(err)?;
            Ok(titled(v, &|_, p| fmt(s.title_chainage, &[&p.label])))
        }
        1 => {
            let [p1, p2] = [input.points.first(), input.points.get(1)];
            let (p1, p2) = (p1.ok_or("point missing")?, p2.ok_or("point missing")?);
            let v = vertical_planes(*p1, *p2, cfg.spacing, cfg.count.round().max(1.0) as usize).map_err(err)?;
            Ok(titled(v, &|i, p| fmt(s.title_offset, &[&(i + 1), &p.label])))
        }
        2 => {
            let (mut lo, mut hi) = (cfg.z_from, cfg.z_to);
            if hi <= lo {
                // Automatic: every whole step inside the solids' Z range.
                let (a, b) = bounds(meshes).ok_or("no geometry")?;
                let st = if cfg.z_step > 0.0 { cfg.z_step } else { 1.0 };
                lo = (a[2] / st).ceil() * st;
                hi = (b[2] / st).floor() * st;
            }
            let v = level_planes(lo, hi, cfg.z_step).map_err(err)?;
            Ok(titled(v, &|_, p| fmt(s.title_level, &[&p.label])))
        }
        _ => {
            let pts = input.points;
            if pts.len() < 3 {
                return Err("3 points required".into());
            }
            Ok(vec![(
                plane_3_points(pts[0], pts[1], pts[2]).map_err(err)?,
                s.title_plane.to_string(),
            )])
        }
    }
}

pub struct Report {
    pub planes: usize,
    pub with_hits: usize,
    pub loops: usize,
}

/// Cuts and builds every entity (pure, testable).
pub fn scene(
    s: &Strings,
    cfg: &SectionsConfig,
    meshes: &[MeshData],
    planes: &[(LabeledPlane, String)],
    set: &str,
) -> (Vec<Prim>, Report) {
    let cuts: Vec<Vec<SectionLoop>> = planes.iter().map(|(p, _)| cut(meshes, &p.plane)).collect();
    let mut prims = Vec::new();
    let mut report = Report {
        planes: planes.len(),
        with_hits: 0,
        loops: 0,
    };
    for ((_, title), loops) in planes.iter().zip(&cuts) {
        if !loops.is_empty() {
            report.with_hits += 1;
            report.loops += loops.len();
        }
        if cfg.draw_3d {
            for l in loops {
                prims.push(Prim {
                    geom: if l.closed {
                        Geom::Polygon3d(l.points.clone())
                    } else {
                        Geom::Polyline3d(l.points.clone())
                    },
                    layer: LAYER_CUT.into(),
                    color: None,
                    tag: Tag::new(Class::Cut)
                        .with("set", set)
                        .with("plane", title)
                        .with("area", format!("{:.3}", l.area())),
                });
            }
        }
    }
    if cfg.draw_sheets {
        let g = if cfg.grid > 0.0 { cfg.grid } else { 1.0 };
        let origin = match (cfg.auto_place, bounds(meshes)) {
            (true, Some((_, hi))) => [hi[0] + 5.0 * g + cfg.gap, hi[1]],
            _ => [cfg.sheet_x, cfg.sheet_y],
        };
        let sc = SheetConfig {
            origin,
            columns: cfg.columns.round().max(1.0) as usize,
            gap: cfg.gap.max(0.0),
            grid: g,
            text_height: cfg.text_height,
        };
        let inputs: Vec<SheetInput> = planes
            .iter()
            .zip(&cuts)
            .map(|((p, title), loops)| SheetInput {
                title,
                loops,
                marker: p.marker.map(|m| p.plane.to_2d(m)),
            })
            .collect();
        let area = |a: f64| fmt(s.area_fmt, &[&format!("{a:.2}")]);
        for item in layout(&inputs, &sc, &area) {
            let title = &planes[item.sheet].1;
            let (layer, role) = match item.role {
                Role::Grid | Role::GridLabel => {
                    (LAYER_GRID, if item.role == Role::Grid { "grid" } else { "grid_label" })
                }
                Role::Frame => (LAYER_SHEET, "frame"),
                Role::Title => (LAYER_SHEET, "title"),
                Role::Area => (LAYER_SHEET, "area"),
                Role::Section => (LAYER_SHEET, "section"),
                Role::Marker => (LAYER_SHEET, "marker"),
            };
            let color = match item.role {
                Role::Section => Some(1),
                Role::Marker => Some(6),
                _ => None,
            };
            let geom = match item.geom {
                SheetGeom::Polyline { points, closed } => {
                    let p3: Vec<Vec3> = points.iter().map(|p| [p[0], p[1], 0.0]).collect();
                    if closed {
                        Geom::Polygon3d(p3)
                    } else {
                        Geom::Polyline3d(p3)
                    }
                }
                SheetGeom::Text { at, height, value } => Geom::Text {
                    at: [at[0], at[1], 0.0],
                    height,
                    value,
                },
            };
            prims.push(Prim {
                geom,
                layer: layer.into(),
                color,
                tag: Tag::new(Class::Sheet)
                    .with("set", set)
                    .with("plane", title)
                    .with("role", role),
            });
        }
    }
    (prims, report)
}

/// Full run: replace previous sections when asked, draw, report.
pub fn run(
    sink: &mut dyn CadSink,
    s: &Strings,
    cfg: &SectionsConfig,
    meshes: &[MeshData],
    input: &PlaneInput,
) -> Option<Report> {
    if meshes.is_empty() {
        sink.error(s.m_no_meshes);
        return None;
    }
    let planes = match planes(s, cfg, input, meshes) {
        Ok(p) => p,
        Err(e) => {
            sink.error(&e);
            return None;
        }
    };
    let set = crate::axes::new_id();
    let (prims, report) = scene(s, cfg, meshes, &planes, &set);
    sink.begin("MIN_SC_RUN");
    sink.ensure_layers(&[
        LayerDef::new(LAYER_CUT, 1),
        LayerDef::new(LAYER_SHEET, 7),
        LayerDef::new(LAYER_GRID, 8),
    ]);
    if cfg.replace {
        sink.remove_where(&|t: &Tag| matches!(t.class, Class::Cut | Class::Sheet));
    }
    sink.add(prims);
    sink.end();
    sink.info(&fmt(
        s.m_sections_done,
        &[&report.planes, &report.with_hits, &report.loops],
    ));
    Some(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cad::mock::MockSink;
    use crate::i18n::{for_lang, Lang};
    use solids_engine::axis::{axis_rings, AxisParams};
    use solids_engine::mesh::ClosedMesh;
    use solids_engine::section::Profile;

    fn gallery() -> (Vec<Vec3>, MeshData) {
        let axis = vec![[0.0, 0.0, 100.0], [0.0, 30.0, 100.0]];
        let prof = Profile::Rectangular {
            width: 4.0,
            height: 3.0,
        };
        let rings = axis_rings(&AxisParams {
            axis: &axis,
            profile: &prof,
            interval: 2.0,
            offset: [0.0, 0.0],
        })
        .unwrap();
        let m = ClosedMesh::from_rings(&rings).unwrap();
        (
            axis,
            MeshData {
                vertices: m.vertices,
                faces: m.faces,
            },
        )
    }

    #[test]
    fn perpendicular_sections_with_sheets_replace_previous_run() {
        let s = for_lang(Lang::Es);
        let (axis, mesh) = gallery();
        let cfg = SectionsConfig {
            interval: 10.0,
            ..Default::default()
        };
        let mut sink = MockSink::default();
        let input = PlaneInput {
            axis: Some(&axis),
            points: &[],
        };
        let r = run(&mut sink, s, &cfg, std::slice::from_ref(&mesh), &input).unwrap();
        assert_eq!((r.planes, r.with_hits, r.loops), (4, 4, 4));
        let cuts: Vec<_> = sink.entities.iter().filter(|p| p.tag.class == Class::Cut).collect();
        assert_eq!(cuts.len(), 4);
        assert!(cuts.iter().all(|c| c.tag.get("area") == Some("12.000")));
        assert_eq!(cuts[1].tag.get("plane"), Some("PK 0+010.00"));
        let first = sink.entities.len();
        run(&mut sink, s, &cfg, &[mesh], &input).unwrap();
        assert_eq!(sink.entities.len(), first, "previous sections replaced");
        assert_eq!(sink.undo_depth, 0);
        // Sheets lie at z = 0, to the right of the solid.
        let frame = sink
            .entities
            .iter()
            .find(|p| p.tag.get("role") == Some("frame"))
            .unwrap();
        let Geom::Polygon3d(pts) = &frame.geom else { panic!() };
        assert!(pts.iter().all(|p| p[2] == 0.0 && p[0] > 2.0));
    }

    #[test]
    fn automatic_levels_and_errors() {
        let s = for_lang(Lang::En);
        let (_, mesh) = gallery();
        let cfg = SectionsConfig {
            mode: 2,
            z_step: 1.0,
            draw_sheets: false,
            ..Default::default()
        };
        let mut sink = MockSink::default();
        let r = run(
            &mut sink,
            s,
            &cfg,
            std::slice::from_ref(&mesh),
            &PlaneInput {
                axis: None,
                points: &[],
            },
        )
        .unwrap();
        // Solid spans z 98.5..101.5 -> levels 99, 100, 101.
        assert_eq!((r.planes, r.with_hits), (3, 3));
        assert!(run(
            &mut sink,
            s,
            &cfg,
            &[],
            &PlaneInput {
                axis: None,
                points: &[]
            }
        )
        .is_none());
        let cfg3 = SectionsConfig {
            mode: 3,
            ..Default::default()
        };
        assert!(run(
            &mut sink,
            s,
            &cfg3,
            &[mesh],
            &PlaneInput {
                axis: None,
                points: &[[0.0; 3]]
            }
        )
        .is_none());
    }
}
