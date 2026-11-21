//! Handlers of the geotechnical operations (`gt_*`, see `openmining_ops`):
//! they bind `geotech` to the host (files, drillholes of the tab, axis
//! polylines, block model, panels) and return a typed result.

use std::collections::BTreeMap;

use geotech_engine::classify::summarize;
use geotech_engine::data::{GtField, GtKind, GtTable};
use ocs_plugin_api::dialog::{DialogControl, DialogDef, DialogResponse, DialogRow, DialogValue};
use ocs_plugin_api::host::acadrust::EntityType;
use ocs_plugin_api::host::{Handle, HostApi};
use openmining_ops::parse::Args;
use openmining_ops::result::{ErrorCode, OpError, OpResult};
use serde_json::{json, Value};

use crate::cad::CadSink;
use crate::config::{flatten, ProjectConfig};
use crate::geotech::{self as gt, GtResult, GtState};
use crate::gti18n::{g, GtStrings};
use crate::host_sink::{tag_of, HostSink};
use crate::i18n::fmt;
use crate::interactive::{PickSelection, SelectionKind};
use crate::opsrun::Outcome;
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
    crate::opsbind::push_one(host, cfg, panel, status);
}

fn no_data(msg: &str) -> OpError {
    OpError::new(ErrorCode::NoData, msg)
}

/// Tables of the tab; after reopening the drawing they are read again from
/// the files of the project (no questions: the columns are remembered).
fn need_tables(host: &mut dyn HostApi, tab: u64, cfg: &mut ProjectConfig) -> Result<(), OpError> {
    if state::with_tab(tab, |t| !t.gt.is_empty()) {
        return Ok(());
    }
    let gs = g();
    match gt::load(gs, &mut cfg.gt, &mut |_, _, _| None) {
        Ok((st, _)) => {
            host.push_info(gs.m_reloaded);
            state::with_tab(tab, |t| t.gt = st);
            Ok(())
        }
        Err(_) => Err(no_data(gs.m_no_data)),
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

/// Handles of a `handles` argument.
pub fn handles_arg(args: &Args, key: &str) -> Option<Vec<Handle>> {
    let list = args.get(key)?.as_array()?;
    Some(
        list.iter()
            .filter_map(|v| u64::from_str_radix(v.as_str()?, 16).ok())
            .map(Handle::new)
            .collect(),
    )
}

fn table_info(t: &Option<GtTable>) -> Value {
    match t {
        None => Value::Null,
        Some(t) => json!({
            "records": t.records.len(),
            "columns": t.mapping.iter().map(|(f, _)| f.key()).collect::<Vec<_>>(),
            "unused": t.unmapped,
        }),
    }
}

fn load(host: &mut dyn HostApi, tab: u64, cfg: &mut ProjectConfig) -> Outcome {
    let gs = g();
    let result = {
        let mut ask = |k, h: &[String], f: &[GtField]| ask_columns(&mut *host, gs, k, h, f);
        gt::load(gs, &mut cfg.gt, &mut ask)
    };
    match result {
        Err(e) => {
            push(host, cfg, gtpanels::DATA, Some((gtpanels::DATA_STATUS, e.clone())));
            let code = if e == gs.m_no_files {
                ErrorCode::MissingArgument
            } else {
                ErrorCode::Failed
            };
            Err(OpError::new(code, e))
        }
        Ok((st, lines)) => {
            let mut errors = Vec::new();
            for (is_err, l) in &lines {
                if *is_err {
                    host.push_error(l);
                    errors.push(l.clone());
                } else {
                    host.push_info(l);
                }
            }
            let status = lines.iter().map(|(_, l)| l.clone()).collect::<Vec<_>>().join("\n");
            let data = json!({
                "tables": {
                    "logging": table_info(&st.logging),
                    "mapping": table_info(&st.mapping),
                    "lab": if st.lab_rows > 0 { json!({"records": st.lab_rows}) } else { Value::Null },
                    "joints": table_info(&st.joints),
                },
                "lab_lithologies": st.lab_rows,
                "errors": errors,
            });
            state::with_tab(tab, |t| t.gt = st);
            finish(host, tab, cfg, "MIN_GT_LOAD");
            push(host, cfg, gtpanels::DATA, Some((gtpanels::DATA_STATUS, status)));
            Ok(OpResult {
                warnings: errors,
                ..OpResult::data(data)
            })
        }
    }
}

pub(crate) fn classify_data(gs: &GtStrings, results: &[GtResult], drawn: usize) -> Value {
    let cs: Vec<_> = results.iter().map(|r| r.c.clone()).collect();
    let sm = summarize(&cs);
    let mut classes = [0usize; 5];
    for c in &cs {
        if let Some(k) = c.class {
            classes[(k - 1) as usize] += 1;
        }
    }
    let missing: BTreeMap<&str, usize> = sm.missing.iter().map(|(f, n)| (f.key(), *n)).collect();
    let mut derived: BTreeMap<&str, usize> = BTreeMap::new();
    for c in &cs {
        for d in &c.derived {
            *derived.entry(d.key()).or_default() += 1;
        }
    }
    let invalid: Vec<Value> = results
        .iter()
        .flat_map(|r| {
            r.c.invalid
                .iter()
                .map(move |(f, v)| json!({"table": r.kind.key(), "row": r.row, "field": f.key(), "value": v}))
        })
        .collect();
    let unlocated: Vec<&GtResult> = results.iter().filter(|r| r.at.is_err()).collect();
    let examples: Vec<Value> = unlocated
        .iter()
        .take(5)
        .map(|r| json!({"table": r.kind.key(), "row": r.row, "id": r.label, "reason": gs.locate_error(r.at.as_ref().unwrap_err())}))
        .collect();
    json!({
        "records": sm.total,
        "with_rmr": sm.with_rmr,
        "with_q": sm.with_q,
        "with_gsi": sm.with_gsi,
        "unclassified": sm.unclassified,
        "classes": classes,
        "missing": missing,
        "invalid": invalid,
        "derived": derived,
        "unlocated": unlocated.len(),
        "unlocated_examples": examples,
        "drawn": drawn,
    })
}

fn classify_and_draw(
    host: &mut dyn HostApi,
    tab: u64,
    cfg: &ProjectConfig,
    axes: &[(String, Vec<mining_core::Vec3>)],
) -> Outcome {
    let gs = g();
    let (st, data) = state::with_tab(tab, |t| (t.gt.clone(), t.data.clone()));
    let mut warnings = Vec::new();
    if st.logging.is_some() && data.is_none() {
        host.push_error(gs.m_need_holes);
        warnings.push(gs.m_need_holes.to_string());
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
    let drawn = gt::draw(&mut sink, gs, &cfg.gt, &results, crate::drillholes::origin(cfg));
    session::commit(&mut sink, crate::i18n::s(), tab, cfg);
    sink.end();
    let created = std::mem::take(&mut sink.created);
    push(
        host,
        cfg,
        gtpanels::DATA,
        Some((
            gtpanels::DATA_STATUS,
            lines.iter().take(3).cloned().collect::<Vec<_>>().join("\n"),
        )),
    );
    let data = classify_data(gs, &results, drawn);
    state::with_tab(tab, |t| t.gt.results = results);
    Ok(OpResult {
        data,
        created,
        warnings,
        ..Default::default()
    })
}

/// Follow-up of the axis pick of `gt_classify`.
pub fn classify_exec(host: &mut dyn HostApi, tab: u64, cfg: &ProjectConfig) -> Option<Outcome> {
    let Some(Pending::GtAxes(handles)) = state::take_pending() else {
        return None;
    };
    Some(classify_with(host, tab, cfg, &handles))
}

fn classify_with(host: &mut dyn HostApi, tab: u64, cfg: &ProjectConfig, handles: &[Handle]) -> Outcome {
    let axes = axes_of(host, handles);
    if axes.is_empty() {
        return Err(OpError::new(ErrorCode::NotFound, crate::i18n::s().m_not_polyline).at("axes"));
    }
    classify_and_draw(host, tab, cfg, &axes)
}

/// Re-registers the Hoek-Brown panel with a new envelope (the host rebuilds a
/// changed panel hidden and with defaults: show it and restore the values).
fn refresh_hb(host: &mut dyn HostApi, cfg: &ProjectConfig, svg: &str, out: String) {
    if host.register_panel_for_plugin(MANIFEST.id, gtpanels::hb(cfg, svg)) {
        host.toggle_panel_for_plugin(MANIFEST.id, gtpanels::HB, Some(true));
    }
    push(host, cfg, gtpanels::HB, Some((gtpanels::HB_OUT, out)));
}

fn hoek_brown(host: &mut dyn HostApi, cfg: &ProjectConfig) -> Result<Value, OpError> {
    let o = gt::hoek_brown(g(), &cfg.gt).map_err(|e| OpError::new(ErrorCode::OutOfRange, e))?;
    o.text.lines().for_each(|l| host.push_info(l));
    let data = json!({
        "mb": o.hb.mb,
        "s": o.hb.s,
        "a": o.hb.a,
        "sigma_c": o.hb.sigma_c(),
        "sigma_t": o.hb.sigma_t(),
        "sigma_cm": o.hb.sigma_cm(),
        "erm": o.erm,
        "sigma3_max": o.s3max,
        "c": o.c,
        "phi": o.phi,
    });
    refresh_hb(host, cfg, &o.svg, o.text);
    Ok(data)
}

fn estimate(host: &mut dyn HostApi, tab: u64, cfg: &mut ProjectConfig) -> Outcome {
    let gs = g();
    need_tables(host, tab, cfg)?;
    let results = state::with_tab(tab, |t| t.gt.results.clone());
    if results.is_empty() {
        return Err(no_data(gs.m_est_none));
    }
    let (mut model, created) = match crate::rscommands::current_model(host, tab, cfg) {
        Some(m) => (m, false),
        None => {
            let m = gt::model_for(&results, cfg).map_err(|e| no_data(&e))?;
            host.push_info(&fmt(gs.m_model_created, &[&m.len()]));
            (m, true)
        }
    };
    let out = gt::estimate(&mut model, &results, cfg).map_err(|e| no_data(&e))?;
    let line = fmt(gs.m_est_done, &[&out.estimated, &out.total, &out.columns.join(", ")]);
    host.push_info(&line);
    let mut warnings = Vec::new();
    if out.no_domain > 0 {
        let w = fmt(gs.m_no_domain, &[&out.no_domain]);
        host.push_info(&w);
        warnings.push(w);
    }
    let r = crate::rsi18n::r();
    let saved = match crate::rscommands::save_model(host, cfg, &model, r) {
        Ok(path) => {
            host.push_info(&fmt(r.m_bm_saved, &[&path]));
            Some(path)
        }
        Err(e) => {
            host.push_info(&e);
            warnings.push(e);
            None
        }
    };
    state::with_tab(tab, |t| t.rs.model = Some(model));
    push(host, cfg, gtpanels::MODEL, Some((gtpanels::MODEL_STATUS, line)));
    finish(host, tab, cfg, "MIN_GT_ESTIMATE");
    Ok(OpResult {
        warnings,
        ..OpResult::data(json!({
            "estimated": out.estimated,
            "blocks": out.total,
            "no_domain": out.no_domain,
            "columns": out.columns,
            "model_created": created,
            "saved": saved,
        }))
    })
}

/// Runs the operation `name` (settings arguments are already in `cfg`).
pub fn run(host: &mut dyn HostApi, name: &str, args: &Args, tab: u64, cfg: &mut ProjectConfig) -> Outcome {
    let gs = g();
    match name {
        "gt_load" => load(host, tab, cfg),
        "gt_classify" => {
            need_tables(host, tab, cfg)?;
            if !state::with_tab(tab, |t| t.gt.needs_axes()) {
                return classify_and_draw(host, tab, cfg, &[]);
            }
            if let Some(h) = handles_arg(args, "axes") {
                return classify_with(host, tab, cfg, &h);
            }
            state::with_tab(tab, |t| t.config = cfg.clone());
            let sel = host.selection();
            if sel.is_empty() {
                host.start_interactive(Box::new(PickSelection::new(
                    gs.m_pick_axis.to_string(),
                    SelectionKind::GtAxes,
                )));
                Ok(OpResult::waiting(gs.m_pick_axis))
            } else {
                classify_with(host, tab, cfg, &sel)
            }
        }
        "gt_clear" => {
            let n = gt::clear(&mut HostSink::new(host), gs);
            Ok(OpResult::data(json!({ "removed": n })))
        }
        "gt_hb" => {
            let data = hoek_brown(host, cfg)?;
            finish(host, tab, cfg, "MIN_GT_HB");
            Ok(OpResult::data(data))
        }
        "gt_hb_data" => {
            need_tables(host, tab, cfg)?;
            let st: GtState = state::with_tab(tab, |t| t.gt.clone());
            let used = gt::hb_from_data(gs, &mut cfg.gt, &st).map_err(|e| no_data(&e))?;
            host.push_info(&used.message);
            let hb = hoek_brown(host, cfg)?;
            finish(host, tab, cfg, "MIN_GT_HB_DATA");
            Ok(OpResult::data(json!({
                "gsi": cfg.gt.hb_gsi,
                "records": used.records,
                "lab": used.lab,
                "sigci": cfg.gt.hb_sigci,
                "mi": cfg.gt.hb_mi,
                "ei": cfg.gt.hb_ei,
                "hoek_brown": hb,
            })))
        }
        "gt_estimate" => estimate(host, tab, cfg),
        "gt_view" => {
            // The block model view of Resources, on the geotechnical column with the
            // geotechnical view settings. They are copied into the resource settings
            // (shown in its panel) so a pick-based view, re-read after the pick,
            // keeps them. Every block with a value is shown (no cut-off, any domain).
            let (g, attr) = (cfg.gt.clone(), cfg.gt.md_attr.min(5));
            let b = &mut cfg.rs.bm;
            b.view_attr = crate::resources::GT_VIEW_ATTR + attr;
            b.view_mode = g.view_mode.min(3);
            (b.view_z_from, b.view_z_to, b.view_z_step) = (g.view_z_from, g.view_z_to, g.view_z_step);
            (b.view_spacing, b.view_count, b.view_interval, b.view_limit) =
                (g.view_spacing, g.view_count, g.view_interval, g.view_limit);
            b.view_domain = 0;
            (b.view_filter, b.view_threshold) = (g.view_filter.min(2), g.view_threshold);
            (b.view_cut, b.view_cut_point) = (g.view_cut, [g.view_cut_e, g.view_cut_n, g.view_cut_z]);
            (b.view_cut_dip, b.view_cut_dipdir, b.view_cut_side) =
                (g.view_cut_dip, g.view_cut_dipdir, g.view_cut_side.min(1));
            state::with_tab(tab, |t| t.config = cfg.clone());
            session::push_rs(host, cfg);
            let r = crate::rscommands::run(host, "rs_bmview", args, tab, cfg)?;
            let attribute = ["rmr", "gsi", "logq", "class", "dist", "n"][attr];
            let mode = ["levels", "two_points", "axis", "cubes"][g.view_mode.min(3)];
            let mut data = json!({ "attribute": attribute, "column": gt::COLUMNS[attr], "mode": mode });
            if let (Some(d), Some(extra)) = (data.as_object_mut(), r.data.as_object()) {
                d.extend(extra.iter().map(|(k, v)| (k.clone(), v.clone())));
            }
            Ok(OpResult { data, ..r })
        }
        other => Err(OpError::new(
            ErrorCode::UnknownCommand,
            format!("unknown geotechnical operation {other}"),
        )),
    }
}
