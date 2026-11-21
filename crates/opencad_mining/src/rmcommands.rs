//! Handlers of the geotechnical phases 3 and 4 (`st_*`, `sm_*`; see
//! `openmining_ops`): structures (stereonet, sets, kinematics, wedges) and
//! stresses and design (in situ stress, Kirsch, convergence-confinement,
//! pillars, crown pillars).

use geotech_engine::convergence::{self, Ground};
use geotech_engine::hoek_brown::{rock_mass_modulus, sigma3_max, Application, HoekBrown};
use geotech_engine::kinematic::{self, Face, Report};
use geotech_engine::kirsch;
use geotech_engine::pillar::{self, Formula, Layout, Shape};
use geotech_engine::sets::find_sets;
use geotech_engine::stereonet::{density, Line, Plane};
use geotech_engine::stress::{section, InSitu};
use geotech_engine::wedge::{self, FaceKind, Joint, Opening};
use ocs_plugin_api::host::{Handle, HostApi};
use openmining_ops::parse::Args;
use openmining_ops::result::{ErrorCode, OpError, OpResult};
use serde_json::{json, Value};

use crate::cad::{CadSink, LayerDef};
use crate::charts::{render, Chart, Series, Style};
use crate::config::ProjectConfig;
use crate::host_sink::{tag_of, HostSink};
use crate::i18n::fmt;
use crate::opsrun::Outcome;
use crate::rmconfig::JointSetCfg;
use crate::rmi18n::{m, RmStrings};
use crate::rockmech as rk;
use crate::state;
use crate::xdata::{Class, Tag, APP};
use crate::{session, MANIFEST};

pub const STRUCT: &str = "min_rm_struct";
pub const DESIGN: &str = "min_rm_design";
const ST_STATUS: &str = "rm_st_status";
const ST_CHART: &str = "rm_st_chart";
const SM_STATUS: &str = "rm_sm_status";
const SM_CHART: &str = "rm_sm_chart";

/// Last design chart of a tab (drawn by `sm_draw`).
#[derive(Debug, Clone, Default)]
pub struct RmState {
    /// Index 0 Kirsch, 1 convergence: (title, x label, y label, series).
    pub charts: [Option<rk::SavedChart>; 2],
}

fn no_data(msg: impl Into<String>) -> OpError {
    OpError::new(ErrorCode::NoData, msg)
}

fn f(v: f64, d: usize) -> String {
    if v.is_finite() {
        format!("{v:.d$}")
    } else {
        "∞".into()
    }
}

fn finite(v: f64) -> Value {
    if v.is_finite() {
        json!(v)
    } else {
        Value::Null
    }
}

fn point(args: &Args, key: &str) -> Option<mining_core::Vec3> {
    let a = args.get(key)?.as_array()?;
    Some([
        a.first()?.as_f64()?,
        a.get(1)?.as_f64()?,
        a.get(2).and_then(Value::as_f64).unwrap_or(0.0),
    ])
}

fn handle(args: &Args, key: &str) -> Option<Handle> {
    args.get(key)
        .and_then(Value::as_str)
        .and_then(|h| u64::from_str_radix(h, 16).ok())
        .map(Handle::new)
}

/// Tags of every plugin entity (paged query).
fn plugin_tags(host: &mut dyn HostApi) -> Vec<Tag> {
    let mut out = Vec::new();
    let mut offset = 0u64;
    loop {
        let q = format!(r#"{{"xdata_app":"{APP}","offset":{offset},"limit":5000}}"#);
        let Ok(page) = host.query_entities(&q) else { break };
        out.extend(page.entities.iter().filter_map(tag_of));
        match page.next_offset {
            Some(n) if n > offset => offset = n,
            _ => break,
        }
    }
    out
}

/// Poles of the sources chosen: `(planes, from table, from traces)`.
fn poles(host: &mut dyn HostApi, tab: u64, cfg: &ProjectConfig) -> Result<(Vec<Plane>, usize, usize), OpError> {
    let mut planes = Vec::new();
    if cfg.rm.src_joints {
        // Tables are reloaded from the project files after reopening the drawing.
        if state::with_tab(tab, |t| t.gt.joints.is_none()) && !cfg.gt.joints.trim().is_empty() {
            let mut gc = cfg.gt.clone();
            if let Ok((st, _)) = crate::geotech::load(crate::gti18n::g(), &mut gc, &mut |_, _, _| None) {
                state::with_tab(tab, |t| t.gt = st);
            }
        }
        if let Some(t) = state::with_tab(tab, |t| t.gt.joints.clone()) {
            planes.extend(rk::table_planes(&t));
        }
    }
    let from_table = planes.len();
    if cfg.rm.src_traces {
        planes.extend(rk::trace_planes(&plugin_tags(host)));
    }
    if planes.is_empty() {
        return Err(no_data(m().m_no_poles));
    }
    let n = planes.len();
    Ok((planes, from_table, n - from_table))
}

fn set_planes(cfg: &ProjectConfig) -> Vec<(String, Plane)> {
    cfg.rm
        .sets
        .iter()
        .map(|s| (s.name.clone(), Plane::new(s.dip, s.dipdir)))
        .collect()
}

/// Re-registers a panel with a new chart (the host rebuilds a changed panel
/// hidden and with defaults: show it and restore the values).
fn refresh(
    host: &mut dyn HostApi,
    cfg: &ProjectConfig,
    panel: &'static str,
    chart: (&str, &str),
    status: (&str, String),
) {
    let lists = crate::opsbind::Lists::of(host.tab_id(), cfg);
    if host.register_panel_for_plugin(MANIFEST.id, crate::opsbind::panel_with(panel, cfg, &lists, &[chart])) {
        host.toggle_panel_for_plugin(MANIFEST.id, panel, Some(true));
    }
    crate::opsbind::push_one(host, cfg, panel, Some(status));
}

fn commit(host: &mut dyn HostApi, tab: u64, cfg: &ProjectConfig, label: &str) {
    let mut sink = HostSink::new(host);
    sink.begin(label);
    session::commit(&mut sink, crate::i18n::s(), tab, cfg);
    sink.end();
}

fn report(host: &mut dyn HostApi, lines: &[String]) {
    lines.iter().for_each(|l| host.push_info(l));
}

fn stereonet(
    host: &mut dyn HostApi,
    tab: u64,
    cfg: &ProjectConfig,
    face: Option<Plane>,
    status: Vec<String>,
) -> Result<Value, OpError> {
    let s = m();
    let (planes, nt, ntr) = poles(host, tab, cfg)?;
    let sets = set_planes(cfg);
    let vecs: Vec<_> = planes.iter().map(|p| p.pole().vector()).collect();
    let max = vecs.iter().map(|v| density(&vecs, *v)).fold(0.0, f64::max);
    let line = fmt(s.m_stereo, &[&planes.len(), &nt, &ntr, &f(max, 1)]);
    let svg = rk::stereonet_svg(&planes, &sets, cfg.rm.contours, face);
    let mut text = vec![line.clone()];
    text.extend(status);
    host.push_info(&line);
    refresh(host, cfg, STRUCT, (ST_CHART, &svg), (ST_STATUS, text.join("\n")));
    Ok(json!({"poles": planes.len(), "from_table": nt, "from_traces": ntr, "max_density": max, "sets": sets.len()}))
}

fn sets_op(host: &mut dyn HostApi, tab: u64, cfg: &mut ProjectConfig) -> Outcome {
    let s = m();
    let (planes, _, _) = poles(host, tab, cfg)?;
    let vecs: Vec<_> = planes.iter().map(|p| p.pole().vector()).collect();
    let found = find_sets(&vecs, cfg.rm.n_sets, cfg.rm.min_angle, cfg.rm.max_angle);
    if found.is_empty() {
        return Err(no_data(s.m_no_sets));
    }
    let old = cfg.rm.sets.clone();
    cfg.rm.sets = found
        .iter()
        .enumerate()
        .map(|(i, st)| {
            let prev = old.get(i);
            JointSetCfg {
                name: format!("J{}", i + 1),
                dip: (st.plane.dip * 10.0).round() / 10.0,
                dipdir: (st.plane.dipdir * 10.0).round() / 10.0,
                friction: prev.map_or(cfg.rm.friction, |p| p.friction),
                cohesion: prev.map_or(0.0, |p| p.cohesion),
            }
        })
        .collect();
    let mut lines: Vec<String> = found
        .iter()
        .enumerate()
        .map(|(i, st)| {
            fmt(
                s.m_set,
                &[
                    &format!("J{}", i + 1),
                    &format!("{:02.0}", st.plane.dip),
                    &format!("{:03.0}", st.plane.dipdir),
                    &st.count,
                    &f(st.k, 0),
                    &f(st.alpha95, 1),
                ],
            )
        })
        .collect();
    let assigned: usize = found.iter().map(|x| x.count).sum();
    if assigned < planes.len() {
        lines.push(fmt(s.m_unassigned, &[&(planes.len() - assigned)]));
    }
    report(host, &lines);
    commit(host, tab, cfg, "MIN_ST_SETS");
    crate::opsbind::push_all(host, cfg);
    stereonet(host, tab, cfg, None, lines)?;
    let sets: Vec<Value> = found
        .iter()
        .enumerate()
        .map(|(i, st)| json!({"name": format!("J{}", i + 1), "dip": st.plane.dip, "dipdir": st.plane.dipdir, "count": st.count, "k": finite(st.k), "alpha95": st.alpha95}))
        .collect();
    Ok(OpResult::data(
        json!({"sets": sets, "unassigned": planes.len() - assigned}),
    ))
}

fn kinematic_op(host: &mut dyn HostApi, tab: u64, cfg: &ProjectConfig) -> Outcome {
    let s = m();
    let (planes, _, _) = poles(host, tab, cfg)?;
    let sets = set_planes(cfg);
    let face = Face {
        dip: cfg.rm.face_dip,
        dipdir: cfg.rm.face_dipdir,
        friction: cfg.rm.friction,
        lateral: cfg.rm.lateral,
    };
    let only: Vec<Plane> = sets.iter().map(|x| x.1).collect();
    let r = kinematic::analyse(&face, &planes, &only);
    let names = |idx: &[usize]| {
        if idx.is_empty() {
            "-".to_string()
        } else {
            idx.iter().map(|i| sets[*i].0.clone()).collect::<Vec<_>>().join(", ")
        }
    };
    let pairs = if r.wedge_sets.is_empty() {
        "-".to_string()
    } else {
        r.wedge_sets
            .iter()
            .map(|(a, b)| format!("{}×{}", sets[*a].0, sets[*b].0))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let lines = vec![
        fmt(
            s.m_kin,
            &[
                &f(face.dip, 0),
                &f(face.dipdir, 0),
                &f(face.friction, 0),
                &f(Report::pct(r.planar, r.planes), 1),
                &f(Report::pct(r.wedge, r.intersections), 1),
                &f(Report::pct(r.flexural, r.planes), 1),
            ],
        ),
        fmt(
            s.m_kin_sets,
            &[&names(&r.planar_sets), &pairs, &names(&r.flexural_sets)],
        ),
    ];
    report(host, &lines);
    stereonet(host, tab, cfg, Some(Plane::new(face.dip, face.dipdir)), lines)?;
    let mode = |n: usize, of: usize| json!({"critical": n, "of": of, "pct": Report::pct(n, of)});
    Ok(OpResult::data(json!({
        "planar": mode(r.planar, r.planes),
        "wedge": mode(r.wedge, r.intersections),
        "flexural": mode(r.flexural, r.planes),
        "planar_sets": r.planar_sets.iter().map(|i| sets[*i].0.clone()).collect::<Vec<_>>(),
        "wedge_sets": r.wedge_sets.iter().map(|(a, b)| [sets[*a].0.clone(), sets[*b].0.clone()]).collect::<Vec<_>>(),
        "flexural_sets": r.flexural_sets.iter().map(|i| sets[*i].0.clone()).collect::<Vec<_>>(),
    })))
}

fn mode_text(s: &RmStrings, mode: &wedge::Mode, names: &[String; 3]) -> String {
    match mode {
        wedge::Mode::Stable => s.modes[0].into(),
        wedge::Mode::Falling => s.modes[1].into(),
        wedge::Mode::Sliding1(i) => fmt(s.modes[2], &[&names[*i]]),
        wedge::Mode::Sliding2(i, j) => fmt(s.modes[3], &[&names[*i], &names[*j]]),
    }
}

fn wedges_op(host: &mut dyn HostApi, args: &Args, cfg: &ProjectConfig) -> Outcome {
    let s = m();
    let x = &cfg.rm;
    let idx = [x.set_a, x.set_b, x.set_c];
    let chosen: Option<Vec<&JointSetCfg>> = idx
        .iter()
        .map(|i| i.checked_sub(1).and_then(|k| x.sets.get(k)))
        .collect();
    let chosen = match chosen {
        Some(c) if idx[0] != idx[1] && idx[0] != idx[2] && idx[1] != idx[2] => c,
        _ => return Err(no_data(s.m_need_sets)),
    };
    let joints: [Joint; 3] = std::array::from_fn(|k| Joint {
        plane: Plane::new(chosen[k].dip, chosen[k].dipdir),
        friction: chosen[k].friction,
        cohesion: chosen[k].cohesion,
    });
    let names: [String; 3] = std::array::from_fn(|k| chosen[k].name.clone());
    // Axis direction and drawing point: an axis polyline at the chainage, a point, or none.
    let mut place = point(args, "at");
    let (mut trend, mut plunge) = (x.axis_trend, x.axis_plunge);
    if let Some(h) = handle(args, "axis") {
        let pts = crate::fetch_points(host, &[h])
            .and_then(|mut v| v.pop())
            .ok_or_else(|| OpError::new(ErrorCode::NotFound, crate::i18n::s().m_not_polyline).at("axis"))?;
        let (p, t, pl) = rk::axis_at(&pts, x.chainage)
            .ok_or_else(|| OpError::new(ErrorCode::NotFound, crate::i18n::s().m_not_polyline).at("axis"))?;
        place = Some(p);
        (trend, plunge) = (t, pl);
    }
    let o = Opening {
        axis: Line { trend, plunge },
        width: x.width,
        height: x.height,
        max_length: x.max_length,
    };
    let mut lines = Vec::new();
    let mut list = Vec::new();
    let mut data = Vec::new();
    for (k, face) in FaceKind::ALL.iter().enumerate() {
        match wedge::wedge(&o, *face, &joints, x.gamma, x.support) {
            None => lines.push(fmt(s.m_no_wedge, &[&s.faces[k]])),
            Some(w) => {
                lines.push(fmt(
                    s.m_wedge,
                    &[
                        &s.faces[k],
                        &mode_text(s, &w.mode, &names),
                        &w.fs.map_or_else(|| "∞".to_string(), |v| f(v, 2)),
                        &f(w.volume, 2),
                        &f(w.weight, 1),
                        &f(w.apex_height, 2),
                    ],
                ));
                data.push(json!({"face": face.key(), "mode": w.mode.key(), "fs": w.fs, "volume": w.volume, "weight": w.weight, "apex_height": w.apex_height, "base_area": w.base_area}));
                list.push(w);
            }
        }
    }
    report(host, &lines);
    let mut created = Vec::new();
    let mut drawn = 0;
    if let Some(p) = place {
        let prims = rk::wedge_prims(
            &o,
            &list,
            p,
            cfg.ug.v_align,
            &names.join(","),
            crate::drillholes::origin(cfg),
        );
        let mut sink = HostSink::new(host);
        sink.begin("MIN_ST_WEDGES");
        sink.ensure_layers(&[LayerDef::new(rk::LAYER_WEDGES, 30)]);
        sink.remove_where(&|t: &Tag| t.class == Class::RmWedge);
        drawn = sink.add(prims);
        sink.end();
        created = std::mem::take(&mut sink.created);
    }
    crate::opsbind::push_one(host, cfg, STRUCT, Some((ST_STATUS, lines.join("\n"))));
    Ok(OpResult {
        created,
        ..OpResult::data(json!({"sets": names, "axis": [trend, plunge], "wedges": data, "drawn": drawn}))
    })
}

fn insitu(cfg: &ProjectConfig) -> InSitu {
    let x = &cfg.rm;
    InSitu {
        gamma: x.gamma,
        depth: x.depth,
        k_h: x.k_h,
        k_hmin: x.k_hmin,
        az_h: x.az_h,
    }
}

fn hoek_brown(cfg: &ProjectConfig) -> Result<HoekBrown, OpError> {
    let g = &cfg.gt;
    HoekBrown::new(g.hb_sigci, g.hb_gsi, g.hb_mi, g.hb_d)
        .map_err(|e| OpError::new(ErrorCode::OutOfRange, format!("Hoek-Brown: {e}")))
}

fn radius(cfg: &ProjectConfig) -> (f64, bool) {
    if cfg.rm.opening == 1 {
        (kirsch::equivalent_radius(cfg.rm.width, cfg.rm.height), true)
    } else {
        (cfg.rm.radius, false)
    }
}

fn design_status(host: &mut dyn HostApi, cfg: &ProjectConfig, lines: &[String], chart: Option<&str>) {
    match chart {
        Some(svg) => refresh(host, cfg, DESIGN, (SM_CHART, svg), (SM_STATUS, lines.join("\n"))),
        None => crate::opsbind::push_one(host, cfg, DESIGN, Some((SM_STATUS, lines.join("\n")))),
    }
}

fn svg(title: &str, xl: &str, yl: &str, series: &[Series], zero: bool) -> String {
    render(&Chart {
        title,
        x_label: xl,
        y_label: yl,
        series,
        width: 320.0,
        height: 230.0,
        y_from_zero: zero,
    })
}

/// Runs the operation `name` (setting arguments are already in `cfg`).
pub fn run(host: &mut dyn HostApi, name: &str, args: &Args, tab: u64, cfg: &mut ProjectConfig) -> Outcome {
    let s = m();
    match name {
        "st_stereonet" => {
            let d = stereonet(host, tab, cfg, None, Vec::new())?;
            commit(host, tab, cfg, "MIN_ST_STEREONET");
            Ok(OpResult::data(d))
        }
        "st_sets" => sets_op(host, tab, cfg),
        "st_kinematic" => {
            let r = kinematic_op(host, tab, cfg)?;
            commit(host, tab, cfg, "MIN_ST_KINEMATIC");
            Ok(r)
        }
        "st_wedges" => {
            let r = wedges_op(host, args, cfg)?;
            commit(host, tab, cfg, "MIN_ST_WEDGES");
            Ok(r)
        }
        "st_draw" => {
            let (planes, _, _) = poles(host, tab, cfg)?;
            let at = point(args, "at").unwrap_or([0.0; 3]);
            let prims = rk::stereonet_prims(
                &planes,
                &set_planes(cfg),
                cfg.rm.contours,
                at,
                cfg.rm.draw_radius,
                crate::drillholes::origin(cfg),
            );
            let mut sink = HostSink::new(host);
            sink.begin("MIN_ST_DRAW");
            sink.ensure_layers(&[LayerDef::new(rk::LAYER_STEREO, 7)]);
            sink.remove_where(&|t: &Tag| t.class == Class::RmDraw && t.get("kind") == Some("stereonet"));
            let n = sink.add(prims);
            session::commit(&mut sink, crate::i18n::s(), tab, cfg);
            sink.end();
            let created = std::mem::take(&mut sink.created);
            host.push_info(&fmt(s.m_drawn, &[&n]));
            Ok(OpResult {
                created,
                ..OpResult::data(json!({"entities": n}))
            })
        }
        "sm_stress" => {
            let st = insitu(cfg);
            let sec = section(&st, cfg.rm.axis_trend);
            let line = fmt(
                s.m_stress,
                &[
                    &f(st.sigma_v(), 2),
                    &f(st.k_h * st.sigma_v(), 2),
                    &f(st.k_hmin * st.sigma_v(), 2),
                    &f(cfg.rm.axis_trend, 0),
                    &f(sec.sigma_1, 2),
                    &f(sec.sigma_3, 2),
                    &f(sec.angle, 0),
                    &f(sec.sigma_axial, 2),
                ],
            );
            host.push_info(&line);
            design_status(host, cfg, &[line], None);
            commit(host, tab, cfg, "MIN_SM_STRESS");
            Ok(OpResult::data(json!({
                "sigma_v": st.sigma_v(), "sigma_h_max": st.k_h * st.sigma_v(), "sigma_h_min": st.k_hmin * st.sigma_v(),
                "section": {"sigma_x": sec.sigma_x, "sigma_z": sec.sigma_z, "tau_xz": sec.tau_xz, "sigma_1": sec.sigma_1, "sigma_3": sec.sigma_3, "angle": sec.angle, "sigma_axial": sec.sigma_axial},
            })))
        }
        "sm_kirsch" => {
            let sec = section(&insitu(cfg), cfg.rm.axis_trend);
            let hb = hoek_brown(cfg)?;
            let (a, eq) = radius(cfg);
            let k = kirsch::analyse(sec.sigma_1, sec.sigma_3, a, &hb);
            let line = fmt(
                s.m_kirsch,
                &[
                    &f(a, 2),
                    &if eq { s.m_equiv } else { "" },
                    &f(k.sigma_t_max, 2),
                    &f(k.sigma_t_min, 2),
                    &f(hb.sigma_c(), 2),
                    &f(k.strength_factor, 2),
                    &f(k.overstressed_depth, 2),
                ],
            );
            host.push_info(&line);
            let series = vec![
                Series::new("σθ", k.boundary.clone(), Style::Line, 0),
                Series::new(
                    s.strength_s,
                    vec![(0.0, hb.sigma_c()), (360.0, hb.sigma_c())],
                    Style::Dashed,
                    1,
                ),
            ];
            let chart = svg(s.chart_kirsch, s.axis_angle, s.axis_stress, &series, false);
            state::with_tab(tab, |t| {
                t.rm.charts[0] = Some(rk::SavedChart {
                    title: s.chart_kirsch.into(),
                    x_label: s.axis_angle.into(),
                    y_label: s.axis_stress.into(),
                    series,
                })
            });
            design_status(host, cfg, &[line], Some(&chart));
            commit(host, tab, cfg, "MIN_SM_KIRSCH");
            Ok(OpResult::data(json!({
                "radius": a, "equivalent": eq, "p1": sec.sigma_1, "p3": sec.sigma_3, "sigma_t_max": k.sigma_t_max, "sigma_t_min": k.sigma_t_min,
                "sigma_cm": hb.sigma_c(), "strength_factor": finite(k.strength_factor), "overstressed_depth": k.overstressed_depth,
            })))
        }
        "sm_convergence" => {
            let x = &cfg.rm;
            let sec = section(&insitu(cfg), x.axis_trend);
            let hb = hoek_brown(cfg)?;
            let p0 = (sec.sigma_1 + sec.sigma_3) / 2.0;
            let s3 = sigma3_max(&hb, Application::Tunnel, x.gamma / 1000.0, x.depth);
            let (c, phi) = hb.mohr_coulomb(s3);
            let e = if x.e_mass > 0.0 {
                x.e_mass
            } else {
                rock_mass_modulus(cfg.gt.hb_gsi, cfg.gt.hb_d, (cfg.gt.hb_ei > 0.0).then_some(cfg.gt.hb_ei))
            };
            let (r0, _) = radius(cfg);
            let g = Ground {
                r0,
                p0,
                e,
                nu: x.nu,
                c,
                phi,
            };
            let mut parts = Vec::new();
            if x.use_shotcrete {
                parts.push(convergence::shotcrete(r0, x.sc_t, x.sc_sigma, x.sc_e, 0.25));
            }
            if x.use_bolts {
                parts.push(convergence::bolts(
                    r0,
                    &convergence::Bolt {
                        t_bf: x.b_tbf,
                        sc: x.b_sc,
                        sl: x.b_sl,
                        length: x.b_length,
                        diameter: x.b_diam,
                        e_s: 200_000.0,
                        q: x.b_q,
                    },
                ));
            }
            let sup = convergence::combine(&parts);
            let o = convergence::analyse(&g, sup, x.install);
            let class_i = [
                "few_support_problems",
                "minor_squeezing",
                "severe_squeezing",
                "very_severe_squeezing",
                "extreme_squeezing",
            ]
            .iter()
            .position(|k| *k == o.squeezing)
            .unwrap_or(0);
            let mut lines = vec![fmt(
                s.m_conv,
                &[
                    &f(p0, 2),
                    &f(c, 3),
                    &f(phi, 1),
                    &f(o.p_cr, 2),
                    &f(o.rp_max, 2),
                    &f(o.u_max * 1000.0, 1),
                    &f(o.strain, 2),
                    &s.squeezing[class_i],
                ],
            )];
            match (sup, o.p_eq, o.u_eq, o.fs) {
                (None, ..) => lines.push(s.m_no_support.into()),
                (Some(sp), Some(p), Some(u), Some(fs)) if fs > 1.0 + 1e-9 => {
                    let _ = sp;
                    lines.push(fmt(
                        s.m_support,
                        &[
                            &f(x.install, 1),
                            &f(o.u_install * 1000.0, 1),
                            &f(p, 3),
                            &f(u * 1000.0, 1),
                            &f(fs, 2),
                        ],
                    ))
                }
                (Some(sp), ..) => lines.push(fmt(s.m_support_yield, &[&f(sp.p_max, 3)])),
            }
            report(host, &lines);
            let mm = |v: &[(f64, f64)]| v.iter().map(|(u, p)| (u * 1000.0, *p)).collect::<Vec<_>>();
            let mut series = vec![Series::new(s.ground, mm(&o.ground_curve), Style::Line, 0)];
            if !o.support_curve.is_empty() {
                series.push(Series::new(s.support_s, mm(&o.support_curve), Style::Line, 1));
            }
            let chart = svg(s.chart_conv, s.axis_disp, s.axis_pressure, &series, true);
            state::with_tab(tab, |t| {
                t.rm.charts[1] = Some(rk::SavedChart {
                    title: s.chart_conv.into(),
                    x_label: s.axis_disp.into(),
                    y_label: s.axis_pressure.into(),
                    series,
                })
            });
            design_status(host, cfg, &lines, Some(&chart));
            commit(host, tab, cfg, "MIN_SM_CONVERGENCE");
            Ok(OpResult::data(json!({
                "radius": r0, "p0": p0, "c": c, "phi": phi, "e_mass": e, "p_cr": o.p_cr, "plastic_radius": o.rp_max, "u_max": o.u_max,
                "strain": o.strain, "squeezing": o.squeezing, "u_install": o.u_install, "p_eq": o.p_eq, "u_eq": o.u_eq, "support_fs": o.fs,
                "support_p_max": sup.map(|x| x.p_max),
            })))
        }
        "sm_pillar" => {
            let x = &cfg.rm;
            let shape = [Shape::Square, Shape::Rectangular, Shape::Rib][x.pl_shape.min(2)];
            let lay = Layout {
                shape,
                width: x.pl_width,
                length: x.pl_length,
                height: x.pl_height,
                room: x.pl_room,
            };
            let load = lay.load(insitu(cfg).sigma_v());
            let ucs = if x.pl_ucs > 0.0 { x.pl_ucs } else { cfg.gt.hb_sigci };
            let w = lay.effective_width();
            let formulas = [
                (Formula::LunderPakalnis, "lunder_pakalnis", "Lunder-Pakalnis"),
                (Formula::ObertDuvall, "obert_duvall", "Obert-Duvall"),
                (Formula::Bieniawski, "bieniawski", "Bieniawski"),
                (Formula::HedleyGrant, "hedley_grant", "Hedley-Grant"),
            ];
            let all: serde_json::Map<String, Value> = formulas
                .iter()
                .map(|(fo, k, _)| {
                    let st = pillar::strength(*fo, ucs, w, lay.height);
                    (k.to_string(), json!({"strength": st, "fs": st / load}))
                })
                .collect();
            let (fo, key, label) = formulas[x.pl_formula.min(3)];
            let st = pillar::strength(fo, ucs, w, lay.height);
            let line = fmt(
                s.m_pillar,
                &[
                    &s.shapes[x.pl_shape.min(2)],
                    &f(load, 2),
                    &f(lay.extraction() * 100.0, 1),
                    &f(st, 2),
                    &label,
                    &f(st / load, 2),
                ],
            );
            host.push_info(&line);
            design_status(host, cfg, &[line], None);
            commit(host, tab, cfg, "MIN_SM_PILLAR");
            Ok(OpResult::data(
                json!({"load": load, "extraction": lay.extraction(), "width_height": w / lay.height, "strength": st, "fs": st / load, "formula": key, "ucs": ucs, "all": all}),
            ))
        }
        "sm_span" => {
            let x = &cfg.rm;
            let cs = pillar::scaled_span(x.sp_span, x.sp_length, x.sp_thickness, x.sp_gravity, x.sp_dip);
            let mut line = fmt(s.m_span, &[&f(cs, 2)]);
            let (crit, ratio) = if x.sp_critical > 0.0 {
                (Some(x.sp_critical), Some(x.sp_critical / cs))
            } else {
                (None, None)
            };
            if let (Some(c), Some(r)) = (crit, ratio) {
                line.push_str(&fmt(s.m_span_crit, &[&f(c, 2), &f(r, 2)]));
            }
            host.push_info(&line);
            design_status(host, cfg, &[line], None);
            commit(host, tab, cfg, "MIN_SM_SPAN");
            Ok(OpResult::data(
                json!({"scaled_span": cs, "critical": crit, "ratio": ratio}),
            ))
        }
        "sm_draw" => {
            let k = cfg.rm.draw_chart.min(1);
            let Some(chart) = state::with_tab(tab, |t| t.rm.charts[k].clone()) else {
                return Err(no_data(s.m_no_chart));
            };
            let kind = if k == 0 { "chart_kirsch" } else { "chart_convergence" };
            let prims = rk::chart_prims(
                &chart,
                point(args, "at").unwrap_or([0.0; 3]),
                cfg.rm.draw_size,
                kind,
                crate::drillholes::origin(cfg),
            );
            let mut sink = HostSink::new(host);
            sink.begin("MIN_SM_DRAW");
            sink.ensure_layers(&[LayerDef::new(rk::LAYER_CHART, 7)]);
            sink.remove_where(&|t: &Tag| t.class == Class::RmDraw && t.get("kind") == Some(kind));
            let n = sink.add(prims);
            sink.end();
            let created = std::mem::take(&mut sink.created);
            host.push_info(&fmt(s.m_drawn, &[&n]));
            Ok(OpResult {
                created,
                ..OpResult::data(json!({"entities": n}))
            })
        }
        other => Err(OpError::new(
            ErrorCode::UnknownCommand,
            format!("unknown operation {other}"),
        )),
    }
}
