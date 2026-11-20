//! Open Mining — entry point of the `cdylib` the host loads.
//!
//! Layering: `mining_core`, `drillhole_engine`, `solids_engine` and
//! `geomap_engine` are host-agnostic engines. This crate only adapts them to
//! the Open CAD Studio contract. Commands draw through [`cad::CadSink`], so
//! their logic is tested against an in-memory sink.

// `export_plugin!` (host macro) emits a C entry point taking a raw pointer.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

pub mod axes;
pub mod cad;
pub mod charts;
pub mod commands;
pub mod config;
pub mod drillholes;
pub mod geomap;
pub mod geotech;
pub mod gtcommands;
pub mod gtconfig;
pub mod gti18n;
pub mod gtpanels;
pub mod host_sink;
pub mod i18n;
pub mod interactive;
pub mod opsbind;
pub mod opsrun;
pub mod panels;
pub mod resources;
pub mod ribbon;
pub mod rscommands;
pub mod rsconfig;
pub mod rsi18n;
pub mod rspanels;
pub mod sections;
pub mod session;
pub mod solids;
pub mod state;
pub mod xdata;

use ocs_plugin_api::host::acadrust::EntityType;
use ocs_plugin_api::host::{BuiltinPlugin, Handle, HostApi, HostNotification};
use ocs_plugin_api::manifest::{ApiVersion, EntityClasses, PluginManifest};
use ocs_plugin_api::ribbon::CadModule;

use i18n::Lang;

/// Keep in sync with `plugin.toml`.
pub static MANIFEST: PluginManifest = PluginManifest {
    id: "opencad.mining",
    name: "Open Mining",
    version: env!("CARGO_PKG_VERSION"),
    description: "Mining & geology: drillholes, underground workings and geological mapping",
    api_version: ApiVersion::CURRENT,
    ribbon_order: 40,
    xdata_apps: &[xdata::APP, xdata::APP_DOMAIN],
    command_prefixes: &["MIN_"],
};

macro_rules! classes {
    ($name:ident, $s:expr, $r:expr, $g:expr) => {
        static $name: EntityClasses = EntityClasses {
            app_id: xdata::APP,
            class_prefix: xdata::CLASS_PREFIX,
            labels: &[
                ("DH_TRACE", $s.cls_trace),
                ("DH_COLLAR", $s.cls_collar),
                ("DH_ASSAY", $s.cls_assay),
                ("DH_GEO", $s.cls_geo),
                ("DH_LABEL", $s.cls_label),
                ("UG_SOLID", $s.cls_solid),
                ("UG_SECTION", $s.cls_section),
                ("GM_TRACE", $s.cls_geotrace),
                ("GM_PROJ", $s.cls_geoproj),
                ("RS_COMP", $r.cls_comp),
                ("RS_SURFACE", $r.cls_surface),
                ("RS_VIEW", $r.cls_view),
                ("AX_AXIS", $s.cls_axis),
                ("SC_CUT", $s.cls_cut),
                ("SC_SHEET", $s.cls_sheet),
                ("GT_REC", $g.cls_rec),
            ],
        };
    };
}
classes!(CLASSES_ES, i18n::ES, rsi18n::ES, gti18n::ES);
classes!(CLASSES_EN, i18n::EN, rsi18n::EN, gti18n::EN);
classes!(CLASSES_PT, i18n::PT, rsi18n::PT, gti18n::PT);

pub struct MiningPlugin;

impl BuiltinPlugin for MiningPlugin {
    fn manifest(&self) -> &'static PluginManifest {
        &MANIFEST
    }

    fn ribbon(&self) -> Box<dyn CadModule> {
        Box::new(ribbon::MiningModule)
    }

    fn entity_classes(&self) -> Option<&'static EntityClasses> {
        Some(match i18n::lang() {
            Lang::Es => &CLASSES_ES,
            Lang::En => &CLASSES_EN,
            Lang::Pt => &CLASSES_PT,
        })
    }

    fn on_load(&mut self, host: &mut dyn HostApi) {
        let c = config::ProjectConfig::default();
        for def in panels::ALL.iter().map(|id| panels::build(id, &c)).collect::<Vec<_>>() {
            let id = def.id.clone();
            if !host.register_panel_for_plugin(MANIFEST.id, def) {
                host.push_error(&format!("Open Mining: panel '{id}' could not be registered."));
            }
        }
    }

    fn on_notification(&mut self, _command_id: Option<u64>, n: HostNotification) {
        match n {
            HostNotification::PanelAction {
                panel_id,
                control_id,
                value: Some(v),
                ..
            } if panels::ALL.contains(&panel_id.as_str()) => {
                state::track_value(control_id, v);
            }
            HostNotification::DocumentTabClosed { tab_id } => state::close_tab(tab_id),
            _ => {}
        }
    }

    fn dispatch(&self, host: &mut dyn HostApi, cmd: &str) -> bool {
        let name = cmd.split_whitespace().next().unwrap_or("").to_ascii_uppercase();
        if !name.starts_with("MIN_") {
            return false;
        }
        let s = i18n::s();
        let panel = match name.as_str() {
            "MIN_DH_PANEL" => Some(panels::DH),
            "MIN_UG_PANEL" => Some(panels::UG),
            "MIN_GM_PANEL" => Some(panels::GM),
            "MIN_AX_PANEL" => Some(panels::AX),
            "MIN_SC_PANEL" => Some(panels::SC),
            "MIN_RS_COMP_PANEL" => Some(rspanels::COMP),
            "MIN_RS_VARIO_PANEL" => Some(rspanels::VARIO),
            "MIN_RS_BM_PANEL" => Some(rspanels::BM),
            "MIN_GT_DATA_PANEL" => Some(gtpanels::DATA),
            "MIN_GT_HB_PANEL" => Some(gtpanels::HB),
            "MIN_GT_MODEL_PANEL" => Some(gtpanels::MODEL),
            _ => None,
        };
        if let Some(p) = panel {
            host.toggle_panel_for_plugin(MANIFEST.id, p, None);
            return true;
        }

        let tab = host.tab_id();
        let mut cfg = session::config(host, s);
        if opsrun::try_dispatch(host, cmd, tab, &mut cfg) {
            return true;
        }
        // Follow-ups of interactive picks write the result of their operation.
        let follow = commands::follow_up(host, &name, tab, &mut cfg)
            .or_else(|| rscommands::follow_up(host, &name, tab, &mut cfg))
            .or_else(|| {
                (name == "MIN_GT_CLASSIFY_EXEC")
                    .then(|| gtcommands::classify_exec(host, tab, &cfg).map(|r| ("gt_classify", r)))
                    .flatten()
            });
        match follow {
            Some((op, r)) => {
                opsrun::emit(host, op, &r);
                true
            }
            None => false,
        }
    }
}

/// Faces of the mesh-like entities (Mesh, PolyfaceMesh); all 3DFACEs are
/// merged into one mesh so their section chains across faces.
pub(crate) fn meshes_of(ents: &[Option<EntityType>]) -> Vec<solids_engine::cut::MeshData> {
    use solids_engine::cut::MeshData;
    let v3 = |p: &ocs_plugin_api::host::acadrust::types::Vector3| [p.x, p.y, p.z];
    let mut out = Vec::new();
    let mut faces3d = MeshData::default();
    for e in ents.iter().flatten() {
        match e {
            EntityType::Mesh(m) => out.push(MeshData {
                vertices: m.vertices.iter().map(v3).collect(),
                faces: m.faces.iter().map(|f| f.vertices.clone()).collect(),
            }),
            EntityType::PolyfaceMesh(m) => out.push(MeshData {
                vertices: m.vertices.iter().map(|v| v3(&v.location)).collect(),
                faces: m
                    .faces
                    .iter()
                    .map(|f| {
                        [f.index1, f.index2, f.index3, f.index4]
                            .iter()
                            .filter(|&&i| i != 0)
                            .map(|&i| i.unsigned_abs() as usize - 1)
                            .collect()
                    })
                    .collect(),
            }),
            EntityType::Face3D(f) => {
                let mut c = vec![
                    v3(&f.first_corner),
                    v3(&f.second_corner),
                    v3(&f.third_corner),
                    v3(&f.fourth_corner),
                ];
                if c[3] == c[2] {
                    c.pop();
                }
                let base = faces3d.vertices.len();
                faces3d.faces.push((base..base + c.len()).collect());
                faces3d.vertices.extend(c);
            }
            _ => {}
        }
    }
    if !faces3d.faces.is_empty() {
        out.push(faces3d);
    }
    out
}

pub(crate) fn polyline_points(e: &EntityType) -> Option<Vec<mining_core::Vec3>> {
    match e {
        EntityType::Polyline3D(p) => Some(
            p.vertices
                .iter()
                .map(|v| [v.position.x, v.position.y, v.position.z])
                .collect(),
        ),
        EntityType::LwPolyline(p) => Some(
            p.vertices
                .iter()
                .map(|v| [v.location.x, v.location.y, p.elevation])
                .collect(),
        ),
        EntityType::Line(l) => Some(vec![[l.start.x, l.start.y, l.start.z], [l.end.x, l.end.y, l.end.z]]),
        _ => None,
    }
}

/// Words of a command line; `"..."` keeps spaces (Windows paths).
pub fn command_args(cmd: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let (mut quoted, mut any) = (false, false);
    for ch in cmd.chars() {
        match ch {
            '"' => {
                quoted = !quoted;
                any = true;
            }
            c if c.is_whitespace() && !quoted => {
                if any {
                    out.push(std::mem::take(&mut cur));
                    any = false;
                }
            }
            c => {
                cur.push(c);
                any = true;
            }
        }
    }
    if any {
        out.push(cur);
    }
    out
}

/// Points and closed flag of the line-like entities that can be projected.
pub(crate) fn polyline_geometry(e: &EntityType) -> Option<(Vec<mining_core::Vec3>, bool)> {
    let closed = match e {
        EntityType::Polyline3D(p) => p.flags.closed,
        EntityType::LwPolyline(p) => p.is_closed,
        _ => false,
    };
    polyline_points(e).map(|p| (p, closed))
}

pub(crate) fn fetch_points(host: &dyn HostApi, handles: &[Handle]) -> Option<Vec<Vec<mining_core::Vec3>>> {
    let ents = host.entities_batch(handles).ok()?;
    ents.iter().map(|e| e.as_ref().and_then(polyline_points)).collect()
}

ocs_plugin_api::export_plugin!(MiningPlugin);

#[cfg(test)]
mod args_tests {
    use super::command_args;

    #[test]
    fn quoted_paths_keep_their_spaces() {
        assert_eq!(
            command_args(r#"MIN_DH_LOAD "C:\Mis datos\collar.csv" /tmp/s.csv - "x y.csv""#),
            vec!["MIN_DH_LOAD", r"C:\Mis datos\collar.csv", "/tmp/s.csv", "-", "x y.csv"]
        );
        assert_eq!(command_args("  MIN_DH_LOAD  "), vec!["MIN_DH_LOAD"]);
        assert_eq!(command_args(r#"A "" B"#), vec!["A", "", "B"]);
    }
}

#[cfg(test)]
mod manifest_tests {
    use super::MANIFEST;

    /// `plugin.toml` is read by the host before loading the library; it must
    /// agree with the compiled manifest.
    #[test]
    fn plugin_toml_matches_manifest() {
        let toml = include_str!("../../../plugin.toml");
        let field = |k: &str| {
            toml.lines()
                .find_map(|l| l.trim().strip_prefix(k).and_then(|r| r.trim_start().strip_prefix('=')))
                .map(|v| v.trim().trim_matches('"').to_string())
                .unwrap_or_else(|| panic!("{k} missing in plugin.toml"))
        };
        assert_eq!(field("id"), MANIFEST.id);
        assert_eq!(field("name"), MANIFEST.name);
        assert_eq!(field("version"), MANIFEST.version);
        assert_eq!(field("api_version"), MANIFEST.api_version.major.to_string());
        assert_eq!(field("ribbon_order"), MANIFEST.ribbon_order.to_string());
        for app in MANIFEST.xdata_apps {
            assert!(toml.contains(&format!("\"{app}\"")));
        }
        assert!(toml.contains(&format!("\"{}\"", MANIFEST.command_prefixes[0])));
    }
}

#[cfg(test)]
mod entity_tests {
    use super::*;
    use ocs_plugin_api::host::acadrust::entities::{Face3D, Mesh, MeshFace};
    use ocs_plugin_api::host::acadrust::types::Vector3;
    use solids_engine::cut::{cut, level_planes};

    fn v(x: f64, y: f64, z: f64) -> Vector3 {
        Vector3::new(x, y, z)
    }

    /// A unit cube as a host `Mesh` and as six `3DFACE`s gives the same
    /// horizontal section (a closed 1 x 1 square).
    #[test]
    fn host_meshes_and_3dfaces_are_cut() {
        let c = [
            v(0., 0., 0.),
            v(1., 0., 0.),
            v(1., 1., 0.),
            v(0., 1., 0.),
            v(0., 0., 1.),
            v(1., 0., 1.),
            v(1., 1., 1.),
            v(0., 1., 1.),
        ];
        let quads = [
            [0, 3, 2, 1],
            [4, 5, 6, 7],
            [0, 1, 5, 4],
            [1, 2, 6, 5],
            [2, 3, 7, 6],
            [3, 0, 4, 7],
        ];
        let mut mesh = Mesh::new();
        mesh.vertices = c.to_vec();
        mesh.faces = quads.iter().map(|q| MeshFace::new(q.to_vec())).collect();
        let faces: Vec<Option<EntityType>> = quads
            .iter()
            .map(|q| Some(EntityType::Face3D(Face3D::new(c[q[0]], c[q[1]], c[q[2]], c[q[3]]))))
            .collect();
        let plane = level_planes(0.5, 0.5, 1.0).unwrap()[0].plane;
        for ents in [vec![Some(EntityType::Mesh(mesh))], faces] {
            let meshes = meshes_of(&ents);
            assert_eq!(meshes.len(), 1);
            let loops = cut(&meshes, &plane);
            assert_eq!(loops.len(), 1);
            assert!(loops[0].closed && (loops[0].area() - 1.0).abs() < 1e-12);
        }
    }
}
