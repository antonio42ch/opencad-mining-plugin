//! Dockable panels. Buttons carry a command in `action`, which the host runs
//! through its command line (so they behave exactly like the ribbon tools).

use std::collections::HashMap;

use ocs_plugin_api::dialog::{DialogControl, DialogValue, PanelDef};

use crate::config::ProjectConfig;
use crate::i18n::Strings;

pub const DH: &str = "min_drillholes";
pub const UG: &str = "min_workings";
pub const GM: &str = "min_geomap";
pub const AX: &str = "min_axes";
pub const SC: &str = "min_sections";
pub const ALL: [&str; 11] = [
    DH,
    UG,
    GM,
    AX,
    SC,
    crate::rspanels::COMP,
    crate::rspanels::VARIO,
    crate::rspanels::BM,
    crate::gtpanels::DATA,
    crate::gtpanels::HB,
    crate::gtpanels::MODEL,
];
pub const DH_STATUS: &str = "dh_status";
pub const AX_STATUS: &str = "ax_status";

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
fn check(id: &str, label: &str, default: bool) -> DialogControl {
    DialogControl::Checkbox {
        id: id.into(),
        label: label.into(),
        default,
    }
}
fn num(id: &str, label: &str, default: f64, min: Option<f64>, step: f64) -> DialogControl {
    DialogControl::Number {
        id: id.into(),
        label: label.into(),
        default,
        min,
        max: None,
        step: Some(step),
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
        default_index,
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
fn row(controls: Vec<DialogControl>) -> DialogControl {
    DialogControl::Row {
        controls,
        align_h: None,
        align_v: None,
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

fn panel(id: &str, title: &str, command: &str, controls: Vec<DialogControl>) -> PanelDef {
    let mut p = PanelDef::new(id, title);
    p.command = Some(command.into());
    p.width = Some(320.0);
    p.controls = controls;
    p
}

pub fn drillholes(s: &Strings, c: &ProjectConfig) -> PanelDef {
    let d = &c.dh;
    panel(
        DH,
        s.p_drillholes,
        "MIN_DH_PANEL",
        vec![
            fields(
                "dh_files",
                s.sec_files,
                vec![
                    file("dh_collar", s.f_collar),
                    file("dh_survey", s.f_survey),
                    file("dh_assays", s.f_assays),
                    file("dh_geo", s.f_geo),
                ],
            ),
            button("dh_btn_load", s.btn_load, "MIN_DH_LOAD"),
            DialogControl::Label {
                id: DH_STATUS.into(),
                text: String::new(),
                align_h: None,
            },
            fields(
                "dh_drawing",
                s.sec_drawing,
                vec![
                    check("dh_draw_trace", s.chk_trace, d.draw_trace),
                    check("dh_draw_collar", s.chk_collar, d.draw_collar),
                    check("dh_draw_assays", s.chk_assays, d.draw_assays),
                    check("dh_draw_geo", s.chk_geo, d.draw_geo),
                    check("dh_draw_labels", s.chk_labels, d.draw_labels),
                    num("dh_label_h", s.label_height, d.label_height, Some(0.01), 0.5),
                ],
            ),
            fields(
                "dh_colors",
                s.sec_colors,
                vec![
                    choice("dh_assay_attr", s.assay_attr, &[], 0),
                    choice("dh_scheme", s.scheme, &s.schemes, d.scheme),
                    choice("dh_scope", s.scope, &s.scopes, d.scope),
                    DialogControl::Table {
                        id: "dh_ranges".into(),
                        label: Some(s.custom_ranges.into()),
                        headers: s.range_headers.iter().map(|h| h.to_string()).collect(),
                        rows: Vec::new(),
                        allow_add: true,
                        allow_delete: true,
                    },
                    choice("dh_geo_attr", s.geo_attr, &[], 0),
                    button("dh_btn_geo_colors", s.btn_geo_colors, "MIN_DH_GEOCOLORS"),
                    DialogControl::Table {
                        id: "dh_geo_colors".into(),
                        label: Some(s.geo_colors.into()),
                        headers: s.geo_color_headers.iter().map(|h| h.to_string()).collect(),
                        rows: Vec::new(),
                        allow_add: true,
                        allow_delete: true,
                    },
                ],
            ),
            fields(
                "or_fields",
                s.sec_origin,
                vec![
                    check("or_enabled", s.use_origin, c.origin.enabled),
                    num("or_x", "X", c.origin.x, None, 1000.0),
                    num("or_y", "Y", c.origin.y, None, 1000.0),
                    num("or_z", "Z", c.origin.z, None, 100.0),
                    DialogControl::InfoText {
                        text: s.origin_hint.into(),
                        align_h: None,
                    },
                ],
            ),
            DialogControl::Row {
                controls: vec![
                    button("dh_btn_draw", s.btn_draw, "MIN_DH_DRAW"),
                    button("dh_btn_clear", s.btn_clear, "MIN_DH_CLEAR"),
                ],
                align_h: None,
                align_v: None,
            },
        ],
    )
}

pub fn workings(s: &Strings, c: &ProjectConfig) -> PanelDef {
    let u = &c.ug;
    panel(
        UG,
        s.p_workings,
        "MIN_UG_PANEL",
        vec![
            fields(
                "ug_section",
                s.sec_section,
                vec![
                    choice("ug_profile", s.profile, &s.profiles, u.profile),
                    num("ug_width", s.width, u.width, Some(0.01), 0.5),
                    num("ug_height", s.height, u.height, Some(0.01), 0.5),
                    num("ug_top", s.top_width, u.top_width, Some(0.01), 0.5),
                    num("ug_radius", s.radius, u.radius, Some(0.01), 0.25),
                    num("ug_wall", s.wall_height, u.wall_height, Some(0.01), 0.5),
                    choice("ug_halign", s.h_align, &s.h_aligns, u.h_align),
                    choice("ug_valign", s.v_align, &s.v_aligns, u.v_align),
                ],
            ),
            num("ug_interval", s.interval, u.interval, Some(0.01), 0.5),
            check("ug_sections", s.draw_sections, u.draw_sections),
            check("ug_tangent3d", s.tangent_3d, u.tangent_3d),
            button("ug_btn_axis", s.tool_axis, "MIN_UG_AXIS"),
            DialogControl::Row {
                controls: vec![
                    button("ug_btn_hb", s.tool_hbound, "MIN_UG_HBOUND"),
                    button("ug_btn_vb", s.tool_vbound, "MIN_UG_VBOUND"),
                ],
                align_h: None,
                align_v: None,
            },
        ],
    )
}

pub fn geomap(s: &Strings, c: &ProjectConfig) -> PanelDef {
    let g = &c.gm;
    panel(
        GM,
        s.p_geology,
        "MIN_GM_PANEL",
        vec![
            fields(
                "gm_trace",
                s.sec_trace,
                vec![
                    num("gm_e", s.ax_e, g.e, None, 1.0),
                    num("gm_n", s.ax_n, g.n, None, 1.0),
                    num("gm_z", s.ax_z, g.z, None, 1.0),
                    text("gm_bearing", s.bearing, &g.bearing),
                    num("gm_length", s.length, g.length, Some(0.01), 0.5),
                    choice("gm_anchor", s.anchor, &s.anchors, g.anchor),
                    row(vec![
                        button("gm_btn_draw", s.btn_trace_draw, "MIN_GM_TRACE"),
                        button("gm_btn_pick", s.btn_trace_pick, "MIN_GM_TRACE_PICK"),
                    ]),
                ],
            ),
            fields(
                "gm_dipf",
                s.sec_dip,
                vec![
                    text("gm_dip", s.dip, &g.dip),
                    choice("gm_dipdir", s.dip_dir, &s.dip_dirs, g.dip_dir),
                    text("gm_dipaz", s.dip_az, &g.dip_az),
                ],
            ),
            fields(
                "gm_proj",
                s.sec_projection,
                vec![
                    choice("gm_pmode", s.proj_mode, &s.proj_modes, g.proj_mode),
                    num("gm_pvalue", s.proj_value, g.proj_value, None, 1.0),
                    button("gm_btn_project", s.btn_project, "MIN_GM_PROJECT"),
                ],
            ),
            DialogControl::Fields {
                id: "gm_obs".into(),
                title: Some(s.sec_observation.into()),
                controls: vec![
                    text("gm_rock", s.rock_type, &g.rock_type),
                    text("gm_thickness", s.thickness, &g.thickness),
                    text("gm_min", s.mineralization, &g.mineralization),
                    text("gm_alt", s.alteration, &g.alteration),
                    text("gm_struct", s.structure, &g.structure),
                    text("gm_grade", s.grade, &g.grade),
                    text("gm_sample", s.sample, &g.sample),
                ],
                collapsed: true,
            },
        ],
    )
}

pub fn axes(s: &Strings, c: &ProjectConfig) -> PanelDef {
    let a = &c.ax;
    panel(
        AX,
        s.p_axes,
        "MIN_AX_PANEL",
        vec![
            fields(
                "ax_start",
                s.sec_start,
                vec![
                    num("ax_e", s.ax_e, a.e, None, 1.0),
                    num("ax_n", s.ax_n, a.n, None, 1.0),
                    num("ax_z", s.ax_z, a.z, None, 1.0),
                    text("ax_bearing", s.bearing, &a.bearing),
                ],
            ),
            DialogControl::Row {
                controls: vec![
                    button("ax_btn_pick", s.btn_pick_start, "MIN_AX_PICKSTART"),
                    button("ax_btn_from", s.btn_from_poly, "MIN_AX_FROMPOLY"),
                ],
                align_h: None,
                align_v: None,
            },
            fields(
                "ax_grade_f",
                s.sec_grade,
                vec![
                    num("ax_grade", s.ax_grade, a.grade, None, 0.5),
                    choice("ax_grade_unit", "", &s.grade_units, a.grade_unit),
                ],
            ),
            fields(
                "ax_straight",
                s.sec_straight,
                vec![
                    num("ax_length", s.ax_length, a.length, Some(0.001), 1.0),
                    button("ax_btn_line", s.btn_add_line, "MIN_AX_LINE"),
                ],
            ),
            fields(
                "ax_curve",
                s.sec_curve,
                vec![
                    choice("ax_curve_method", s.curve_method, &s.curve_methods, a.curve_method),
                    num("ax_radius", s.radius, a.radius, Some(0.001), 1.0),
                    num("ax_curve_length", s.curve_length, a.curve_length, Some(0.001), 1.0),
                    num("ax_angle", s.central_angle, a.angle, Some(0.001), 5.0),
                    choice("ax_turn", s.turn, &s.turns, a.turn),
                    num("ax_segment", s.segment, a.segment, Some(0.001), 0.5),
                    button("ax_btn_curve", s.btn_add_curve, "MIN_AX_CURVE"),
                ],
            ),
            check("ax_update_start", s.update_start, a.update_start),
            DialogControl::Row {
                controls: vec![
                    button("ax_btn_undo", s.btn_undo_element, "MIN_AX_UNDO"),
                    button("ax_btn_new", s.btn_new_axis, "MIN_AX_NEW"),
                ],
                align_h: None,
                align_v: None,
            },
            DialogControl::Label {
                id: AX_STATUS.into(),
                text: String::new(),
                align_h: None,
            },
        ],
    )
}

pub fn sections(s: &Strings, c: &ProjectConfig) -> PanelDef {
    let x = &c.sc;
    panel(
        SC,
        s.p_sections,
        "MIN_SC_PANEL",
        vec![
            fields(
                "sc_planes",
                s.sec_planes,
                vec![
                    choice("sc_mode", s.sc_mode, &s.sc_modes, x.mode),
                    num("sc_interval", s.sc_interval, x.interval, Some(0.001), 1.0),
                    num("sc_from", s.sc_from, x.ch_from, Some(0.0), 10.0),
                    num("sc_to", s.sc_to, x.ch_to, Some(0.0), 10.0),
                    num("sc_spacing", s.sc_spacing, x.spacing, Some(0.001), 1.0),
                    num("sc_count", s.sc_count, x.count, Some(1.0), 1.0),
                    num("sc_z_from", s.sc_z_from, x.z_from, None, 5.0),
                    num("sc_z_to", s.sc_z_to, x.z_to, None, 5.0),
                    num("sc_z_step", s.sc_z_step, x.z_step, Some(0.001), 1.0),
                ],
            ),
            fields(
                "sc_output",
                s.sec_output,
                vec![
                    check("sc_draw_3d", s.sc_draw_3d, x.draw_3d),
                    check("sc_draw_sheets", s.sc_draw_sheets, x.draw_sheets),
                    num("sc_columns", s.sc_columns, x.columns, Some(1.0), 1.0),
                    num("sc_gap", s.sc_gap, x.gap, Some(0.0), 1.0),
                    num("sc_grid", s.sc_grid, x.grid, Some(0.01), 1.0),
                    num("sc_text_h", s.sc_text_h, x.text_height, Some(0.01), 0.1),
                    check("sc_auto_place", s.sc_auto_place, x.auto_place),
                    num("sc_sheet_x", s.sc_sheet_x, x.sheet_x, None, 10.0),
                    num("sc_sheet_y", s.sc_sheet_y, x.sheet_y, None, 10.0),
                    check("sc_replace", s.sc_replace, x.replace),
                ],
            ),
            DialogControl::InfoText {
                text: s.sc_hint.into(),
                align_h: None,
            },
            button("sc_btn_run", s.btn_sections, "MIN_SC_RUN"),
        ],
    )
}

/// Values to push into the panels so they show `c` (after restoring settings
/// from a drawing, or after a load fills the attribute lists).
pub fn values_for(
    c: &ProjectConfig,
    assay_attrs: &[String],
    geo_attrs: &[String],
) -> HashMap<&'static str, HashMap<String, DialogValue>> {
    let t = |s: &str| DialogValue::Text(s.to_string());
    let n = DialogValue::Number;
    let b = DialogValue::Bool;
    let ch = |i: usize, opts: &[&str]| DialogValue::Choice {
        index: i,
        text: opts.get(i).map(|s| s.to_string()).unwrap_or_default(),
    };
    let options = |list: &[String], sel: &str| DialogValue::ChoiceOptions {
        options: list.to_vec(),
        selected_index: list.iter().position(|a| a == sel).unwrap_or(0),
    };
    let (d, u, g, o) = (&c.dh, &c.ug, &c.gm, &c.origin);
    let mut dh: HashMap<String, DialogValue> = [
        ("dh_collar", DialogValue::FilePath(d.collar.clone())),
        ("dh_survey", DialogValue::FilePath(d.survey.clone())),
        ("dh_assays", DialogValue::FilePath(d.assays.clone())),
        ("dh_geo", DialogValue::FilePath(d.geo.clone())),
        ("dh_draw_trace", b(d.draw_trace)),
        ("dh_draw_collar", b(d.draw_collar)),
        ("dh_draw_assays", b(d.draw_assays)),
        ("dh_draw_geo", b(d.draw_geo)),
        ("dh_draw_labels", b(d.draw_labels)),
        ("dh_label_h", n(d.label_height)),
        ("dh_scheme", ch(d.scheme, &["", "", "", ""])),
        ("dh_scope", ch(d.scope, &["", ""])),
        (
            "dh_ranges",
            DialogValue::Table(
                d.custom_ranges
                    .iter()
                    .map(|(a, b, c)| vec![a.to_string(), b.to_string(), c.clone()])
                    .collect(),
            ),
        ),
        (
            "dh_geo_colors",
            DialogValue::Table(d.geo_colors.iter().map(|(k, v)| vec![k.clone(), v.clone()]).collect()),
        ),
        ("or_enabled", b(o.enabled)),
        ("or_x", n(o.x)),
        ("or_y", n(o.y)),
        ("or_z", n(o.z)),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v))
    .collect();
    if !assay_attrs.is_empty() {
        dh.insert("dh_assay_attr".into(), options(assay_attrs, &d.assay_attr));
    }
    if !geo_attrs.is_empty() {
        dh.insert("dh_geo_attr".into(), options(geo_attrs, &d.geo_attr));
    }
    let ug = [
        ("ug_profile", ch(u.profile, &["", "", "", "", ""])),
        ("ug_width", n(u.width)),
        ("ug_height", n(u.height)),
        ("ug_top", n(u.top_width)),
        ("ug_radius", n(u.radius)),
        ("ug_wall", n(u.wall_height)),
        ("ug_interval", n(u.interval)),
        ("ug_halign", ch(u.h_align, &["", "", ""])),
        ("ug_valign", ch(u.v_align, &["", "", ""])),
        ("ug_sections", b(u.draw_sections)),
        ("ug_tangent3d", b(u.tangent_3d)),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v))
    .collect();
    let gm = [
        ("gm_bearing", t(&g.bearing)),
        ("gm_length", n(g.length)),
        ("gm_anchor", ch(g.anchor, &["", "", ""])),
        ("gm_e", n(g.e)),
        ("gm_n", n(g.n)),
        ("gm_z", n(g.z)),
        ("gm_dipdir", ch(g.dip_dir, &["", "", ""])),
        ("gm_dipaz", t(&g.dip_az)),
        ("gm_pmode", ch(g.proj_mode, &["", ""])),
        ("gm_pvalue", n(g.proj_value)),
        ("gm_rock", t(&g.rock_type)),
        ("gm_thickness", t(&g.thickness)),
        ("gm_min", t(&g.mineralization)),
        ("gm_alt", t(&g.alteration)),
        ("gm_struct", t(&g.structure)),
        ("gm_grade", t(&g.grade)),
        ("gm_dip", t(&g.dip)),
        ("gm_sample", t(&g.sample)),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v))
    .collect();
    let (a, x) = (&c.ax, &c.sc);
    let ax = [
        ("ax_e", n(a.e)),
        ("ax_n", n(a.n)),
        ("ax_z", n(a.z)),
        ("ax_bearing", t(&a.bearing)),
        ("ax_grade", n(a.grade)),
        ("ax_grade_unit", ch(a.grade_unit, &["", ""])),
        ("ax_length", n(a.length)),
        ("ax_curve_method", ch(a.curve_method, &["", ""])),
        ("ax_radius", n(a.radius)),
        ("ax_curve_length", n(a.curve_length)),
        ("ax_angle", n(a.angle)),
        ("ax_turn", ch(a.turn, &["", ""])),
        ("ax_segment", n(a.segment)),
        ("ax_update_start", b(a.update_start)),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v))
    .collect();
    let sc = [
        ("sc_mode", ch(x.mode, &["", "", "", ""])),
        ("sc_interval", n(x.interval)),
        ("sc_from", n(x.ch_from)),
        ("sc_to", n(x.ch_to)),
        ("sc_spacing", n(x.spacing)),
        ("sc_count", n(x.count)),
        ("sc_z_from", n(x.z_from)),
        ("sc_z_to", n(x.z_to)),
        ("sc_z_step", n(x.z_step)),
        ("sc_draw_3d", b(x.draw_3d)),
        ("sc_draw_sheets", b(x.draw_sheets)),
        ("sc_columns", n(x.columns)),
        ("sc_gap", n(x.gap)),
        ("sc_grid", n(x.grid)),
        ("sc_text_h", n(x.text_height)),
        ("sc_auto_place", b(x.auto_place)),
        ("sc_sheet_x", n(x.sheet_x)),
        ("sc_sheet_y", n(x.sheet_y)),
        ("sc_replace", b(x.replace)),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v))
    .collect();
    HashMap::from([(DH, dh), (UG, ug), (GM, gm), (AX, ax), (SC, sc)])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::{for_lang, Lang};

    /// Every value written back must address a control declared in a panel.
    #[test]
    fn pushed_values_match_declared_controls() {
        fn ids(c: &DialogControl, out: &mut Vec<String>) {
            match c {
                DialogControl::Fields { controls, .. }
                | DialogControl::Row { controls, .. }
                | DialogControl::Column { controls, .. } => controls.iter().for_each(|c| ids(c, out)),
                other => {
                    if let Some(id) = other.id() {
                        out.push(id.to_string())
                    }
                }
            }
        }
        let s = for_lang(Lang::Es);
        let c = ProjectConfig::default();
        let panels = [
            drillholes(s, &c),
            workings(s, &c),
            geomap(s, &c),
            axes(s, &c),
            sections(s, &c),
        ];
        let values = values_for(&c, &["cu".into()], &["rock".into()]);
        for p in &panels {
            let mut declared = Vec::new();
            p.controls.iter().for_each(|c| ids(c, &mut declared));
            for k in values[p.id.as_str()].keys() {
                assert!(declared.contains(k), "{} not declared in {}", k, p.id);
            }
        }
    }

    /// Panel values flow back into the same config.
    #[test]
    fn panel_roundtrip() {
        let mut c = ProjectConfig::default();
        c.dh.collar = "c.csv".into();
        c.ug.width = 5.5;
        c.gm.bearing = "S30W".into();
        (c.gm.e, c.gm.n, c.gm.z) = (500_123.5, 8_500_456.25, 4_012.0);
        (c.gm.dip, c.gm.dip_dir, c.gm.dip_az) = ("65".into(), 2, "N30E".into());
        (c.gm.proj_mode, c.gm.proj_value) = (1, -12.5);
        c.dh.custom_ranges = vec![(0.0, 1.0, "Verde".into())];
        c.ax.radius = 22.5;
        c.ax.turn = 1;
        c.sc.mode = 2;
        c.sc.columns = 3.0;
        let mut flat = HashMap::new();
        for (_, m) in values_for(&c, &[], &[]) {
            flat.extend(m);
        }
        let mut back = ProjectConfig::default();
        back.apply(&flat);
        assert_eq!(back.dh.collar, "c.csv");
        assert_eq!(back.ug.width, 5.5);
        assert_eq!(back.gm.bearing, "S30W");
        assert_eq!((back.gm.e, back.gm.n, back.gm.z), (500_123.5, 8_500_456.25, 4_012.0));
        assert_eq!(
            (back.gm.dip.as_str(), back.gm.dip_dir, back.gm.dip_az.as_str()),
            ("65", 2, "N30E")
        );
        assert_eq!((back.gm.proj_mode, back.gm.proj_value), (1, -12.5));
        assert_eq!(back.dh.custom_ranges, c.dh.custom_ranges);
        assert_eq!((back.ax.radius, back.ax.turn), (22.5, 1));
        assert_eq!((back.sc.mode, back.sc.columns), (2, 3.0));
    }
}
