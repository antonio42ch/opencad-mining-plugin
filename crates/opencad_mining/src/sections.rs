//! Cross-sections: cut meshes with a set of planes, draw the 3D section
//! polylines in place and/or 2D sheets on a grid.

use mining_core::{Vec2, Vec3};
use solids_engine::clip::{clip, Window};
use solids_engine::cut::{
    cut, level_planes, plane_3_points, planes_along_axis, vertical_planes, LabeledPlane, MeshData, Plane, SectionLoop,
};
use solids_engine::sheet::{layout, Role, SheetConfig, SheetGeom, SheetInput};

use crate::cad::{CadSink, Geom, LayerDef, Prim};
use crate::config::SectionsConfig;
use crate::i18n::{fmt, Strings};
use crate::xdata::{Class, Tag};

pub const LAYER_CUT: &str = "MIN_SC_CORTE";
pub const LAYER_SHEET: &str = "MIN_SC_LAMINA";
pub const LAYER_GRID: &str = "MIN_SC_GRILLA";
pub const LAYER_TERRAIN: &str = "MIN_SC_TERRENO";
/// ACI colour of the terrain (green).
const TERRAIN_COLOR: u8 = 3;

/// A terrain to cut with the same planes as the solids.
pub struct TerrainInput<'a> {
    pub name: &'a str,
    pub mesh: &'a MeshData,
}

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
    pub terrain: Option<TerrainReport>,
}

/// What the terrain gave: planes it crosses, polylines and the elevation range of the profiles.
#[derive(Debug, Clone, PartialEq)]
pub struct TerrainReport {
    pub planes_cut: usize,
    pub polylines: usize,
    pub min_elevation: f64,
    pub max_elevation: f64,
    /// Titles of the planes the terrain does not cross within the window.
    pub missed: Vec<String>,
}

/// The window of a plane in which the terrain is drawn: the extent of the solids in that plane
/// plus the margin; vertical planes are limited only along the section.
fn terrain_window(plane: &Plane, loops: &[SectionLoop], meshes: &[MeshData], cfg: &SectionsConfig) -> Window {
    let horizontal = plane.normal[2].abs() > 0.999;
    let extent: Option<(Vec2, Vec2)> = if !loops.is_empty() {
        let pts = loops.iter().flat_map(|l| l.points_2d.iter().copied());
        Some(pts.fold(([f64::MAX; 2], [f64::MIN; 2]), |(lo, hi), p| {
            ([lo[0].min(p[0]), lo[1].min(p[1])], [hi[0].max(p[0]), hi[1].max(p[1])])
        }))
    } else {
        bounds(meshes).map(|(a, b)| {
            let corners = [a[0], b[0]].into_iter().flat_map(|x| {
                [a[1], b[1]]
                    .into_iter()
                    .flat_map(move |y| [a[2], b[2]].into_iter().map(move |z| [x, y, z]))
            });
            corners
                .map(|c| plane.to_2d(c))
                .fold(([f64::MAX; 2], [f64::MIN; 2]), |(lo, hi), p| {
                    ([lo[0].min(p[0]), lo[1].min(p[1])], [hi[0].max(p[0]), hi[1].max(p[1])])
                })
        })
    };
    let m = cfg.terrain_margin.max(0.0);
    match extent {
        Some((lo, hi)) => Window {
            x: Some((lo[0] - m, hi[0] + m)),
            y: horizontal.then_some((lo[1] - m, hi[1] + m)),
        },
        // No solids to size it: a fixed width along a vertical section, the whole terrain in a plan.
        None if horizontal => Window::default(),
        None => Window {
            x: Some((-cfg.terrain_width.max(1.0), cfg.terrain_width.max(1.0))),
            y: None,
        },
    }
}

/// Cuts and builds every entity (pure, testable).
pub fn scene(
    s: &Strings,
    cfg: &SectionsConfig,
    meshes: &[MeshData],
    planes: &[(LabeledPlane, String)],
    set: &str,
    terrain: Option<&TerrainInput>,
) -> (Vec<Prim>, Report) {
    let cuts: Vec<Vec<SectionLoop>> = planes.iter().map(|(p, _)| cut(meshes, &p.plane)).collect();
    // The terrain cut by the same planes, limited to the window around the solids.
    let tcuts: Vec<Vec<SectionLoop>> = match terrain {
        Some(t) => planes
            .iter()
            .zip(&cuts)
            .map(|((p, _), loops)| {
                let raw = cut(std::slice::from_ref(t.mesh), &p.plane);
                clip(&raw, &p.plane, &terrain_window(&p.plane, loops, meshes, cfg))
            })
            .collect(),
        None => vec![Vec::new(); planes.len()],
    };
    let mut prims = Vec::new();
    let mut report = Report {
        planes: planes.len(),
        with_hits: 0,
        loops: 0,
        terrain: terrain.map(|_| TerrainReport {
            planes_cut: 0,
            polylines: 0,
            min_elevation: f64::NAN,
            max_elevation: f64::NAN,
            missed: Vec::new(),
        }),
    };
    for ((_, title), tl) in planes.iter().zip(&tcuts) {
        let Some(tr) = report.terrain.as_mut() else { break };
        if tl.is_empty() {
            tr.missed.push(title.clone());
            continue;
        }
        tr.planes_cut += 1;
        tr.polylines += tl.len();
        for z in tl.iter().flat_map(|l| l.points.iter().map(|p| p[2])) {
            tr.min_elevation = if tr.min_elevation.is_nan() {
                z
            } else {
                tr.min_elevation.min(z)
            };
            tr.max_elevation = if tr.max_elevation.is_nan() {
                z
            } else {
                tr.max_elevation.max(z)
            };
        }
        if cfg.draw_3d {
            for l in tl {
                prims.push(Prim {
                    geom: if l.closed {
                        Geom::Polygon3d(l.points.clone())
                    } else {
                        Geom::Polyline3d(l.points.clone())
                    },
                    layer: LAYER_TERRAIN.into(),
                    color: Some(TERRAIN_COLOR),
                    tag: Tag::new(Class::Cut)
                        .with("set", set)
                        .with("plane", title)
                        .with("kind", "terrain")
                        .with("terrain", terrain.map_or("", |t| t.name)),
                });
            }
        }
    }
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
        let placing = if meshes.is_empty() {
            terrain.map(|t| std::slice::from_ref(t.mesh)).unwrap_or(meshes)
        } else {
            meshes
        };
        let origin = match (cfg.auto_place, bounds(placing)) {
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
            .zip(&tcuts)
            .map(|(((p, title), loops), tl)| SheetInput {
                title,
                loops,
                marker: p.marker.map(|m| p.plane.to_2d(m)),
                terrain: tl,
            })
            .collect();
        let area = |a: f64| fmt(s.area_fmt, &[&format!("{a:.2}")]);
        let terrain_label =
            |lo: f64, hi: f64| fmt(crate::terrain::t().label, &[&format!("{lo:.1}"), &format!("{hi:.1}")]);
        for item in layout(&inputs, &sc, &area, &terrain_label) {
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
                Role::Terrain => (LAYER_TERRAIN, "terrain"),
                Role::TerrainLabel => (LAYER_TERRAIN, "terrain_label"),
            };
            let color = match item.role {
                Role::Section => Some(1),
                Role::Marker => Some(6),
                Role::Terrain | Role::TerrainLabel => Some(TERRAIN_COLOR),
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
    terrain: Option<&TerrainInput>,
) -> Option<Report> {
    if meshes.is_empty() && terrain.is_none() {
        sink.error(s.m_no_meshes);
        return None;
    }
    // Without solids the terrain sizes the planes (automatic levels).
    let sizing = if meshes.is_empty() {
        terrain.map(|t| std::slice::from_ref(t.mesh)).unwrap_or(meshes)
    } else {
        meshes
    };
    let planes = match planes(s, cfg, input, sizing) {
        Ok(p) => p,
        Err(e) => {
            sink.error(&e);
            return None;
        }
    };
    let set = crate::axes::new_id();
    let (prims, report) = scene(s, cfg, meshes, &planes, &set, terrain);
    sink.begin("MIN_SC_RUN");
    sink.ensure_layers(&[
        LayerDef::new(LAYER_CUT, 1),
        LayerDef::new(LAYER_SHEET, 7),
        LayerDef::new(LAYER_GRID, 8),
        LayerDef::new(LAYER_TERRAIN, TERRAIN_COLOR),
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
    if let (Some(t), Some(tr)) = (terrain, &report.terrain) {
        let (lo, hi) = (format!("{:.1}", tr.min_elevation), format!("{:.1}", tr.max_elevation));
        sink.info(&fmt(
            crate::terrain::t().m_profile,
            &[&t.name, &tr.planes_cut, &tr.polylines, &lo, &hi],
        ));
        if !tr.missed.is_empty() {
            let list: Vec<&str> = tr.missed.iter().take(8).map(String::as_str).collect();
            sink.info(&fmt(crate::terrain::t().m_missed, &[&list.join(", ")]));
        }
    }
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
        let r = run(&mut sink, s, &cfg, std::slice::from_ref(&mesh), &input, None).unwrap();
        assert_eq!((r.planes, r.with_hits, r.loops), (4, 4, 4));
        let cuts: Vec<_> = sink.entities.iter().filter(|p| p.tag.class == Class::Cut).collect();
        assert_eq!(cuts.len(), 4);
        assert!(cuts.iter().all(|c| c.tag.get("area") == Some("12.000")));
        assert_eq!(cuts[1].tag.get("plane"), Some("PK 0+010.00"));
        let first = sink.entities.len();
        run(&mut sink, s, &cfg, &[mesh], &input, None).unwrap();
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
            None,
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
            },
            None
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
            },
            None
        )
        .is_none());
    }

    /// A terrain grid `x0..x1` by `y0..y1` every `step`, with z = f(x, y).
    fn terrain(x0: f64, x1: f64, y0: f64, y1: f64, step: f64, f: &dyn Fn(f64, f64) -> f64) -> MeshData {
        let (nx, ny) = (((x1 - x0) / step) as usize + 1, ((y1 - y0) / step) as usize + 1);
        let mut vertices = Vec::new();
        for j in 0..ny {
            for i in 0..nx {
                let (x, y) = (x0 + i as f64 * step, y0 + j as f64 * step);
                vertices.push([x, y, f(x, y)]);
            }
        }
        let mut faces = Vec::new();
        for j in 0..ny - 1 {
            for i in 0..nx - 1 {
                let a = j * nx + i;
                faces.push(vec![a, a + 1, a + nx + 1]);
                faces.push(vec![a, a + nx + 1, a + nx]);
            }
        }
        MeshData { vertices, faces }
    }

    #[test]
    fn the_terrain_profile_is_limited_to_the_solids_plus_the_margin() {
        let s = for_lang(Lang::Es);
        let (axis, mesh) = gallery();
        // A slope rising 0.1 m per m to the east, 30 m above the gallery roof, much wider than the gallery.
        let ground = terrain(-100.0, 100.0, -20.0, 60.0, 5.0, &|x, _| 130.0 + 0.1 * x);
        let cfg = SectionsConfig {
            interval: 10.0,
            terrain_margin: 5.0,
            ..Default::default()
        };
        let mut sink = MockSink::default();
        let input = PlaneInput {
            axis: Some(&axis),
            points: &[],
        };
        let t = TerrainInput {
            name: "Topografía",
            mesh: &ground,
        };
        let r = run(&mut sink, s, &cfg, std::slice::from_ref(&mesh), &input, Some(&t)).unwrap();
        let tr = r.terrain.clone().unwrap();
        assert_eq!((tr.planes_cut, tr.polylines), (4, 4));
        assert!(tr.missed.is_empty());
        // Profile heights follow the slope inside the window (gallery 4 m wide + 5 m each side -> x from -7 to 7).
        assert!(
            (tr.min_elevation - 129.3).abs() < 0.05 && (tr.max_elevation - 130.7).abs() < 0.05,
            "{tr:?}"
        );
        let lines: Vec<_> = sink
            .entities
            .iter()
            .filter(|p| p.layer == LAYER_TERRAIN && p.tag.class == Class::Cut)
            .collect();
        assert_eq!(lines.len(), 4);
        for l in &lines {
            let Geom::Polyline3d(pts) = &l.geom else {
                panic!("a profile is an open polyline")
            };
            assert!(pts.iter().all(|p| p[0] >= -7.0 - 1e-6 && p[0] <= 7.0 + 1e-6), "{pts:?}");
            assert_eq!(l.tag.get("kind"), Some("terrain"));
            assert_eq!(l.tag.get("terrain"), Some("Topografía"));
        }
        // The sheets draw the profile and its elevation range, on the terrain layer.
        let in_sheets: Vec<_> = sink
            .entities
            .iter()
            .filter(|p| p.tag.class == Class::Sheet && p.layer == LAYER_TERRAIN)
            .collect();
        assert_eq!(
            in_sheets
                .iter()
                .filter(|p| p.tag.get("role") == Some("terrain"))
                .count(),
            4
        );
        let label = in_sheets
            .iter()
            .find(|p| p.tag.get("role") == Some("terrain_label"))
            .unwrap();
        let Geom::Text { value, .. } = &label.geom else {
            panic!()
        };
        assert!(value.contains("129.3") && value.contains("130.7"), "{value}");
        // Running again replaces the terrain lines too.
        let n = sink.entities.len();
        run(&mut sink, s, &cfg, &[mesh], &input, Some(&t)).unwrap();
        assert_eq!(sink.entities.len(), n);
    }

    #[test]
    fn a_wider_margin_gives_a_longer_profile_and_without_terrain_nothing_changes() {
        let s = for_lang(Lang::En);
        let (axis, mesh) = gallery();
        let ground = terrain(-100.0, 100.0, -20.0, 60.0, 5.0, &|x, _| 130.0 + 0.1 * x);
        let input = PlaneInput {
            axis: Some(&axis),
            points: &[],
        };
        let t = TerrainInput {
            name: "T",
            mesh: &ground,
        };
        let width = |margin: f64| {
            let cfg = SectionsConfig {
                interval: 10.0,
                terrain_margin: margin,
                draw_sheets: false,
                ..Default::default()
            };
            let mut sink = MockSink::default();
            run(&mut sink, s, &cfg, std::slice::from_ref(&mesh), &input, Some(&t)).unwrap();
            let line = sink.entities.iter().find(|p| p.layer == LAYER_TERRAIN).unwrap();
            let Geom::Polyline3d(pts) = &line.geom else { panic!() };
            let xs: Vec<f64> = pts.iter().map(|p| p[0]).collect();
            xs.iter().cloned().fold(f64::MIN, f64::max) - xs.iter().cloned().fold(f64::MAX, f64::min)
        };
        assert!(
            (width(5.0) - 14.0).abs() < 1e-6 && (width(20.0) - 44.0).abs() < 1e-6,
            "{} {}",
            width(5.0),
            width(20.0)
        );
        // No terrain: no terrain entities and no report.
        let mut sink = MockSink::default();
        let r = run(&mut sink, s, &SectionsConfig::default(), &[mesh], &input, None).unwrap();
        assert!(r.terrain.is_none() && sink.entities.iter().all(|p| p.layer != LAYER_TERRAIN));
    }

    #[test]
    fn a_terrain_alone_gives_profiles_and_contours_and_reports_the_planes_it_misses() {
        let s = for_lang(Lang::En);
        // Two points 100 m apart; terrain z = 100 + 0.1 x over x 0..100, y 0..50.
        let ground = terrain(0.0, 100.0, 0.0, 50.0, 10.0, &|x, _| 100.0 + 0.1 * x);
        let t = TerrainInput {
            name: "Slope",
            mesh: &ground,
        };
        let none = PlaneInput {
            axis: None,
            points: &[[0.0, 25.0, 0.0], [100.0, 25.0, 0.0]],
        };
        let cfg = SectionsConfig {
            mode: 1,
            count: 1.0,
            terrain_width: 200.0,
            ..Default::default()
        };
        let mut sink = MockSink::default();
        let r = run(&mut sink, s, &cfg, &[], &none, Some(&t)).unwrap();
        let tr = r.terrain.unwrap();
        assert_eq!((r.planes, r.with_hits, r.loops), (1, 0, 0));
        assert_eq!((tr.planes_cut, tr.polylines), (1, 1));
        assert!(
            (tr.min_elevation - 100.0).abs() < 1e-6 && (tr.max_elevation - 110.0).abs() < 1e-6,
            "{tr:?}"
        );
        // Level plans: the levels come from the terrain's elevation range; every level inside gives a contour.
        let cfg = SectionsConfig {
            mode: 2,
            z_step: 2.0,
            draw_sheets: false,
            ..Default::default()
        };
        let mut sink = MockSink::default();
        let r = run(
            &mut sink,
            s,
            &cfg,
            &[],
            &PlaneInput {
                axis: None,
                points: &[],
            },
            Some(&t),
        )
        .unwrap();
        let tr = r.terrain.unwrap();
        assert_eq!(r.planes, 6, "100, 102, ... 110");
        assert!(tr.planes_cut >= 4, "{tr:?}");
        // A contour at z = 104 runs along x = 40 for the whole 50 m.
        let contours: Vec<_> = sink.entities.iter().filter(|p| p.layer == LAYER_TERRAIN).collect();
        assert!(contours.iter().any(|c| {
            let Geom::Polyline3d(pts) = &c.geom else { return false };
            pts.iter()
                .all(|p| (p[0] - 40.0).abs() < 1e-6 && (p[2] - 104.0).abs() < 1e-6)
                && (pts[0][1] - pts.last().unwrap()[1]).abs() > 49.0
        }));
    }

    #[test]
    fn without_solids_or_terrain_there_is_nothing_to_cut() {
        let s = for_lang(Lang::En);
        let mut sink = MockSink::default();
        let cfg = SectionsConfig {
            mode: 2,
            ..Default::default()
        };
        assert!(run(
            &mut sink,
            s,
            &cfg,
            &[],
            &PlaneInput {
                axis: None,
                points: &[]
            },
            None
        )
        .is_none());
    }
}
