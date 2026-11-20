//! Panels built from the operation definitions (`openmining_ops`): the
//! controls, their values and how panel values reach the project settings.
//! A panel, the command arguments and the JSON schema share one definition.

use std::collections::HashMap;

use ocs_plugin_api::dialog::{DialogControl, DialogValue, PanelDef};
use ocs_plugin_api::host::HostApi;
use openmining_ops::{self as ops, control_id, Item, Kind, PanelSpec, Param};
use serde_json::{json, Value};

use crate::config::ProjectConfig;
use crate::MANIFEST;

pub fn lang() -> ops::Lang {
    match crate::i18n::lang() {
        crate::i18n::Lang::Es => ops::Lang::Es,
        crate::i18n::Lang::En => ops::Lang::En,
        crate::i18n::Lang::Pt => ops::Lang::Pt,
    }
}

pub fn spec(id: &str) -> Option<&'static PanelSpec> {
    ops::defs::panels().into_iter().find(|p| p.id == id)
}

/// Ids of the panels built from definitions.
pub fn panel_ids() -> Vec<&'static str> {
    ops::defs::panels().into_iter().map(|p| p.id).collect()
}

pub fn settings_json(cfg: &ProjectConfig) -> Value {
    serde_json::to_value(cfg).unwrap_or(Value::Null)
}

/// Back from JSON; an impossible value keeps the previous settings.
pub fn from_json(cfg: &mut ProjectConfig, root: Value) {
    if let Ok(c) = serde_json::from_value::<ProjectConfig>(root) {
        *cfg = c;
    }
}

fn param(op: &str, key: &str) -> &'static Param {
    ops::find(op)
        .and_then(|d| d.param(key))
        .expect("panel items are checked by openmining_ops tests")
}

fn to_dialog(p: &Param, v: &Value) -> DialogValue {
    match p.kind {
        Kind::Number { .. } | Kind::Integer { .. } => DialogValue::Number(v.as_f64().unwrap_or(0.0)),
        Kind::Bool => DialogValue::Bool(v.as_bool().unwrap_or(false)),
        Kind::Choice { .. } => DialogValue::Choice {
            index: v.as_u64().unwrap_or(0) as usize,
            text: String::new(),
        },
        Kind::Table { .. } => DialogValue::Table(
            v.as_array()
                .map(|rows| {
                    rows.iter()
                        .map(|r| r.as_array().map(|c| c.iter().map(cell).collect()).unwrap_or_default())
                        .collect()
                })
                .unwrap_or_default(),
        ),
        _ => DialogValue::Text(v.as_str().unwrap_or("").to_string()),
    }
}

fn cell(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// Settings value of a panel control (`None` = unreadable for this kind).
fn from_dialog(p: &Param, v: &DialogValue) -> Option<Value> {
    Some(match (&p.kind, v) {
        (Kind::Number { .. }, DialogValue::Number(n)) if n.is_finite() => json!(n),
        (Kind::Number { .. }, DialogValue::Text(s)) => json!(s.trim().replace(',', ".").parse::<f64>().ok()?),
        (Kind::Integer { .. }, DialogValue::Number(n)) if n.is_finite() => json!(n.round() as i64),
        (Kind::Bool, DialogValue::Bool(b)) => json!(b),
        (Kind::Choice { .. }, DialogValue::Choice { index, .. }) => json!(index),
        (Kind::Text | Kind::Path { .. }, DialogValue::Text(s) | DialogValue::FilePath(s)) => json!(s),
        (Kind::Table { .. }, DialogValue::Table(rows)) => json!(rows),
        _ => return None,
    })
}

fn control(p: &Param, ptr: &str, v: &Value, l: ops::Lang) -> DialogControl {
    let id = control_id(ptr);
    let label = p.label.get(l).to_string();
    match p.kind {
        Kind::Number { min, max, step } => DialogControl::Number {
            id,
            label,
            default: v.as_f64().unwrap_or(0.0),
            min,
            max,
            step: Some(step),
        },
        Kind::Integer { min, max } => DialogControl::Number {
            id,
            label,
            default: v.as_f64().unwrap_or(0.0),
            min: min.map(|m| m as f64),
            max: max.map(|m| m as f64),
            step: Some(1.0),
        },
        Kind::Bool => DialogControl::Checkbox {
            id,
            label,
            default: v.as_bool().unwrap_or(false),
        },
        Kind::Choice { options } => DialogControl::Choice {
            id,
            label,
            options: options.iter().map(|o| o.label.get(l).to_string()).collect(),
            default_index: (v.as_u64().unwrap_or(0) as usize).min(options.len().saturating_sub(1)),
        },
        Kind::Path { extensions } => DialogControl::FilePicker {
            id,
            label,
            default_path: v.as_str().unwrap_or("").to_string(),
            filter_name: "CSV / Excel".into(),
            extensions: extensions.iter().map(|e| e.to_string()).collect(),
            save: false,
        },
        Kind::Table { columns } => DialogControl::Table {
            id,
            label: Some(label),
            headers: columns.iter().map(|c| c.get(l).to_string()).collect(),
            rows: match to_dialog(p, v) {
                DialogValue::Table(r) => r,
                _ => Vec::new(),
            },
            allow_add: true,
            allow_delete: true,
        },
        _ => DialogControl::Text {
            id,
            label,
            default: v.as_str().unwrap_or("").to_string(),
        },
    }
}

fn build(items: &[Item], root: &Value, charts: &[(&str, &str)], l: ops::Lang) -> Vec<DialogControl> {
    items
        .iter()
        .map(|it| match it {
            Item::Param(op, key) => {
                let p = param(op, key);
                let ptr = p.setting.expect("panel params are settings");
                control(p, ptr, root.pointer(ptr).unwrap_or(&Value::Null), l)
            }
            Item::Button(op) => {
                let d = ops::find(op).expect("checked by tests");
                DialogControl::Button {
                    id: format!("btn_{}", d.name),
                    label: d.label.get(l).to_string(),
                    action: Some(d.command.to_string()),
                    align_h: None,
                    width: None,
                }
            }
            Item::Section {
                id,
                title,
                items,
                collapsed,
            } => DialogControl::Fields {
                id: id.to_string(),
                title: Some(title.get(l).to_string()),
                controls: build(items, root, charts, l),
                collapsed: *collapsed,
            },
            Item::Row(items) => DialogControl::Row {
                controls: build(items, root, charts, l),
                align_h: None,
                align_v: None,
            },
            Item::Info(t) => DialogControl::InfoText {
                text: t.get(l).to_string(),
                align_h: None,
            },
            Item::Status(id) => DialogControl::Label {
                id: id.to_string(),
                text: String::new(),
                align_h: None,
            },
            Item::Chart(id) => DialogControl::Svg {
                id: id.to_string(),
                svg: charts.iter().find(|(c, _)| c == id).map_or("", |(_, s)| s).to_string(),
                height: Some(230.0),
            },
        })
        .collect()
}

/// The panel `id` with the values of `cfg`; `charts` fills chart controls.
pub fn panel(id: &str, cfg: &ProjectConfig, charts: &[(&str, &str)]) -> PanelDef {
    let s = spec(id).unwrap_or_else(|| panic!("no panel spec {id}"));
    let l = lang();
    let mut p = PanelDef::new(s.id, s.title.get(l));
    p.command = Some(s.command.into());
    p.width = Some(340.0);
    p.controls = build(s.items, &settings_json(cfg), charts, l);
    p
}

/// Values to push into the panel `id`.
pub fn values(id: &str, cfg: &ProjectConfig) -> HashMap<String, DialogValue> {
    let root = settings_json(cfg);
    let mut out = HashMap::new();
    if let Some(s) = spec(id) {
        s.visit(&mut |it| {
            if let Item::Param(op, key) = it {
                let p = param(op, key);
                let ptr = p.setting.expect("settings");
                if let Some(v) = root.pointer(ptr) {
                    out.insert(control_id(ptr), to_dialog(p, v));
                }
            }
        });
    }
    out
}

/// Pushes the values of every generated panel.
pub fn push_all(host: &mut dyn HostApi, cfg: &ProjectConfig) {
    for id in panel_ids() {
        host.update_panel_for_plugin(MANIFEST.id, id, values(id, cfg));
    }
}

/// Applies flattened panel values of the generated panels to the settings.
pub fn apply(cfg: &mut ProjectConfig, v: &HashMap<String, DialogValue>) {
    let mut root = settings_json(cfg);
    let mut changed = false;
    for s in ops::defs::panels() {
        s.visit(&mut |it| {
            if let Item::Param(op, key) = it {
                let p = param(op, key);
                let ptr = p.setting.expect("settings");
                if let Some(val) = v.get(&control_id(ptr)).and_then(|dv| from_dialog(p, dv)) {
                    changed |= ops::settings::set(&mut root, ptr, val);
                }
            }
        });
    }
    if changed {
        from_json(cfg, root);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn declared(p: &PanelDef) -> Vec<String> {
        fn walk(c: &DialogControl, out: &mut Vec<String>) {
            match c {
                DialogControl::Fields { controls, .. }
                | DialogControl::Row { controls, .. }
                | DialogControl::Column { controls, .. } => controls.iter().for_each(|c| walk(c, out)),
                other => {
                    if let Some(id) = other.id() {
                        out.push(id.to_string())
                    }
                }
            }
        }
        let mut out = Vec::new();
        p.controls.iter().for_each(|c| walk(c, &mut out));
        out
    }

    #[test]
    fn every_setting_points_into_the_project_settings() {
        ops::settings::check_against(&settings_json(&ProjectConfig::default()));
    }

    #[test]
    fn panel_values_roundtrip_through_the_settings() {
        let mut c = ProjectConfig::default();
        c.gt.logging = "/d/log.csv".into();
        c.gt.correlation = 2;
        c.gt.hb_gsi = 63.0;
        c.gt.md_radii = [80.0, 60.0, 15.0];
        c.gt.md_domains = true;
        c.gt.md_attr = 3;
        let mut flat = HashMap::new();
        for id in panel_ids() {
            let p = panel(id, &c, &[]);
            let ids = declared(&p);
            let v = values(id, &c);
            for k in v.keys() {
                assert!(ids.contains(k), "{k} not declared in {id}");
            }
            flat.extend(v);
        }
        let mut back = ProjectConfig::default();
        apply(&mut back, &flat);
        assert_eq!(back.gt, c.gt);
        // File pickers report FilePath values.
        let mut fp = HashMap::new();
        fp.insert("gt.lab".to_string(), DialogValue::FilePath("/x/lab.csv".into()));
        fp.insert("gt.hb_gsi".to_string(), DialogValue::Text("bad".into()));
        let mut c2 = ProjectConfig::default();
        apply(&mut c2, &fp);
        assert_eq!((c2.gt.lab.as_str(), c2.gt.hb_gsi), ("/x/lab.csv", 50.0));
    }
}
