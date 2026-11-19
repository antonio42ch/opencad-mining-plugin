//! Open Mining — entry point of the `cdylib` the host loads.
//!
//! Layering: `mining_core`, `drillhole_engine`, `solids_engine` and
//! `geomap_engine` are host-agnostic engines. This crate only adapts them to
//! the Open CAD Studio contract. Commands draw through [`cad::CadSink`], so
//! their logic is tested against an in-memory sink.

// `export_plugin!` (host macro) emits a C entry point taking a raw pointer.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

pub mod cad;
pub mod config;
pub mod drillholes;
pub mod geomap;
pub mod host_sink;
pub mod i18n;
pub mod interactive;
pub mod panels;
pub mod ribbon;
pub mod session;
pub mod solids;
pub mod state;
pub mod xdata;

use ocs_plugin_api::host::acadrust::EntityType;
use ocs_plugin_api::host::{BuiltinPlugin, Handle, HostApi, HostNotification};
use ocs_plugin_api::manifest::{ApiVersion, EntityClasses, PluginManifest};
use ocs_plugin_api::ribbon::CadModule;

use cad::CadSink;
use host_sink::HostSink;
use i18n::{fmt, Lang};
use interactive::{PickEntities, PickKind, PickTracePoint};
use state::Pending;

/// Keep in sync with `plugin.toml`.
pub static MANIFEST: PluginManifest = PluginManifest {
    id: "opencad.mining",
    name: "Open Mining",
    version: env!("CARGO_PKG_VERSION"),
    description: "Mining & geology: drillholes, underground workings and geological mapping",
    api_version: ApiVersion::CURRENT,
    ribbon_order: 40,
    xdata_apps: &[xdata::APP],
    command_prefixes: &["MIN_"],
};

macro_rules! classes {
    ($name:ident, $s:expr) => {
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
                ("GM_NOTE", $s.cls_geonote),
            ],
        };
    };
}
classes!(CLASSES_ES, i18n::ES);
classes!(CLASSES_EN, i18n::EN);
classes!(CLASSES_PT, i18n::PT);

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
        let s = i18n::s();
        let c = config::ProjectConfig::default();
        for def in [
            panels::drillholes(s, &c),
            panels::workings(s, &c),
            panels::geomap(s, &c),
        ] {
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
            _ => None,
        };
        if let Some(p) = panel {
            host.toggle_panel_for_plugin(MANIFEST.id, p, None);
            return true;
        }

        let tab = host.tab_id();
        let mut cfg = session::config(host, s);
        match name.as_str() {
            "MIN_DH_LOAD" => {
                let result = {
                    let mut ask = |t, h: &[String], r: &[_], o: &[_]| session::ask_mapping(&mut *host, s, t, h, r, o);
                    drillholes::load(s, &mut cfg, &mut ask)
                };
                match result {
                    Err(e) => {
                        host.push_error(&e);
                        session::set_status(host, e);
                    }
                    Ok(l) => {
                        for (is_err, line) in &l.report {
                            if *is_err {
                                host.push_error(line)
                            } else {
                                host.push_info(line)
                            }
                        }
                        host.push_info(&l.summary);
                        let mut sink = HostSink::new(host);
                        sink.begin("MIN_DH_LOAD");
                        if drillholes::auto_origin(&mut cfg, &l.data) {
                            sink.info(&fmt(s.m_origin_set, &[&cfg.origin.x, &cfg.origin.y, &cfg.origin.z]));
                        }
                        session::commit(&mut sink, s, tab, &cfg);
                        sink.end();
                        session::push_panels(host, &cfg, &l.assay_attrs, &l.geo_attrs);
                        session::set_status(host, l.summary.clone());
                        state::with_tab(tab, |t| t.data = Some(l.data));
                    }
                }
            }
            "MIN_DH_DRAW" => match state::with_tab(tab, |t| t.data.clone()) {
                None => host.push_error(s.m_no_data),
                Some(data) => {
                    let mut sink = HostSink::new(host);
                    sink.begin("MIN_DH_DRAW");
                    drillholes::draw(&mut sink, s, &cfg, &data);
                    session::commit(&mut sink, s, tab, &cfg);
                    sink.end();
                }
            },
            "MIN_DH_CLEAR" => {
                drillholes::clear(&mut HostSink::new(host), s);
            }
            "MIN_DH_GEOCOLORS" => match state::with_tab(tab, |t| t.data.clone()) {
                None => host.push_error(s.m_no_data),
                Some(data) => {
                    cfg.dh.geo_colors = drillholes::geo_color_table(&data, &cfg.dh);
                    state::with_tab(tab, |t| t.config = cfg.clone());
                    session::push_panels(host, &cfg, &[], &[]);
                }
            },
            "MIN_UG_AXIS" | "MIN_UG_HBOUND" | "MIN_UG_VBOUND" => {
                state::with_tab(tab, |t| t.config = cfg.clone());
                let custom = cfg.ug.profile().is_none();
                let (kind, prompts) = match name.as_str() {
                    "MIN_UG_AXIS" => {
                        let mut p = vec![s.pick_axis.to_string()];
                        if custom {
                            p.push(s.pick_profile.to_string());
                        }
                        (PickKind::Axis { custom_profile: custom }, p)
                    }
                    "MIN_UG_HBOUND" => (
                        PickKind::Boundaries { vertical: false },
                        s.pick_boundaries_h.iter().map(|x| x.to_string()).collect(),
                    ),
                    _ => (
                        PickKind::Boundaries { vertical: true },
                        s.pick_boundaries_v.iter().map(|x| x.to_string()).collect(),
                    ),
                };
                // A single pre-selected axis skips the pick.
                let sel = host.selection();
                if matches!(kind, PickKind::Axis { custom_profile: false }) && sel.len() == 1 {
                    state::set_pending(Pending::Axis {
                        axis: sel[0],
                        profile: None,
                    });
                    run_solid(host, s, tab, &cfg);
                } else {
                    host.start_interactive(Box::new(PickEntities::new(kind, prompts)));
                }
            }
            "MIN_UG_RUN" => run_solid(host, s, tab, &cfg),
            "MIN_GM_TRACE" => {
                state::with_tab(tab, |t| t.config = cfg.clone());
                host.start_interactive(Box::new(PickTracePoint::new(cfg.gm.clone(), s.pick_point.to_string())));
            }
            "MIN_GM_RUN" => {
                if let Some(Pending::TracePoint(p)) = state::take_pending() {
                    let mut sink = HostSink::new(host);
                    sink.begin("MIN_GM_TRACE");
                    geomap::insert(&mut sink, s, &cfg.gm, p);
                    session::commit(&mut sink, s, tab, &cfg);
                    sink.end();
                }
            }
            _ => return false,
        }
        true
    }
}

fn polyline_points(e: &EntityType) -> Option<Vec<mining_core::Vec3>> {
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

fn fetch_points(host: &dyn HostApi, handles: &[Handle]) -> Option<Vec<Vec<mining_core::Vec3>>> {
    let ents = host.entities_batch(handles).ok()?;
    ents.iter().map(|e| e.as_ref().and_then(polyline_points)).collect()
}

fn run_solid(host: &mut dyn HostApi, s: &i18n::Strings, tab: u64, cfg: &config::ProjectConfig) {
    let Some(p) = state::take_pending() else {
        return;
    };
    let handles: Vec<Handle> = match &p {
        Pending::Axis { axis, profile } => std::iter::once(*axis).chain(*profile).collect(),
        Pending::Boundaries { handles, .. } => handles.to_vec(),
        Pending::TracePoint(_) => return,
    };
    let Some(pts) = fetch_points(host, &handles) else {
        host.push_error(s.m_not_polyline);
        return;
    };
    let profile2d: Option<Vec<mining_core::Vec2>> = pts
        .get(1)
        .filter(|_| matches!(p, Pending::Axis { .. }))
        .map(|v| v.iter().map(|q| [q[0], q[1]]).collect());
    let method = match &p {
        Pending::Axis { .. } => solids::Method::Axis {
            axis: &pts[0],
            custom_profile: profile2d.as_deref(),
        },
        Pending::Boundaries { vertical, .. } => solids::Method::Boundaries {
            lines: [&pts[0], &pts[1], &pts[2], &pts[3]],
            vertical: *vertical,
        },
        Pending::TracePoint(_) => unreachable!(),
    };
    let mut sink = HostSink::new(host);
    sink.begin("MIN_UG_SOLID");
    if solids::create(&mut sink, s, &cfg.ug, method) {
        session::commit(&mut sink, s, tab, cfg);
    }
    sink.end();
}

ocs_plugin_api::export_plugin!(MiningPlugin);

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
        assert!(toml.contains(&format!("\"{}\"", MANIFEST.xdata_apps[0])));
        assert!(toml.contains(&format!("\"{}\"", MANIFEST.command_prefixes[0])));
    }
}
