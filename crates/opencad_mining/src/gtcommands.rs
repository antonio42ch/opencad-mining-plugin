//! `MIN_GT_*` commands: bind the geotechnical module (`geotech`) to the host
//! (files, drillholes of the tab, axis polylines, block model, panels).

use std::collections::BTreeMap;

use geotech_engine::data::{GtField, GtKind};
use ocs_plugin_api::dialog::{DialogControl, DialogDef, DialogResponse, DialogRow, DialogValue};
use ocs_plugin_api::host::acadrust::EntityType;
use ocs_plugin_api::host::{Handle, HostApi};

use crate::cad::CadSink;
use crate::config::{flatten, ProjectConfig};
use crate::geotech::{self as gt, GtState};
use crate::gti18n::{g, GtStrings};
use crate::host_sink::{tag_of, HostSink};
use crate::i18n::fmt;
use crate::interactive::{PickSelection, SelectionKind};
use crate::state::{self, Pending};
use crate::{gtpanels, session, MANIFEST};

/// Modal column chooser for a geotechnical table.
fn ask_columns(
    host: &mut dyn HostApi,
    gs: &GtStrings,
    kind: GtKind,
    headers: &[String],
    fields: &[GtField],
) -> Option<BTreeMap<String, String>> {
    let s = crate::i18n::s();
    let mut rows = vec![DialogRow::new(vec![DialogControl::InfoText {
        text: fmt(gs.m_map_intro, &[&gt::kind_name(gs, kind)]),
        align_h: None,
    }])];
    for f in fields {
        let mut options = vec![s.m_none.to_string()];
        options.extend(headers.iter().cloned());
        let guess = headers
            .iter()
            .position(|h| drillhole_engine::table::normalize_header(h) == f.key().replace('_', ""))
            .map_or(0, |i| i + 1);
        rows.push(DialogRow::new(vec![DialogControl::Choice {
            id: f.key().into(),
            label: gs.field(*f),
            options,
            default_index: guess,
        }]));
    }
    let dialog = DialogDef {
        title: gs.m_map_title.into(),
        rows,
        submit_label: "OK".into(),
        cancel_label: s.m_cancelled.trim_end_matches('.').into(),
        width: Some(420.0),
    };
    match host.show_dialog(dialog) {
        DialogResponse::Submitted(values) => Some(
            flatten(&values)
                .into_iter()
                .filter_map(|(k, v)| match v {
                    DialogValue::Choice { text, .. } if text != s.m_none => Some((k, text)),
                    _ => None,
                })
                .collect(),
        ),
        DialogResponse::Cancelled => None,
    }
}

fn finish(host: &mut dyn HostApi, tab: u64, cfg: &ProjectConfig, label: &str) {
    let mut sink = HostSink::new(host);
    sink.begin(label);
    session::commit(&mut sink, crate::i18n::s(), tab, cfg);
    sink.end();
}

fn push(host: &mut dyn HostApi, cfg: &ProjectConfig, panel: &'static str, status: Option<(&str, String)>) {
    let mut values = gtpanels::values_for(cfg).remove(panel).unwrap_or_default();
    if let Some((id, text)) = status {
        values.insert(id.to_string(), DialogValue::Text(text));
    }
    host.update_panel_for_plugin(MANIFEST.id, panel, values);
}

/// Pushes every geotechnical panel (settings restored from a drawing).
pub fn push_all(host: &mut dyn HostApi, cfg: &ProjectConfig) {
    for (panel, values) in gtpanels::values_for(cfg) {
        host.update_panel_for_plugin(MANIFEST.id, panel, values);
    }
}

/// Tables of the tab; after reopening the drawing they are read again from
/// the files of the project (no questions: the columns are remembered).
fn need_tables(host: &mut dyn HostApi, tab: u64, cfg: &mut ProjectConfig) -> bool {
    if state::with_tab(tab, |t| !t.gt.is_empty()) {
        return true;
    }
    let gs = g();
    match gt::load(gs, &mut cfg.gt, &mut |_, _, _| None) {
        Ok((st, _)) => {
            host.push_info(gs.m_reloaded);
            state::with_tab(tab, |t| t.gt = st);
            true
        }
        Err(_) => {
            host.push_error(gs.m_no_data);
            false
        }
    }
}

/// Axis polylines: named by their layer (the `Axis` column of a mapping
/// table is compared with it), else by the axis id, else by the handle.
fn axes_of(host: &dyn HostApi, handles: &[Handle]) -> Vec<(String, Vec<mining_core::Vec3>)> {
    let ents = host.entities_batch(handles).unwrap_or_default();
    handles
        .iter()
        .zip(ents)
        .filter_map(|(h, e)| {
            let e: EntityType = e?;
            let pts = crate::fetch_points(host, &[*h])?.pop()?;
            let layer = e.common().layer.clone();
            let name = if !layer.is_empty() && layer != "0" {
                layer
            } else {
                tag_of(&e)
                    .and_then(|t| t.get("id").map(str::to_string))
                    .unwrap_or_else(|| format!("{:X}", h.value()))
            };
            Some((name, pts))
        })
        .collect()
}

fn classify_and_draw(host: &mut dyn HostApi, tab: u64, cfg: &ProjectConfig, axes: &[(String, Vec<mining_core::Vec3>)]) {
    let gs = g();
    let (st, data) = state::with_tab(tab, |t| (t.gt.clone(), t.data.clone()));
    if st.logging.is_some() && data.is_none() {
        host.push_error(gs.m_need_holes);
    }
    let holes = data
        .as_ref()
        .map(drillhole_engine::pipeline::build_holes)
        .unwrap_or_default();
    let results = gt::classify_all(&st, &holes, axes, &cfg.gt);
    let lines = gt::report(gs, &results);
    lines.iter().for_each(|l| host.push_info(l));
    let mut sink = HostSink::new(host);
    sink.begin("MIN_GT_CLASSIFY");
    gt::draw(&mut sink, gs, &cfg.gt, &results, crate::drillholes::origin(cfg));
    session::commit(&mut sink, crate::i18n::s(), tab, cfg);
    sink.end();
    push(
        host,
        cfg,
        gtpanels::DATA,
        Some((
            gtpanels::DATA_STATUS,
            lines.iter().take(3).cloned().collect::<Vec<_>>().join("\n"),
        )),
    );
    state::with_tab(tab, |t| t.gt.results = results);
}

/// Re-registers the Hoek-Brown panel with a new envelope (the host rebuilds a
/// changed panel hidden and with defaults: show it and restore the values).
fn refresh_hb(host: &mut dyn HostApi, cfg: &ProjectConfig, svg: &str, out: String) {
    if host.register_panel_for_plugin(MANIFEST.id, gtpanels::hb(g(), cfg, svg)) {
        host.toggle_panel_for_plugin(MANIFEST.id, gtpanels::HB, Some(true));
    }
    push(host, cfg, gtpanels::HB, Some((gtpanels::HB_OUT, out)));
}

fn run_hb(host: &mut dyn HostApi, cfg: &ProjectConfig) {
    match gt::hoek_brown(g(), &cfg.gt) {
        Err(e) => host.push_error(&e),
        Ok(o) => {
            o.text.lines().for_each(|l| host.push_info(l));
            refresh_hb(host, cfg, &o.svg, o.text);
        }
    }
}

pub fn dispatch(host: &mut dyn HostApi, cmd: &str, name: &str, tab: u64, cfg: &mut ProjectConfig) -> bool {
    let gs = g();
    match name {
        "MIN_GT_LOAD" => {
            // Optional paths: MIN_GT_LOAD logging [mapping [lab [joints]]], "-" skips one.
            let args = crate::command_args(cmd);
            for (slot, arg) in [
                &mut cfg.gt.logging,
                &mut cfg.gt.mapping,
                &mut cfg.gt.lab,
                &mut cfg.gt.joints,
            ]
            .into_iter()
            .zip(args.iter().skip(1))
            {
                if arg != "-" {
                    *slot = arg.clone();
                }
            }
            let result = {
                let mut ask = |k, h: &[String], f: &[GtField]| ask_columns(&mut *host, gs, k, h, f);
                gt::load(gs, &mut cfg.gt, &mut ask)
            };
            match result {
                Err(e) => {
                    host.push_error(&e);
                    push(host, cfg, gtpanels::DATA, Some((gtpanels::DATA_STATUS, e)));
                }
                Ok((st, lines)) => {
                    for (is_err, l) in &lines {
                        if *is_err {
                            host.push_error(l)
                        } else {
                            host.push_info(l)
                        }
                    }
                    let status = lines.iter().map(|(_, l)| l.clone()).collect::<Vec<_>>().join("\n");
                    state::with_tab(tab, |t| t.gt = st);
                    finish(host, tab, cfg, "MIN_GT_LOAD");
                    push(host, cfg, gtpanels::DATA, Some((gtpanels::DATA_STATUS, status)));
                }
            }
        }
        "MIN_GT_CLASSIFY" => {
            if !need_tables(host, tab, cfg) {
                return true;
            }
            if state::with_tab(tab, |t| t.gt.needs_axes()) {
                state::with_tab(tab, |t| t.config = cfg.clone());
                let sel = host.selection();
                if sel.is_empty() {
                    host.start_interactive(Box::new(PickSelection::new(
                        gs.m_pick_axis.to_string(),
                        SelectionKind::GtAxes,
                    )));
                } else {
                    state::set_pending(Pending::GtAxes(sel));
                    return dispatch(host, cmd, "MIN_GT_CLASSIFY_EXEC", tab, cfg);
                }
            } else {
                classify_and_draw(host, tab, cfg, &[]);
            }
        }
        "MIN_GT_CLASSIFY_EXEC" => {
            if let Some(Pending::GtAxes(handles)) = state::take_pending() {
                let axes = axes_of(host, &handles);
                if axes.is_empty() {
                    host.push_error(crate::i18n::s().m_not_polyline);
                    return true;
                }
                classify_and_draw(host, tab, cfg, &axes);
            }
        }
        "MIN_GT_CLEAR" => {
            gt::clear(&mut HostSink::new(host), gs);
        }
        "MIN_GT_HB" => {
            run_hb(host, cfg);
            finish(host, tab, cfg, "MIN_GT_HB");
        }
        "MIN_GT_HB_DATA" => {
            if !need_tables(host, tab, cfg) {
                return true;
            }
            let st: GtState = state::with_tab(tab, |t| t.gt.clone());
            match gt::hb_from_data(gs, &mut cfg.gt, &st) {
                Err(e) => host.push_error(&e),
                Ok(msg) => {
                    host.push_info(&msg);
                    run_hb(host, cfg);
                    finish(host, tab, cfg, "MIN_GT_HB_DATA");
                }
            }
        }
        "MIN_GT_ESTIMATE" => {
            if !need_tables(host, tab, cfg) {
                return true;
            }
            let results = state::with_tab(tab, |t| t.gt.results.clone());
            if results.is_empty() {
                host.push_error(gs.m_est_none);
                return true;
            }
            let mut model = match crate::rscommands::current_model(host, tab, cfg) {
                Some(m) => m,
                None => match gt::model_for(&results, cfg) {
                    Ok(m) => {
                        host.push_info(&fmt(gs.m_model_created, &[&m.len()]));
                        m
                    }
                    Err(e) => {
                        host.push_error(&e);
                        return true;
                    }
                },
            };
            match gt::estimate(&mut model, &results, cfg) {
                Err(e) => host.push_error(&e),
                Ok(out) => {
                    let line = fmt(gs.m_est_done, &[&out.estimated, &out.total, &out.columns.join(", ")]);
                    host.push_info(&line);
                    if out.no_domain > 0 {
                        host.push_info(&fmt(gs.m_no_domain, &[&out.no_domain]));
                    }
                    let r = crate::rsi18n::r();
                    match crate::rscommands::save_model(host, cfg, &model, r) {
                        Ok(path) => host.push_info(&fmt(r.m_bm_saved, &[&path])),
                        Err(e) => host.push_info(&e),
                    }
                    state::with_tab(tab, |t| t.rs.model = Some(model));
                    push(host, cfg, gtpanels::MODEL, Some((gtpanels::MODEL_STATUS, line)));
                    finish(host, tab, cfg, "MIN_GT_ESTIMATE");
                }
            }
        }
        "MIN_GT_VIEW" => {
            // Same view as Resources, on the geotechnical column; pushed into the
            // resource panel so a pick-based view (re-read after the pick) keeps it.
            cfg.rs.bm.view_attr = crate::resources::GT_VIEW_ATTR + cfg.gt.md_attr.min(3);
            state::with_tab(tab, |t| t.config = cfg.clone());
            session::push_rs(host, cfg);
            return crate::rscommands::dispatch(host, "MIN_RS_BMVIEW", tab, cfg);
        }
        _ => return false,
    }
    true
}

/// Values tracked from the geotechnical panels (`state::track_value`).
pub fn is_panel(id: &str) -> bool {
    gtpanels::ALL.contains(&id)
}
