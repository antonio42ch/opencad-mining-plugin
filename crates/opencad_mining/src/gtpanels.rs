//! Panels of the geotechnical module.

use std::collections::HashMap;

use ocs_plugin_api::dialog::{DialogControl, DialogValue, PanelDef};

use crate::config::ProjectConfig;
use crate::gti18n::GtStrings;

pub const DATA: &str = "min_gt_data";
pub const HB: &str = "min_gt_hb";
pub const MODEL: &str = "min_gt_model";
pub const ALL: [&str; 3] = [DATA, HB, MODEL];

pub const DATA_STATUS: &str = "gt_status";
pub const HB_OUT: &str = "gt_hb_out";
pub const HB_CHART: &str = "gt_hb_chart";
pub const MODEL_STATUS: &str = "gt_md_status";

const FILE_EXT: &[&str] = &["csv", "txt", "tsv", "xlsx", "xls", "ods"];

fn file(id: &str, label: &str) -> DialogControl {
    DialogControl::FilePicker {
        id: id.into(),
        label: label.into(),
        default_path: String::new(),
        filter_name: "CSV / Excel".into(),
        extensions: FILE_EXT.iter().map(|s| s.to_string()).collect(),
        save: false,
    }
}
fn num(id: &str, label: &str, default: f64, min: Option<f64>, max: Option<f64>, step: f64) -> DialogControl {
    DialogControl::Number {
        id: id.into(),
        label: label.into(),
        default,
        min,
        max,
        step: Some(step),
    }
}
fn check(id: &str, label: &str, default: bool) -> DialogControl {
    DialogControl::Checkbox {
        id: id.into(),
        label: label.into(),
        default,
    }
}
fn text(id: &str, label: &str, default: &str) -> DialogControl {
    DialogControl::Text {
        id: id.into(),
        label: label.into(),
        default: default.into(),
    }
}
fn choice(id: &str, label: &str, options: &[&str], default_index: usize) -> DialogControl {
    DialogControl::Choice {
        id: id.into(),
        label: label.into(),
        options: options.iter().map(|s| s.to_string()).collect(),
        default_index: default_index.min(options.len().saturating_sub(1)),
    }
}
fn button(id: &str, label: &str, command: &str) -> DialogControl {
    DialogControl::Button {
        id: id.into(),
        label: label.into(),
        action: Some(command.into()),
        align_h: None,
        width: None,
    }
}
fn fields(id: &str, title: &str, controls: Vec<DialogControl>) -> DialogControl {
    DialogControl::Fields {
        id: id.into(),
        title: Some(title.into()),
        controls,
        collapsed: false,
    }
}
fn row(controls: Vec<DialogControl>) -> DialogControl {
    DialogControl::Row {
        controls,
        align_h: None,
        align_v: None,
    }
}
fn label(id: &str) -> DialogControl {
    DialogControl::Label {
        id: id.into(),
        text: String::new(),
        align_h: None,
    }
}
fn info(t: &str) -> DialogControl {
    DialogControl::InfoText {
        text: t.into(),
        align_h: None,
    }
}
fn panel(id: &str, title: &str, command: &str, controls: Vec<DialogControl>) -> PanelDef {
    let mut p = PanelDef::new(id, title);
    p.command = Some(command.into());
    p.width = Some(340.0);
    p.controls = controls;
    p
}

pub fn data(g: &GtStrings, c: &ProjectConfig) -> PanelDef {
    let x = &c.gt;
    panel(
        DATA,
        g.p_data,
        "MIN_GT_DATA_PANEL",
        vec![
            fields(
                "gt_f_files",
                g.sec_files,
                vec![
                    file("gt_logging", g.f_logging),
                    file("gt_mapping", g.f_mapping),
                    file("gt_lab", g.f_lab),
                    file("gt_joints", g.f_joints),
                ],
            ),
            button("gt_btn_load", g.btn_load, "MIN_GT_LOAD"),
            fields(
                "gt_f_opts",
                g.sec_options,
                vec![
                    choice("gt_rmrv", g.rmr_version, &g.rmr_versions, x.rmr_version),
                    info(g.rmr14_note),
                    choice("gt_corr", g.correlation, &g.correlations, x.correlation),
                    check("gt_cross", g.cross_fill, x.cross_fill),
                    check("gt_rqdff", g.rqd_from_ff, x.rqd_from_ff),
                ],
            ),
            fields(
                "gt_f_draw",
                g.sec_draw,
                vec![
                    choice("gt_color", g.color_by, &g.colors_by, x.color_by),
                    check("gt_labels", g.labels, x.labels),
                    num("gt_label_h", g.label_h, x.label_height, Some(0.01), None, 0.5),
                ],
            ),
            row(vec![
                button("gt_btn_classify", g.btn_classify, "MIN_GT_CLASSIFY"),
                button("gt_btn_clear", g.btn_clear, "MIN_GT_CLEAR"),
            ]),
            label(DATA_STATUS),
        ],
    )
}

/// `svg` is the current envelope (re-registering refreshes it).
pub fn hb(g: &GtStrings, c: &ProjectConfig, svg: &str) -> PanelDef {
    let x = &c.gt;
    panel(
        HB,
        g.p_hb,
        "MIN_GT_HB_PANEL",
        vec![
            fields(
                "gt_f_hb",
                g.sec_hb_in,
                vec![
                    num("gt_hb_sigci", g.hb_sigci, x.hb_sigci, Some(0.1), None, 10.0),
                    num("gt_hb_gsi", g.hb_gsi, x.hb_gsi, Some(0.0), Some(100.0), 5.0),
                    num("gt_hb_mi", g.hb_mi, x.hb_mi, Some(0.1), None, 1.0),
                    num("gt_hb_d", g.hb_d, x.hb_d, Some(0.0), Some(1.0), 0.1),
                    num("gt_hb_ei", g.hb_ei, x.hb_ei, Some(0.0), None, 1000.0),
                    choice("gt_hb_app", g.hb_app, &g.hb_apps, x.hb_app),
                    num("gt_hb_gamma", g.hb_gamma, x.hb_gamma, Some(1.0), None, 1.0),
                    num("gt_hb_depth", g.hb_depth, x.hb_depth, Some(1.0), None, 50.0),
                    text("gt_hb_lith", g.hb_lith, &x.hb_lith),
                ],
            ),
            row(vec![
                button("gt_btn_hb", g.btn_hb_calc, "MIN_GT_HB"),
                button("gt_btn_hbdata", g.btn_hb_data, "MIN_GT_HB_DATA"),
            ]),
            label(HB_OUT),
            DialogControl::Svg {
                id: HB_CHART.into(),
                svg: svg.into(),
                height: Some(230.0),
            },
        ],
    )
}

pub fn model(g: &GtStrings, c: &ProjectConfig) -> PanelDef {
    let x = &c.gt;
    panel(
        MODEL,
        g.p_model,
        "MIN_GT_MODEL_PANEL",
        vec![
            info(g.model_hint),
            fields(
                "gt_f_search",
                g.sec_search,
                vec![
                    choice("gt_md_method", g.md_method, &g.md_methods, x.md_method),
                    num("gt_md_power", g.md_power, x.md_power, Some(0.1), None, 0.5),
                    num(
                        "gt_md_r1",
                        &format!("{} 1", g.md_radius),
                        x.md_radii[0],
                        Some(0.1),
                        None,
                        10.0,
                    ),
                    num(
                        "gt_md_r2",
                        &format!("{} 2", g.md_radius),
                        x.md_radii[1],
                        Some(0.1),
                        None,
                        10.0,
                    ),
                    num(
                        "gt_md_r3",
                        &format!("{} 3", g.md_radius),
                        x.md_radii[2],
                        Some(0.1),
                        None,
                        10.0,
                    ),
                    num("gt_md_az", g.md_az, x.md_az, None, None, 5.0),
                    num("gt_md_min", g.md_min, x.md_min, Some(1.0), None, 1.0),
                    num("gt_md_max", g.md_max, x.md_max, Some(1.0), None, 1.0),
                    num("gt_md_hole", g.md_hole, x.md_hole, Some(0.0), None, 1.0),
                    check("gt_md_domains", g.md_domains, x.md_domains),
                ],
            ),
            button("gt_btn_est", g.btn_estimate, "MIN_GT_ESTIMATE"),
            label(MODEL_STATUS),
            fields(
                "gt_f_view",
                g.sec_view,
                vec![
                    choice("gt_md_attr", g.md_attr, &g.md_attrs, x.md_attr),
                    button("gt_btn_view", g.btn_view, "MIN_GT_VIEW"),
                ],
            ),
        ],
    )
}

/// Values to push into the geotechnical panels.
pub fn values_for(c: &ProjectConfig) -> HashMap<&'static str, HashMap<String, DialogValue>> {
    let x = &c.gt;
    let n = DialogValue::Number;
    let b = DialogValue::Bool;
    let t = |s: &str| DialogValue::Text(s.to_string());
    let ch = |i: usize| DialogValue::Choice {
        index: i,
        text: String::new(),
    };
    let collect = |v: Vec<(&str, DialogValue)>| -> HashMap<String, DialogValue> {
        v.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
    };
    let data = collect(vec![
        ("gt_logging", t(&x.logging)),
        ("gt_mapping", t(&x.mapping)),
        ("gt_lab", t(&x.lab)),
        ("gt_joints", t(&x.joints)),
        ("gt_rmrv", ch(x.rmr_version)),
        ("gt_corr", ch(x.correlation)),
        ("gt_cross", b(x.cross_fill)),
        ("gt_rqdff", b(x.rqd_from_ff)),
        ("gt_color", ch(x.color_by)),
        ("gt_labels", b(x.labels)),
        ("gt_label_h", n(x.label_height)),
    ]);
    let hb = collect(vec![
        ("gt_hb_sigci", n(x.hb_sigci)),
        ("gt_hb_gsi", n(x.hb_gsi)),
        ("gt_hb_mi", n(x.hb_mi)),
        ("gt_hb_d", n(x.hb_d)),
        ("gt_hb_ei", n(x.hb_ei)),
        ("gt_hb_app", ch(x.hb_app)),
        ("gt_hb_gamma", n(x.hb_gamma)),
        ("gt_hb_depth", n(x.hb_depth)),
        ("gt_hb_lith", t(&x.hb_lith)),
    ]);
    let model = collect(vec![
        ("gt_md_method", ch(x.md_method)),
        ("gt_md_power", n(x.md_power)),
        ("gt_md_r1", n(x.md_radii[0])),
        ("gt_md_r2", n(x.md_radii[1])),
        ("gt_md_r3", n(x.md_radii[2])),
        ("gt_md_az", n(x.md_az)),
        ("gt_md_min", n(x.md_min)),
        ("gt_md_max", n(x.md_max)),
        ("gt_md_hole", n(x.md_hole)),
        ("gt_md_domains", b(x.md_domains)),
        ("gt_md_attr", ch(x.md_attr)),
    ]);
    HashMap::from([(DATA, data), (HB, hb), (MODEL, model)])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Lang;

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
    fn values_address_declared_controls_and_roundtrip() {
        for l in [Lang::Es, Lang::En, Lang::Pt] {
            let g = crate::gti18n::for_lang(l);
            let c = ProjectConfig::default();
            let values = values_for(&c);
            for p in [data(g, &c), hb(g, &c, "<svg/>"), model(g, &c)] {
                let ids = declared(&p);
                for k in values[p.id.as_str()].keys() {
                    assert!(ids.contains(k), "{k} not declared in {} ({l:?})", p.id);
                }
            }
        }
        let mut c = ProjectConfig::default();
        c.gt.logging = "/d/log.csv".into();
        c.gt.correlation = 2;
        c.gt.hb_gsi = 63.0;
        c.gt.md_radii = [80.0, 60.0, 15.0];
        c.gt.md_domains = true;
        c.gt.md_attr = 3;
        let mut flat = HashMap::new();
        for (_, m) in values_for(&c) {
            flat.extend(m);
        }
        let mut back = ProjectConfig::default();
        back.apply(&flat);
        assert_eq!(back.gt, c.gt);
    }
}
