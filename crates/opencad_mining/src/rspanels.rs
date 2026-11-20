//! Panels of the resource-estimation modules.

use std::collections::HashMap;

use ocs_plugin_api::dialog::{DialogControl, DialogValue, PanelDef};

use crate::config::ProjectConfig;
use crate::rsi18n::RsStrings;

pub const COMP: &str = "min_rs_comp";
pub const VARIO: &str = "min_rs_vario";
pub const BM: &str = "min_rs_bm";
pub const ALL: [&str; 3] = [COMP, VARIO, BM];

pub const COMP_STATUS: &str = "rs_comp_status";
pub const COMP_CHART: &str = "rs_comp_chart";
pub const VARIO_STATUS: &str = "rs_v_status";
pub const VARIO_CHART: &str = "rs_v_chart";
pub const BM_STATUS: &str = "rs_b_status";
pub const BM_CHART: &str = "rs_b_chart";

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
fn status(id: &str) -> DialogControl {
    DialogControl::Label {
        id: id.into(),
        text: String::new(),
        align_h: None,
    }
}
fn chart(id: &str, svg: &str) -> DialogControl {
    DialogControl::Svg {
        id: id.into(),
        svg: svg.into(),
        height: Some(230.0),
    }
}
fn triple(prefix: &str, label: &str, v: [f64; 3], min: Option<f64>, step: f64) -> Vec<DialogControl> {
    ["x", "y", "z"]
        .iter()
        .enumerate()
        .map(|(i, a)| {
            num(
                &format!("{prefix}{a}"),
                &format!("{label} {}", a.to_uppercase()),
                v[i],
                min,
                step,
            )
        })
        .collect()
}

fn panel(id: &str, title: &str, command: &str, controls: Vec<DialogControl>) -> PanelDef {
    let mut p = PanelDef::new(id, title);
    p.command = Some(command.into());
    p.width = Some(340.0);
    p.controls = controls;
    p
}

/// `svg` is the current chart (re-registering the panel with another one
/// refreshes the image).
pub fn comp(r: &RsStrings, c: &ProjectConfig, svg: &str) -> PanelDef {
    let x = &c.rs;
    panel(
        COMP,
        r.p_comp,
        "MIN_RS_COMP_PANEL",
        vec![
            fields(
                "rs_f_comp",
                r.sec_composite,
                vec![
                    num("rs_comp_len", r.comp_len, x.comp_length, Some(0.01), 1.0),
                    choice("rs_comp_rem", r.comp_rem, &r.comp_rems, x.comp_remainder),
                    num("rs_comp_minfrac", r.comp_minfrac, x.comp_min_fraction, Some(0.0), 0.1),
                    num("rs_comp_mincov", r.comp_mincov, x.comp_min_coverage, Some(0.0), 0.1),
                    text("rs_unit", r.unit, &x.unit),
                ],
            ),
            fields(
                "rs_f_dom",
                r.sec_domains,
                vec![DialogControl::Table {
                    id: "rs_domains".into(),
                    label: None,
                    headers: r.domain_headers.iter().map(|s| s.to_string()).collect(),
                    rows: Vec::new(),
                    allow_add: true,
                    allow_delete: true,
                }],
            ),
            fields(
                "rs_f_dc",
                r.sec_decluster,
                vec![
                    num("rs_dc_min", r.dc_min, x.dc_min, Some(0.1), 5.0),
                    num("rs_dc_max", r.dc_max, x.dc_max, Some(0.1), 10.0),
                    num("rs_dc_steps", r.dc_steps, x.dc_steps, Some(1.0), 1.0),
                    num("rs_dc_z", r.dc_z, x.dc_zratio, Some(0.01), 0.1),
                ],
            ),
            button("rs_btn_comp", r.btn_composite, "MIN_RS_COMPOSITE"),
            status(COMP_STATUS),
            chart(COMP_CHART, svg),
            row(vec![button("rs_btn_drawcomp", r.btn_drawcomp, "MIN_RS_DRAWCOMP")]),
            fields(
                "rs_f_solid",
                r.sec_solids,
                vec![
                    choice("rs_target", r.target_domain, &[], 0),
                    button("rs_btn_setsolid", r.btn_setsolid, "MIN_RS_SETSOLID"),
                ],
            ),
            fields(
                "rs_f_ct",
                r.sec_contact,
                vec![
                    text("rs_ct_above", r.ct_above, &x.contact.above),
                    text("rs_ct_below", r.ct_below, &x.contact.below),
                    num("rs_ct_step", r.ct_step, x.contact.step, Some(0.1), 5.0),
                    choice("rs_ct_kind", r.ct_kind, &r.ct_kinds, x.contact.kind),
                    num("rs_ct_nugget", r.ct_nugget, x.contact.nugget, Some(0.0), 1.0),
                    num("rs_ct_sill", r.ct_sill, x.contact.sill, Some(0.0), 10.0),
                    num("rs_ct_range", r.ct_range, x.contact.range, Some(1.0), 10.0),
                    num("rs_ct_radius", r.ct_radius, x.contact.radius, Some(1.0), 10.0),
                    num("rs_ct_min", r.ct_min, x.contact.min_samples, Some(1.0), 1.0),
                    num("rs_ct_max", r.ct_max, x.contact.max_samples, Some(1.0), 1.0),
                    button("rs_btn_contact", r.btn_contact, "MIN_RS_CONTACT"),
                ],
            ),
        ],
    )
}

fn structure_controls(r: &RsStrings, k: usize, title: &str) -> DialogControl {
    let id = |s: &str| format!("rs_m_s{k}_{s}");
    fields(
        &format!("rs_f_s{k}"),
        title,
        vec![
            choice(&id("kind"), r.v_kind, &r.ct_kinds, 0),
            num(&id("sill"), r.v_sill, 0.0, Some(0.0), 1.0),
            num(&id("r1"), r.v_r1, 100.0, Some(0.1), 10.0),
            num(&id("r2"), r.v_r2, 100.0, Some(0.1), 10.0),
            num(&id("r3"), r.v_r3, 50.0, Some(0.1), 10.0),
        ],
    )
}

pub fn vario(r: &RsStrings, c: &ProjectConfig, svg: &str) -> PanelDef {
    let v = &c.rs.vario;
    panel(
        VARIO,
        r.p_vario,
        "MIN_RS_VARIO_PANEL",
        vec![
            choice("rs_v_domain", r.v_domain, &[], v.domain),
            fields(
                "rs_f_vexp",
                r.sec_vexp,
                vec![
                    num("rs_m_az", r.v_az, v.work.azimuth, None, 5.0),
                    num("rs_m_dip", r.v_dip, v.work.dip, None, 5.0),
                    num("rs_m_rake", r.v_rake, v.work.rake, None, 5.0),
                    num("rs_v_lag", r.v_lag, v.lag, Some(0.1), 5.0),
                    num("rs_v_nlags", r.v_nlags, v.nlags, Some(1.0), 1.0),
                    num("rs_v_aztol", r.v_aztol, v.az_tol, Some(0.0), 2.5),
                    num("rs_v_diptol", r.v_diptol, v.dip_tol, Some(0.0), 2.5),
                    num("rs_v_bwh", r.v_bwh, v.bw_h, Some(0.0), 5.0),
                    num("rs_v_bwv", r.v_bwv, v.bw_v, Some(0.0), 5.0),
                    check("rs_v_omni", r.v_omni, v.omni),
                    button("rs_btn_vario", r.btn_vario, "MIN_RS_VARIO"),
                ],
            ),
            status(VARIO_STATUS),
            chart(VARIO_CHART, svg),
            fields(
                "rs_f_vmodel",
                r.sec_vmodel,
                vec![
                    num("rs_m_nugget", r.v_nugget, v.work.nugget, Some(0.0), 0.5),
                    structure_controls(r, 1, r.v_struct1),
                    structure_controls(r, 2, r.v_struct2),
                ],
            ),
            row(vec![button("rs_btn_vupdate", r.btn_vupdate, "MIN_RS_VARIOUPDATE")]),
            fields(
                "rs_f_vfit",
                r.sec_vfit,
                vec![
                    choice("rs_v_fitkind", r.v_fitkind, &r.ct_kinds, v.fit_kind1),
                    check("rs_v_fittwo", r.v_fittwo, v.fit_two),
                    check("rs_v_fixnug", r.v_fixnug, v.fix_nugget),
                    button("rs_btn_fit", r.btn_fit, "MIN_RS_FIT"),
                ],
            ),
            row(vec![
                button("rs_btn_msave", r.btn_msave, "MIN_RS_MODELSAVE"),
                button("rs_btn_mload", r.btn_mload, "MIN_RS_MODELLOAD"),
            ]),
            button("rs_btn_cv", r.btn_crossval, "MIN_RS_CROSSVAL"),
        ],
    )
}

pub fn blocks(r: &RsStrings, c: &ProjectConfig, svg: &str) -> PanelDef {
    let b = &c.rs.bm;
    let mut geometry = vec![check("rs_b_auto", r.b_auto, b.auto_extent)];
    geometry.extend(triple("rs_b_o", r.b_origin, b.origin, None, 10.0));
    geometry.extend(triple("rs_b_n", r.b_counts, b.counts, Some(1.0), 1.0));
    geometry.push(num("rs_b_pad", r.b_pad, b.pad, Some(0.0), 10.0));
    geometry.extend(triple("rs_b_s", r.b_size, b.size, Some(0.01), 1.0));
    geometry.push(num("rs_b_rot", r.b_rot, b.rotation, None, 5.0));
    geometry.extend(triple("rs_b_d", r.b_disc, b.disc, Some(1.0), 1.0));
    panel(
        BM,
        r.p_bm,
        "MIN_RS_BM_PANEL",
        vec![
            fields("rs_f_geo", r.sec_geometry, geometry),
            fields(
                "rs_f_meth",
                r.sec_methods,
                vec![
                    check("rs_b_ok", r.b_ok, b.ordinary),
                    check("rs_b_sk", r.b_sk, b.simple),
                    check("rs_b_idw", r.b_idw, b.idw),
                    num("rs_b_idwp", r.b_idwp, b.idw_power, Some(0.1), 0.5),
                    check("rs_b_nn", r.b_nn, b.nearest),
                ],
            ),
            fields(
                "rs_f_search",
                r.sec_search,
                vec![
                    num("rs_b_r1", &format!("{} 1", r.b_radii), b.radii[0], Some(0.1), 10.0),
                    num("rs_b_r2", &format!("{} 2", r.b_radii), b.radii[1], Some(0.1), 10.0),
                    num("rs_b_r3", &format!("{} 3", r.b_radii), b.radii[2], Some(0.1), 10.0),
                    num("rs_b_min", r.b_min, b.min_samples, Some(1.0), 1.0),
                    num("rs_b_max", r.b_max, b.max_samples, Some(1.0), 1.0),
                    num("rs_b_oct", r.b_oct, b.max_per_octant, Some(0.0), 1.0),
                    num("rs_b_hole", r.b_hole, b.max_per_hole, Some(0.0), 1.0),
                    num("rs_b_p2", r.b_p2, b.pass2, Some(0.0), 0.5),
                    num("rs_b_p2min", r.b_p2min, b.pass2_min, Some(1.0), 1.0),
                    num("rs_b_p3", r.b_p3, b.pass3, Some(0.0), 0.5),
                    num("rs_b_p3min", r.b_p3min, b.pass3_min, Some(1.0), 1.0),
                    num("rs_b_minfrac", r.b_minfrac, b.min_fraction, Some(0.0), 0.05),
                ],
            ),
            row(vec![
                button("rs_btn_bmc", r.btn_bmcreate, "MIN_RS_BMCREATE"),
                button("rs_btn_est", r.btn_estimate, "MIN_RS_ESTIMATE"),
            ]),
            status(BM_STATUS),
            fields(
                "rs_f_chart",
                r.sec_chart,
                vec![
                    choice("rs_w_chart", r.w_chart, &r.w_charts, b.chart),
                    choice("rs_w_axis", r.w_axis, &r.w_axes, b.swath_axis),
                    num("rs_w_swath", r.w_swath, b.swath_width, Some(0.1), 10.0),
                    button("rs_btn_val", r.btn_validate, "MIN_RS_VALIDATE"),
                    chart(BM_CHART, svg),
                ],
            ),
            fields(
                "rs_f_view",
                r.sec_view,
                vec![
                    choice("rs_w_mode", r.w_mode, &r.w_modes, b.view_mode),
                    choice("rs_w_attr", r.w_attr, &r.w_attrs, b.view_attr),
                    num("rs_w_cutoff", r.w_cutoff, b.view_cutoff, None, 0.5),
                    choice("rs_w_domain", r.w_domain, &[], b.view_domain),
                    num("rs_w_zfrom", r.w_zfrom, b.view_z_from, None, 12.0),
                    num("rs_w_zto", r.w_zto, b.view_z_to, None, 12.0),
                    num("rs_w_zstep", r.w_zstep, b.view_z_step, Some(0.1), 1.0),
                    num("rs_w_spacing", r.w_spacing, b.view_spacing, Some(0.1), 5.0),
                    num("rs_w_count", r.w_count, b.view_count, Some(1.0), 1.0),
                    num("rs_w_interval", r.w_interval, b.view_interval, Some(0.1), 5.0),
                    num("rs_w_limit", r.w_limit, b.view_limit, Some(1.0), 1000.0),
                    button("rs_btn_view", r.btn_bmview, "MIN_RS_BMVIEW"),
                ],
            ),
            fields(
                "rs_f_store",
                r.sec_storage,
                vec![
                    text("rs_b_folder", r.b_folder, &b.folder),
                    row(vec![
                        button("rs_btn_load", r.btn_bmload, "MIN_RS_BMLOAD"),
                        button("rs_btn_export", r.btn_bmexport, "MIN_RS_BMEXPORT"),
                    ]),
                ],
            ),
        ],
    )
}

/// Values to push into the resource panels. `domain_options` fills the
/// domain choices (the view filter gets an "all" entry first).
pub fn values_for(c: &ProjectConfig, r: &RsStrings) -> HashMap<&'static str, HashMap<String, DialogValue>> {
    let x = &c.rs;
    let n = DialogValue::Number;
    let b = DialogValue::Bool;
    let t = |s: &str| DialogValue::Text(s.to_string());
    let ch = |i: usize| DialogValue::Choice {
        index: i,
        text: String::new(),
    };
    let names = x.domain_names();
    let opts = |list: Vec<String>, sel: usize| DialogValue::ChoiceOptions {
        selected_index: sel.min(list.len().saturating_sub(1)),
        options: list,
    };
    let mut with_all = vec![r.w_all.to_string()];
    with_all.extend(names.clone());
    let collect = |v: Vec<(&str, DialogValue)>| -> HashMap<String, DialogValue> {
        v.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
    };

    let mut comp = collect(vec![
        ("rs_comp_len", n(x.comp_length)),
        ("rs_comp_rem", ch(x.comp_remainder)),
        ("rs_comp_minfrac", n(x.comp_min_fraction)),
        ("rs_comp_mincov", n(x.comp_min_coverage)),
        ("rs_unit", t(&x.unit)),
        ("rs_dc_min", n(x.dc_min)),
        ("rs_dc_max", n(x.dc_max)),
        ("rs_dc_steps", n(x.dc_steps)),
        ("rs_dc_z", n(x.dc_zratio)),
        ("rs_ct_above", t(&x.contact.above)),
        ("rs_ct_below", t(&x.contact.below)),
        ("rs_ct_step", n(x.contact.step)),
        ("rs_ct_kind", ch(x.contact.kind)),
        ("rs_ct_nugget", n(x.contact.nugget)),
        ("rs_ct_sill", n(x.contact.sill)),
        ("rs_ct_range", n(x.contact.range)),
        ("rs_ct_radius", n(x.contact.radius)),
        ("rs_ct_min", n(x.contact.min_samples)),
        ("rs_ct_max", n(x.contact.max_samples)),
    ]);
    let side = |s: usize| ["", r.word_above, r.word_below][s.min(2)].to_string();
    comp.insert(
        "rs_domains".into(),
        DialogValue::Table(
            x.domains
                .iter()
                .map(|d| {
                    vec![
                        d.name.clone(),
                        d.values.clone(),
                        d.density.to_string(),
                        d.cap.to_string(),
                        side(d.side),
                    ]
                })
                .collect(),
        ),
    );
    if !names.is_empty() {
        comp.insert("rs_target".into(), opts(names.clone(), 0));
    }

    let v = &x.vario;
    let w = &v.work;
    let mut vario = collect(vec![
        ("rs_m_az", n(w.azimuth)),
        ("rs_m_dip", n(w.dip)),
        ("rs_m_rake", n(w.rake)),
        ("rs_v_lag", n(v.lag)),
        ("rs_v_nlags", n(v.nlags)),
        ("rs_v_aztol", n(v.az_tol)),
        ("rs_v_diptol", n(v.dip_tol)),
        ("rs_v_bwh", n(v.bw_h)),
        ("rs_v_bwv", n(v.bw_v)),
        ("rs_v_omni", b(v.omni)),
        ("rs_m_nugget", n(w.nugget)),
        ("rs_v_fitkind", ch(v.fit_kind1)),
        ("rs_v_fittwo", b(v.fit_two)),
        ("rs_v_fixnug", b(v.fix_nugget)),
    ]);
    for (i, s) in w.structures.iter().enumerate() {
        let k = i + 1;
        vario.insert(format!("rs_m_s{k}_kind"), ch(s.kind));
        vario.insert(format!("rs_m_s{k}_sill"), n(s.sill));
        for (a, rg) in s.ranges.iter().enumerate() {
            vario.insert(format!("rs_m_s{k}_r{}", a + 1), n(*rg));
        }
    }
    if !names.is_empty() {
        vario.insert("rs_v_domain".into(), opts(names.clone(), v.domain));
    }

    let m = &x.bm;
    let mut bm = collect(vec![
        ("rs_b_auto", b(m.auto_extent)),
        ("rs_b_pad", n(m.pad)),
        ("rs_b_rot", n(m.rotation)),
        ("rs_b_ok", b(m.ordinary)),
        ("rs_b_sk", b(m.simple)),
        ("rs_b_idw", b(m.idw)),
        ("rs_b_idwp", n(m.idw_power)),
        ("rs_b_nn", b(m.nearest)),
        ("rs_b_min", n(m.min_samples)),
        ("rs_b_max", n(m.max_samples)),
        ("rs_b_oct", n(m.max_per_octant)),
        ("rs_b_hole", n(m.max_per_hole)),
        ("rs_b_p2", n(m.pass2)),
        ("rs_b_p2min", n(m.pass2_min)),
        ("rs_b_p3", n(m.pass3)),
        ("rs_b_p3min", n(m.pass3_min)),
        ("rs_b_minfrac", n(m.min_fraction)),
        ("rs_b_folder", t(&m.folder)),
        ("rs_w_mode", ch(m.view_mode)),
        ("rs_w_attr", ch(m.view_attr)),
        ("rs_w_cutoff", n(m.view_cutoff)),
        ("rs_w_zfrom", n(m.view_z_from)),
        ("rs_w_zto", n(m.view_z_to)),
        ("rs_w_zstep", n(m.view_z_step)),
        ("rs_w_spacing", n(m.view_spacing)),
        ("rs_w_count", n(m.view_count)),
        ("rs_w_interval", n(m.view_interval)),
        ("rs_w_limit", n(m.view_limit)),
        ("rs_w_axis", ch(m.swath_axis)),
        ("rs_w_swath", n(m.swath_width)),
        ("rs_w_chart", ch(m.chart)),
        ("rs_w_domain", opts(with_all, m.view_domain)),
    ]);
    for (a, name) in ["x", "y", "z"].iter().enumerate() {
        bm.insert(format!("rs_b_o{name}"), n(m.origin[a]));
        bm.insert(format!("rs_b_n{name}"), n(m.counts[a]));
        bm.insert(format!("rs_b_s{name}"), n(m.size[a]));
        bm.insert(format!("rs_b_d{name}"), n(m.disc[a]));
        bm.insert(format!("rs_b_r{}", a + 1), n(m.radii[a]));
    }
    HashMap::from([(COMP, comp), (VARIO, vario), (BM, bm)])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Lang;
    use crate::rsi18n::for_lang;

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

    /// Every pushed value addresses a declared control, and every editable
    /// control of the config is declared (the config reads what the panel shows).
    #[test]
    fn pushed_values_match_declared_controls_in_every_language() {
        for l in [Lang::Es, Lang::En, Lang::Pt] {
            let r = for_lang(l);
            let mut c = ProjectConfig::default();
            c.rs.domains = vec![crate::rsconfig::DomainCfg {
                name: "Óxidos".into(),
                ..Default::default()
            }];
            let panels = [comp(r, &c, "<svg/>"), vario(r, &c, "<svg/>"), blocks(r, &c, "<svg/>")];
            let values = values_for(&c, r);
            for p in &panels {
                let ids = declared(p);
                for k in values[p.id.as_str()].keys() {
                    assert!(ids.contains(k), "{k} not declared in {} ({l:?})", p.id);
                }
            }
        }
    }

    #[test]
    fn panel_values_roundtrip_into_the_config() {
        let r = for_lang(Lang::Es);
        let mut c = ProjectConfig::default();
        c.rs.comp_length = 2.0;
        c.rs.domains = vec![
            crate::rsconfig::DomainCfg {
                name: "Óxidos".into(),
                values: "OXIDO".into(),
                density: 2.4,
                cap: 30.0,
                side: 1,
            },
            crate::rsconfig::DomainCfg {
                name: "Sulfuros".into(),
                values: "SULFURO".into(),
                density: 2.8,
                cap: 0.0,
                side: 2,
            },
        ];
        c.rs.vario.work.structures[1].ranges[2] = 33.0;
        c.rs.bm.size = [5.0, 6.0, 7.0];
        c.rs.bm.radii = [300.0, 200.0, 40.0];
        c.rs.bm.simple = true;
        let mut flat = HashMap::new();
        for (_, m) in values_for(&c, r) {
            flat.extend(m);
        }
        let mut back = ProjectConfig::default();
        back.apply(&flat);
        assert_eq!(back.rs.comp_length, 2.0);
        assert_eq!(back.rs.domains, c.rs.domains);
        assert_eq!(back.rs.vario.work.structures[1].ranges[2], 33.0);
        assert_eq!(
            (back.rs.bm.size, back.rs.bm.radii, back.rs.bm.simple),
            ([5.0, 6.0, 7.0], [300.0, 200.0, 40.0], true)
        );
    }
}
