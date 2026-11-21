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
    let mut c = cfg.clone();
    c.rs.migrate_legacy();
    serde_json::to_value(&c).unwrap_or(Value::Null)
}

/// Run-time lists behind `Name` / `Index` parameters.
#[derive(Debug, Clone, Default)]
pub struct Lists {
    pub assay: Vec<String>,
    pub geo: Vec<String>,
    pub domains: Vec<String>,
}

impl Lists {
    /// Lists of a tab: attributes of its loaded drillholes, domains of `cfg`.
    pub fn of(tab: u64, cfg: &ProjectConfig) -> Lists {
        let (assay, geo) = crate::state::with_tab(tab, |t| {
            t.data.as_ref().map_or((Vec::new(), Vec::new()), |d| {
                (
                    drillhole_engine::import::assay_attributes(&d.assays),
                    drillhole_engine::import::geo_attributes(&d.geo),
                )
            })
        });
        Lists {
            assay,
            geo,
            domains: cfg.rs.domain_names(),
        }
    }
    pub fn get(&self, source: &str) -> &[String] {
        match source {
            "assay attributes" => &self.assay,
            "geology attributes" => &self.geo,
            _ => &self.domains,
        }
    }
    /// Choice entries of an `Index` parameter (with the "all" entry).
    fn index_options(&self, source: &str, all: Option<ops::L>, l: ops::Lang) -> Vec<String> {
        all.map(|a| a.get(l).to_string())
            .into_iter()
            .chain(self.get(source).iter().cloned())
            .collect()
    }
}

/// Rows shown by a table control (typed settings: legacy forms included).
fn rows_of(codec: &str, cfg: &ProjectConfig) -> Vec<Vec<String>> {
    let r = crate::rsi18n::r();
    match codec {
        "ranges" => cfg
            .dh
            .custom_ranges
            .iter()
            .map(|(a, b, c)| vec![a.to_string(), b.to_string(), c.clone()])
            .collect(),
        "geo_colors" => cfg
            .dh
            .geo_colors
            .iter()
            .map(|(k, v)| vec![k.clone(), v.clone()])
            .collect(),
        "domains" => cfg
            .rs
            .domains
            .iter()
            .map(|d| {
                let surface = if !d.surface.trim().is_empty() {
                    d.surface.clone()
                } else {
                    ["", r.word_above, r.word_below][d.side.min(2)].to_string()
                };
                vec![
                    d.name.clone(),
                    d.values.clone(),
                    d.density.to_string(),
                    d.cap.to_string(),
                    surface,
                ]
            })
            .collect(),
        "contacts" => cfg
            .rs
            .contact_rows()
            .into_iter()
            .map(|c| vec![c.name, c.above, c.below])
            .collect(),
        "ch_orient" => cfg
            .ch
            .orient
            .iter()
            .map(|o| {
                vec![
                    o.channel.clone(),
                    crate::resources::num(o.azimuth),
                    crate::resources::num(o.inclination),
                ]
            })
            .collect(),
        "joint_sets" => cfg
            .rm
            .sets
            .iter()
            .map(|s| {
                vec![
                    s.name.clone(),
                    crate::resources::num(s.dip),
                    crate::resources::num(s.dipdir),
                    crate::resources::num(s.friction),
                    crate::resources::num(s.cohesion),
                ]
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn parse_num(s: &str) -> Option<f64> {
    s.trim().replace(',', ".").parse::<f64>().ok().filter(|v| v.is_finite())
}

/// Settings value of table rows (rows that cannot be read are dropped).
fn json_of(codec: &str, rows: &[Vec<String>]) -> Option<Value> {
    let cell = |r: &Vec<String>, i: usize| r.get(i).map(|s| s.trim().to_string()).unwrap_or_default();
    Some(match codec {
        "ranges" => json!(rows
            .iter()
            .filter_map(|r| Some(json!([parse_num(r.first()?)?, parse_num(r.get(1)?)?, cell(r, 2)])))
            .collect::<Vec<_>>()),
        "geo_colors" => Value::Object(rows.iter().filter(|r| !cell(r, 0).is_empty()).map(|r| (cell(r, 0), json!(cell(r, 1)))).collect()),
        "domains" => json!(rows
            .iter()
            .filter(|r| !cell(r, 0).is_empty())
            .map(|r| json!({
                "name": cell(r, 0),
                "values": r.get(1).cloned().unwrap_or_default(),
                "density": r.get(2).and_then(|s| parse_num(s)).unwrap_or(2.7),
                "cap": r.get(3).and_then(|s| parse_num(s)).unwrap_or(0.0),
                "surface": cell(r, 4),
            }))
            .collect::<Vec<_>>()),
        "contacts" => json!(rows
            .iter()
            .filter(|r| r.iter().any(|c| !c.trim().is_empty()))
            .enumerate()
            .map(|(i, r)| {
                let name = cell(r, 0);
                json!({"name": if name.is_empty() { format!("C{}", i + 1) } else { name }, "above": cell(r, 1), "below": cell(r, 2)})
            })
            .collect::<Vec<_>>()),
        "ch_orient" => json!(rows
            .iter()
            .filter_map(|r| {
                let ch = cell(r, 0);
                let az = r.get(1).and_then(|s| parse_num(s))?;
                let inc = r.get(2).and_then(|s| parse_num(s)).unwrap_or(0.0);
                (!ch.is_empty() && (-90.0..=90.0).contains(&inc)).then(|| json!({"channel": ch, "azimuth": az.rem_euclid(360.0), "inclination": inc}))
            })
            .collect::<Vec<_>>()),
        "joint_sets" => json!(rows
            .iter()
            .enumerate()
            .filter_map(|(i, r)| {
                let dip = r.get(1).and_then(|s| parse_num(s)).filter(|d| (0.0..=90.0).contains(d))?;
                let dd = r.get(2).and_then(|s| parse_num(s))?;
                let name = cell(r, 0);
                Some(json!({
                    "name": if name.is_empty() { format!("J{}", i + 1) } else { name },
                    "dip": dip, "dipdir": dd.rem_euclid(360.0),
                    "friction": r.get(3).and_then(|s| parse_num(s)).unwrap_or(30.0),
                    "cohesion": r.get(4).and_then(|s| parse_num(s)).unwrap_or(0.0),
                }))
            })
            .collect::<Vec<_>>()),
        _ => return None,
    })
}

/// Settings value of a table argument (rows given as strings).
pub fn table_json(codec: &str, v: &Value) -> Option<Value> {
    let rows: Vec<Vec<String>> = v
        .as_array()?
        .iter()
        .map(|r| r.as_array().map(|c| c.iter().map(cell).collect()).unwrap_or_default())
        .collect();
    json_of(codec, &rows)
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

fn to_dialog(p: &Param, v: &Value, cfg: &ProjectConfig, lists: &Lists, l: ops::Lang) -> DialogValue {
    match p.kind {
        Kind::Name { source } => {
            let list = lists.get(source).to_vec();
            let cur = v.as_str().unwrap_or("");
            DialogValue::ChoiceOptions {
                selected_index: list.iter().position(|a| a == cur).unwrap_or(0),
                options: list,
            }
        }
        Kind::Index { source, all } => {
            let list = lists.index_options(source, all, l);
            let i = v.as_u64().unwrap_or(0) as usize;
            DialogValue::ChoiceOptions {
                selected_index: i.min(list.len().saturating_sub(1)),
                options: list,
            }
        }
        Kind::Table { codec, .. } => DialogValue::Table(rows_of(codec, cfg)),
        Kind::Number { .. } | Kind::Integer { .. } => DialogValue::Number(v.as_f64().unwrap_or(0.0)),
        Kind::Bool => DialogValue::Bool(v.as_bool().unwrap_or(false)),
        Kind::Choice { .. } => DialogValue::Choice {
            index: v.as_u64().unwrap_or(0) as usize,
            text: String::new(),
        },
        // A file picker only takes a path value: a text one is ignored, the picker
        // stays empty and its empty value would wipe the setting on the next run.
        Kind::Path { .. } => DialogValue::FilePath(v.as_str().unwrap_or("").to_string()),
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
    let in_range =
        |x: f64, min: Option<f64>, max: Option<f64>| min.is_none_or(|m| x >= m) && max.is_none_or(|m| x <= m);
    Some(match (&p.kind, v) {
        // Out-of-range panel values keep the previous setting (as the hand-written panels did).
        (Kind::Number { min, max, .. }, DialogValue::Number(n)) if n.is_finite() && in_range(*n, *min, *max) => {
            json!(n)
        }
        (Kind::Number { min, max, .. }, DialogValue::Text(s)) => {
            json!(parse_num(s).filter(|x| in_range(*x, *min, *max))?)
        }
        (Kind::Name { .. }, DialogValue::Choice { text, .. }) if !text.is_empty() => json!(text),
        (Kind::Index { .. }, DialogValue::Choice { index, .. }) => json!(index),
        (Kind::Integer { .. }, DialogValue::Number(n)) if n.is_finite() => json!(n.round() as i64),
        (Kind::Bool, DialogValue::Bool(b)) => json!(b),
        (Kind::Choice { .. }, DialogValue::Choice { index, .. }) => json!(index),
        (Kind::Text | Kind::Path { .. }, DialogValue::Text(s) | DialogValue::FilePath(s)) => json!(s),
        (Kind::Table { codec, .. }, DialogValue::Table(rows)) => json_of(codec, rows)?,
        _ => return None,
    })
}

fn control(p: &Param, ptr: &str, v: &Value, cfg: &ProjectConfig, lists: &Lists, l: ops::Lang) -> DialogControl {
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
        Kind::Table { columns, codec } => DialogControl::Table {
            id,
            label: Some(label),
            headers: columns.iter().map(|c| c.get(l).to_string()).collect(),
            rows: rows_of(codec, cfg),
            allow_add: true,
            allow_delete: true,
        },
        Kind::Name { source } => {
            let list = lists.get(source).to_vec();
            let cur = v.as_str().unwrap_or("");
            DialogControl::Choice {
                id,
                label,
                default_index: list.iter().position(|a| a == cur).unwrap_or(0),
                options: list,
            }
        }
        Kind::Index { source, all } => {
            let list = lists.index_options(source, all, l);
            let i = (v.as_u64().unwrap_or(0) as usize).min(list.len().saturating_sub(1));
            DialogControl::Choice {
                id,
                label,
                options: list,
                default_index: i,
            }
        }
        _ => DialogControl::Text {
            id,
            label,
            default: v.as_str().unwrap_or("").to_string(),
        },
    }
}

struct Ctx<'a> {
    root: &'a Value,
    cfg: &'a ProjectConfig,
    lists: &'a Lists,
    charts: &'a [(&'a str, &'a str)],
    l: ops::Lang,
}

fn build(items: &[Item], c: &Ctx) -> Vec<DialogControl> {
    let l = c.l;
    items
        .iter()
        .map(|it| match it {
            Item::Param(op, key) => {
                let p = param(op, key);
                let ptr = p.setting.expect("panel params are settings");
                control(p, ptr, c.root.pointer(ptr).unwrap_or(&Value::Null), c.cfg, c.lists, l)
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
                controls: build(items, c),
                collapsed: *collapsed,
            },
            Item::Row(items) => DialogControl::Row {
                controls: build(items, c),
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
                svg: c
                    .charts
                    .iter()
                    .find(|(k, _)| k == id)
                    .map_or("", |(_, s)| s)
                    .to_string(),
                height: Some(230.0),
            },
        })
        .collect()
}

/// The panel `id` with the values of `cfg`; `charts` fills chart controls.
pub fn panel(id: &str, cfg: &ProjectConfig, charts: &[(&str, &str)]) -> PanelDef {
    panel_with(
        id,
        cfg,
        &Lists {
            domains: cfg.rs.domain_names(),
            ..Default::default()
        },
        charts,
    )
}

pub fn panel_with(id: &str, cfg: &ProjectConfig, lists: &Lists, charts: &[(&str, &str)]) -> PanelDef {
    let s = spec(id).unwrap_or_else(|| panic!("no panel spec {id}"));
    let l = lang();
    let mut p = PanelDef::new(s.id, s.title.get(l));
    p.command = Some(s.command.into());
    p.width = Some(340.0);
    let root = settings_json(cfg);
    p.controls = build(
        s.items,
        &Ctx {
            root: &root,
            cfg,
            lists,
            charts,
            l,
        },
    );
    p
}

/// Values to push into the panel `id`. Run-time lists that are empty are
/// not pushed (the choice keeps what it shows).
pub fn values(id: &str, cfg: &ProjectConfig, lists: &Lists) -> HashMap<String, DialogValue> {
    let root = settings_json(cfg);
    let l = lang();
    let mut out = HashMap::new();
    if let Some(s) = spec(id) {
        s.visit(&mut |it| {
            if let Item::Param(op, key) = it {
                let p = param(op, key);
                let ptr = p.setting.expect("settings");
                let empty_list = match p.kind {
                    Kind::Name { source } | Kind::Index { source, all: None } => lists.get(source).is_empty(),
                    _ => false,
                };
                if let (Some(v), false) = (root.pointer(ptr), empty_list) {
                    out.insert(control_id(ptr), to_dialog(p, v, cfg, lists, l));
                }
            }
        });
    }
    out
}

/// Pushes the values of every generated panel.
pub fn push_all(host: &mut dyn HostApi, cfg: &ProjectConfig) {
    let lists = Lists::of(host.tab_id(), cfg);
    for id in panel_ids() {
        host.update_panel_for_plugin(MANIFEST.id, id, values(id, cfg, &lists));
    }
}

/// Pushes one panel, optionally with a status label.
pub fn push_one(host: &mut dyn HostApi, cfg: &ProjectConfig, id: &str, status: Option<(&str, String)>) {
    let lists = Lists::of(host.tab_id(), cfg);
    let mut v = values(id, cfg, &lists);
    if let Some((k, text)) = status {
        v.insert(k.to_string(), DialogValue::Text(text));
    }
    host.update_panel_for_plugin(MANIFEST.id, id, v);
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
        c.gt.view_mode = 3;
        c.gt.view_limit = 500.0;
        c.gt.view_z_step = 6.0;
        c.gt.view_filter = 2;
        c.gt.view_threshold = 2.5;
        c.gt.view_cut = true;
        c.gt.view_cut_e = 500_100.0;
        c.gt.view_cut_dip = 45.0;
        c.gt.view_cut_side = 1;
        let mut flat = HashMap::new();
        for id in panel_ids() {
            let p = panel(id, &c, &[]);
            let ids = declared(&p);
            let v = values(id, &c, &Lists::default());
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

    #[test]
    fn tables_and_lists_roundtrip_through_the_panels() {
        use crate::rsconfig::{ContactRow, DomainCfg};
        let mut c = ProjectConfig::default();
        c.dh.custom_ranges = vec![(0.0, 1.5, "Verde".into()), (1.5, 9.0, "1".into())];
        c.dh.geo_colors = [("OXIDO".to_string(), "Rojo".to_string())].into();
        c.dh.assay_attr = "AU".into();
        c.rs.domains = vec![
            DomainCfg {
                name: "Óxidos".into(),
                values: "OX;OXIDO".into(),
                density: 2.4,
                cap: 30.0,
                side: 0,
                surface: "Encima C1".into(),
            },
            DomainCfg {
                name: "Sulfuros".into(),
                values: "SU".into(),
                ..Default::default()
            },
        ];
        c.rs.contacts = vec![ContactRow {
            name: "C1".into(),
            above: "OX".into(),
            below: "SU".into(),
        }];
        c.rs.vario.domain = 1;
        c.rs.bm.view_domain = 2;
        let lists = Lists {
            assay: vec!["CU".into(), "AU".into()],
            geo: vec![],
            domains: c.rs.domain_names(),
        };
        let mut flat = HashMap::new();
        for id in panel_ids() {
            let mut v = values(id, &c, &lists);
            // The host answers a run-time choice with its index and text.
            for val in v.values_mut() {
                if let DialogValue::ChoiceOptions {
                    options,
                    selected_index,
                } = val
                {
                    *val = DialogValue::Choice {
                        index: *selected_index,
                        text: options.get(*selected_index).cloned().unwrap_or_default(),
                    };
                }
            }
            flat.extend(v);
        }
        assert!(
            matches!(&flat["rs.bm.view_domain"], DialogValue::Choice { index: 2, text } if text == "Sulfuros"),
            "index 0 is 'all'"
        );
        let mut back = ProjectConfig::default();
        apply(&mut back, &flat);
        assert_eq!(back.dh.custom_ranges, c.dh.custom_ranges);
        assert_eq!(back.dh.geo_colors, c.dh.geo_colors);
        assert_eq!(back.dh.assay_attr, "AU");
        assert_eq!(back.rs.domains, c.rs.domains);
        assert_eq!(back.rs.contacts, c.rs.contacts);
        assert_eq!((back.rs.vario.domain, back.rs.bm.view_domain), (1, 2));
        // Unreadable range rows are dropped, empty domain names too.
        let mut bad = HashMap::new();
        bad.insert(
            "dh.custom_ranges".to_string(),
            DialogValue::Table(vec![
                vec!["x".into(), "1".into(), "Rojo".into()],
                vec!["0".into(), "2".into(), "Azul".into()],
            ]),
        );
        bad.insert(
            "rs.domains".to_string(),
            DialogValue::Table(vec![vec!["".into(), "OX".into()], vec!["D".into()]]),
        );
        let mut c2 = ProjectConfig::default();
        apply(&mut c2, &bad);
        assert_eq!(c2.dh.custom_ranges, vec![(0.0, 2.0, "Azul".to_string())]);
        assert_eq!((c2.rs.domains.len(), c2.rs.domains[0].density), (1, 2.7));
    }

    #[test]
    fn legacy_single_contact_and_domain_side_survive() {
        let mut c = ProjectConfig::default();
        c.rs.domains = vec![crate::rsconfig::DomainCfg {
            name: "Sup".into(),
            side: 1,
            ..Default::default()
        }];
        c.rs.contact.above = "OX".into();
        // A settings round trip (any argument or panel change) keeps the v0.4 side.
        let mut root = settings_json(&c);
        ops::settings::set(&mut root, "/gt/hb_gsi", serde_json::json!(60.0));
        let mut back = c.clone();
        from_json(&mut back, root);
        assert_eq!(back.rs.domains[0].conditions(), vec![(String::new(), true)]);
        assert_eq!(back.gt.hb_gsi, 60.0);
        // The single v0.4 contact is shown as C1.
        assert_eq!(
            rows_of("contacts", &c),
            vec![vec!["C1".to_string(), "OX".into(), String::new()]]
        );
    }

    /// Paths are pushed as path values (the only kind a file picker accepts),
    /// so a path given as an argument survives the next panel read.
    #[test]
    fn paths_are_pushed_as_file_paths_and_read_back() {
        let mut c = ProjectConfig::default();
        c.ch.points = "/d/puntos.csv".into();
        c.gt.logging = "/d/log.csv".into();
        c.dh.collar = "/d/collar.csv".into();
        let mut flat = HashMap::new();
        for id in panel_ids() {
            flat.extend(values(id, &c, &Lists::default()));
        }
        for k in ["ch.points", "gt.logging", "dh.collar"] {
            assert!(
                matches!(&flat[k], DialogValue::FilePath(p) if p.starts_with("/d/")),
                "{k}: {:?}",
                flat[k]
            );
        }
        let mut back = ProjectConfig::default();
        apply(&mut back, &flat);
        assert_eq!(
            (
                back.ch.points.as_str(),
                back.gt.logging.as_str(),
                back.dh.collar.as_str()
            ),
            ("/d/puntos.csv", "/d/log.csv", "/d/collar.csv")
        );
    }
}
