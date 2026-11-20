//! Per-command session: settings from the drawing, panel values, persistence.

use std::collections::{BTreeMap, HashMap};

use drillhole_engine::error::TableKind;
use drillhole_engine::import::Field;
use ocs_plugin_api::dialog::{DialogControl, DialogDef, DialogResponse, DialogRow, DialogValue};
use ocs_plugin_api::host::HostApi;

use crate::cad::CadSink;
use crate::config::{flatten, ProjectConfig};
use crate::host_sink::HostSink;
use crate::i18n::{fmt, Strings};
use crate::{panels, state, MANIFEST};

/// Current settings for the active tab: restored from the drawing the first
/// time (and pushed into the panels), otherwise refreshed from the panels.
pub fn config(host: &mut dyn HostApi, s: &Strings) -> ProjectConfig {
    let tab = host.tab_id();
    let first = state::with_tab(tab, |t| !std::mem::replace(&mut t.settings_checked, true));
    if first {
        let stored = HostSink::new(host)
            .load_settings()
            .and_then(|j| ProjectConfig::from_json(&j));
        if let Some(cfg) = stored {
            push_panels(host, &cfg, &[], &[]);
            host.push_info(s.m_config_restored);
            state::with_tab(tab, |t| t.config = cfg.clone());
            return cfg;
        }
    }
    let mut cfg = state::with_tab(tab, |t| t.config.clone());
    cfg.apply(&state::tracked_values());
    for p in panels::ALL {
        // `panel_snapshot` (not `panel_snapshot_for_plugin`): out of process the
        // IPC client implements only this one and the host authenticates the
        // caller from the connection; the `_for_plugin` variant falls back to
        // the contract default, which always fails.
        // A snapshot lists only readable controls in `values`; `errors` names
        // the unreadable ones (a field never edited, an empty dropdown...).
        // Use every value that is there: discarding the whole panel for one
        // unreadable control silently fell back to stale settings.
        if let Ok(snap) = host.panel_snapshot(p) {
            cfg.apply(&flatten(&snap.values));
        }
    }
    cfg
}

/// Stores the settings for the tab and in the drawing when they changed.
pub fn commit(sink: &mut dyn CadSink, s: &Strings, tab: u64, cfg: &ProjectConfig) {
    let changed = state::with_tab(tab, |t| {
        let changed = t.config != *cfg;
        t.config = cfg.clone();
        changed
    });
    let stored = sink.load_settings();
    if changed || stored.is_none() || stored.as_deref() != Some(cfg.to_json().as_str()) {
        if let Err(e) = sink.save_settings(&cfg.to_json()) {
            sink.error(&fmt(s.m_config_save_failed, &[&e]));
        }
    }
}

/// Pushes the resource panels' values (domain lists included).
pub fn push_rs(host: &mut dyn HostApi, cfg: &ProjectConfig) {
    for (panel, values) in crate::rspanels::values_for(cfg, crate::rsi18n::r()) {
        host.update_panel_for_plugin(MANIFEST.id, panel, values);
    }
}

/// Replaces the chart of a resource panel. The host rebuilds a panel whose
/// definition changed (it comes back hidden and with default values), so the
/// panel is shown again and every value restored from `cfg`.
pub fn refresh_chart(host: &mut dyn HostApi, cfg: &ProjectConfig, panel: &'static str, svg: &str) {
    let r = crate::rsi18n::r();
    let def = match panel {
        crate::rspanels::COMP => crate::rspanels::comp(r, cfg, svg),
        crate::rspanels::VARIO => crate::rspanels::vario(r, cfg, svg),
        _ => crate::rspanels::blocks(r, cfg, svg),
    };
    if host.register_panel_for_plugin(MANIFEST.id, def) {
        host.toggle_panel_for_plugin(MANIFEST.id, panel, Some(true));
    }
    push_rs(host, cfg);
}

/// Sets a status label of a resource panel.
pub fn set_label(host: &mut dyn HostApi, panel: &'static str, id: &str, text: String) {
    host.update_panel_for_plugin(
        MANIFEST.id,
        panel,
        HashMap::from([(id.to_string(), DialogValue::Text(text))]),
    );
}

/// Current value of a control that is not part of the saved settings.
pub fn panel_value(host: &mut dyn HostApi, panel: &str, id: &str) -> Option<DialogValue> {
    if let Ok(snap) = host.panel_snapshot(panel) {
        if let Some(v) = flatten(&snap.values).remove(id) {
            return Some(v);
        }
    }
    state::tracked_values().remove(id)
}

pub fn push_panels(host: &mut dyn HostApi, cfg: &ProjectConfig, assay_attrs: &[String], geo_attrs: &[String]) {
    push_rs(host, cfg);
    crate::gtcommands::push_all(host, cfg);
    for (panel, values) in panels::values_for(cfg, assay_attrs, geo_attrs) {
        host.update_panel_for_plugin(MANIFEST.id, panel, values);
    }
}

/// Pushes one panel's values (and optionally a status label).
pub fn push_panel(host: &mut dyn HostApi, cfg: &ProjectConfig, panel: &'static str, status: Option<(&str, String)>) {
    let mut values = panels::values_for(cfg, &[], &[]).remove(panel).unwrap_or_default();
    if let Some((id, text)) = status {
        values.insert(id.to_string(), DialogValue::Text(text));
    }
    host.update_panel_for_plugin(MANIFEST.id, panel, values);
}

pub fn set_status(host: &mut dyn HostApi, text: String) {
    host.update_panel_for_plugin(
        MANIFEST.id,
        panels::DH,
        HashMap::from([(panels::DH_STATUS.to_string(), DialogValue::Text(text))]),
    );
}

/// Modal column-mapping dialog.
pub fn ask_mapping(
    host: &mut dyn HostApi,
    s: &Strings,
    table: TableKind,
    headers: &[String],
    req: &[Field],
    opt: &[Field],
) -> Option<BTreeMap<String, String>> {
    let mut rows = vec![DialogRow::new(vec![DialogControl::InfoText {
        text: fmt(s.m_mapping_intro, &[&s.table(table)]),
        align_h: None,
    }])];
    let guess = |f: &Field| {
        headers
            .iter()
            .position(|h| drillhole_engine::table::normalize_header(h) == f.key().replace('_', ""))
            .unwrap_or(0)
    };
    for f in req {
        rows.push(DialogRow::new(vec![DialogControl::Choice {
            id: f.key().into(),
            label: s.field(f.key()).into(),
            options: headers.to_vec(),
            default_index: guess(f),
        }]));
    }
    for f in opt {
        let mut options = vec![s.m_none.to_string()];
        options.extend(headers.iter().cloned());
        rows.push(DialogRow::new(vec![DialogControl::Choice {
            id: f.key().into(),
            label: s.field(f.key()).into(),
            options,
            default_index: 0,
        }]));
    }
    let dialog = DialogDef {
        title: s.m_mapping_title.into(),
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
