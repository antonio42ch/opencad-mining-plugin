//! `rs_*` operations: they bind the resource engines (`resources`) to the
//! host: drillhole data of the tab, solids tagged with a domain, model files
//! next to the drawing, panel feedback, and return a typed result.

use std::time::Instant;

use ocs_plugin_api::host::acadrust::EntityType;
use ocs_plugin_api::host::{Handle, HostApi};
use openmining_ops::parse::Args;
use openmining_ops::result::{ErrorCode, OpError, OpResult};
use serde_json::{json, Value};

use crate::cad::CadSink;
use crate::config::ProjectConfig;
use crate::host_sink::{record_for, tag_of_app, HostSink};
use crate::i18n::fmt;
use crate::interactive::{PickEntities, PickKind, PickSelection, PickViewPoints, SelectionKind};
use crate::opsrun::Outcome;
use crate::resources::{self as rs, DomainMeshes, ViewInput};
use crate::rsi18n::{r, RsStrings};
use crate::state::{self, Pending};
use crate::xdata::{Class, Tag, APP_DOMAIN};
use crate::{rspanels, session};

const PAGE: u64 = 5_000;

fn domain_index(cfg: &ProjectConfig) -> usize {
    cfg.rs
        .vario
        .domain
        .min(rs::effective_domains(cfg).len().saturating_sub(1))
}

fn doc_path(host: &dyn HostApi) -> Option<std::path::PathBuf> {
    host.document_path(host.tab_id())
}

fn no_data(msg: impl Into<String>) -> OpError {
    OpError::new(ErrorCode::NoData, msg)
}

fn failed(msg: impl Into<String>) -> OpError {
    OpError::new(ErrorCode::Failed, msg)
}

/// Solids assigned to a domain, grouped by domain name.
pub fn domain_meshes(host: &mut dyn HostApi) -> DomainMeshes {
    let mut found: Vec<(String, EntityType)> = Vec::new();
    let mut offset = 0u64;
    loop {
        let q = format!(r#"{{"xdata_app":"{APP_DOMAIN}","offset":{offset},"limit":{PAGE}}}"#);
        let Ok(page) = host.query_entities(&q) else { break };
        for e in &page.entities {
            let Some(name) = tag_of_app(e, APP_DOMAIN).and_then(|t| t.get("domain").map(str::to_string)) else {
                continue;
            };
            found.push((name, e.clone()));
        }
        match page.next_offset {
            Some(n) if n > offset => offset = n,
            _ => break,
        }
    }
    group_domain_entities(found)
}

/// Meshes per domain. Entities are grouped **before** building the meshes so
/// all 3DFACEs of one domain form one surface (one face alone is open and
/// contains nothing).
pub fn group_domain_entities(found: Vec<(String, EntityType)>) -> DomainMeshes {
    let mut by_domain: std::collections::BTreeMap<String, Vec<Option<EntityType>>> = Default::default();
    for (name, e) in found {
        by_domain.entry(name).or_default().push(Some(e));
    }
    by_domain
        .into_iter()
        .map(|(name, ents)| (name, crate::meshes_of(&ents)))
        .collect()
}

/// Writes `content`, creating the folder; the error text is for the user.
fn write_beside(path: &std::path::Path, content: &[u8]) -> Result<(), String> {
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(path, content).map_err(|e| e.to_string())
}

/// Refuses composites whose domain ids no longer match the domain table.
fn domains_ok(tab: u64, cfg: &ProjectConfig, r: &RsStrings) -> Result<(), OpError> {
    let names = state::with_tab(tab, |t| t.rs.domain_names.clone());
    rs::check_domains(&names, cfg, r).map_err(no_data)
}

fn assign_solids(host: &mut dyn HostApi, handles: &[Handle], domain: &str) -> usize {
    let Ok(ents) = host.entities_batch(handles) else {
        return 0;
    };
    let mut n = 0;
    for (h, e) in handles.iter().zip(ents) {
        let Some(e) = e else { continue };
        if crate::meshes_of(&[Some(e)]).is_empty() {
            continue;
        }
        let tag = Tag::new(Class::DomainSolid).with("domain", domain);
        if host.write_record(*h, record_for(APP_DOMAIN, &tag)) {
            n += 1;
        }
    }
    n
}

fn need_data(tab: u64, r: &RsStrings) -> Result<drillhole_engine::DataSet, OpError> {
    state::with_tab(tab, |t| t.data.clone()).ok_or_else(|| no_data(r.m_need_data))
}

/// Composites of the tab; after reopening a drawing they are recovered from
/// `<drawing>_composites.csv` when they were made with the current grade
/// attribute and domain table.
fn need_composites(host: &mut dyn HostApi, tab: u64, cfg: &ProjectConfig, r: &RsStrings) -> Result<(), OpError> {
    if state::with_tab(tab, |t| !t.rs.composites.is_empty()) {
        return Ok(());
    }
    let path = rs::model_file(doc_path(host).as_deref(), &cfg.rs.bm.folder, "_composites.csv");
    let Some(text) = path.as_ref().and_then(|p| std::fs::read_to_string(p).ok()) else {
        return Err(no_data(r.m_no_composites));
    };
    let doms = rs::effective_domains(cfg);
    let stamp = rs::composite_stamp(cfg);
    match rs::parse_composites_csv(&text, &doms) {
        Ok(comps) if cfg.rs.composited == stamp && !comps.is_empty() => {
            let weights = rs::decluster_weights(&comps, cfg);
            host.push_info(&fmt(r.m_reload_comp, &[&path.unwrap().display(), &comps.len()]));
            state::with_tab(tab, |t| {
                t.rs.set_composites(comps, weights, stamp.domains, stamp.variable)
            });
            Ok(())
        }
        _ => Err(no_data(r.m_comp_stale)),
    }
}

fn ready(host: &mut dyn HostApi, tab: u64, cfg: &ProjectConfig, r: &RsStrings) -> Result<(), OpError> {
    need_composites(host, tab, cfg, r)?;
    domains_ok(tab, cfg, r)
}

/// Block model of the tab or of its `.ombm` file, without complaining when
/// there is none (the geotechnical module then creates its own).
pub(crate) fn current_model(
    host: &mut dyn HostApi,
    tab: u64,
    cfg: &ProjectConfig,
) -> Option<blockmodel_engine::BlockModel> {
    if let Some(m) = state::with_tab(tab, |t| t.rs.model.clone()) {
        return Some(m);
    }
    let path = rs::model_file(doc_path(host).as_deref(), &cfg.rs.bm.folder, ".ombm")?;
    let m = blockmodel_engine::file::load(&path).ok()?;
    host.push_info(&fmt(r().m_reload_model, &[&path.display()]));
    state::with_tab(tab, |t| t.rs.model = Some(m.clone()));
    Some(m)
}

/// Block model of the tab, recovered from `<drawing>.ombm` after reopening.
fn need_model(
    host: &mut dyn HostApi,
    tab: u64,
    cfg: &ProjectConfig,
    r: &RsStrings,
) -> Result<blockmodel_engine::BlockModel, OpError> {
    current_model(host, tab, cfg).ok_or_else(|| no_data(r.m_bm_no_model))
}

fn finish(host: &mut dyn HostApi, tab: u64, cfg: &ProjectConfig, label: &str) {
    let s = crate::i18n::s();
    let mut sink = HostSink::new(host);
    sink.begin(label);
    session::commit(&mut sink, s, tab, cfg);
    sink.end();
}

fn domain_name(cfg: &ProjectConfig) -> String {
    rs::effective_domains(cfg)[domain_index(cfg)].name.clone()
}

/// Samples of the working domain of the variography panel.
fn working_samples(tab: u64, cfg: &ProjectConfig) -> Vec<geostat_engine::Sample> {
    let id = domain_index(cfg) as u32 + 1;
    state::with_tab(tab, |t| rs::samples_of(&t.rs.composites))
        .into_iter()
        .filter(|s| s.domain == id)
        .collect()
}

fn domain_samples(
    tab: u64,
    cfg: &ProjectConfig,
    r: &RsStrings,
) -> Result<(usize, String, Vec<geostat_engine::Sample>), OpError> {
    let (idx, dom) = (domain_index(cfg), domain_name(cfg));
    let samples = working_samples(tab, cfg);
    if samples.is_empty() {
        return Err(no_data(fmt(r.m_no_domain_samples, &[&dom])));
    }
    Ok((idx, dom, samples))
}

fn finite(v: f64) -> Value {
    if v.is_finite() {
        json!(v)
    } else {
        Value::Null
    }
}

fn model_json(cfg: &ProjectConfig) -> Value {
    serde_json::to_value(&cfg.rs.vario.work).unwrap_or(Value::Null)
}

fn composite(host: &mut dyn HostApi, tab: u64, cfg: &mut ProjectConfig, r: &RsStrings) -> Outcome {
    let data = need_data(tab, r)?;
    let out = rs::composite(&data, cfg, r).map_err(failed)?;
    host.push_info(&fmt(
        r.m_composited,
        &[&out.composites.len(), &rs::num(out.length), &out.holes, &out.capped],
    ));
    let lines = rs::stats_lines(r, &out.stats);
    lines.iter().for_each(|l| host.push_info(l));
    let doms = rs::effective_domains(cfg);
    let svg = rs::histogram_chart(r, &out.composites, &out.weights, &doms, &cfg.rs.unit);
    let mut warnings = Vec::new();
    let mut file = None;
    if let Some(path) = rs::model_file(doc_path(host).as_deref(), &cfg.rs.bm.folder, "_composites.csv") {
        match write_beside(&path, rs::composites_csv(&out.composites, &doms).as_bytes()) {
            Ok(()) => file = Some(path.display().to_string()),
            Err(e) => {
                let w = fmt(r.m_export_failed, &[&e]);
                host.push_error(&w);
                warnings.push(w);
            }
        }
    }
    let status = format!(
        "{}\n{}",
        fmt(r.m_status_comp, &[&out.composites.len(), &doms.len()]),
        lines.join("\n")
    );
    let stats: Vec<Value> = out
        .stats
        .iter()
        .map(|d| {
            json!({
                "name": d.name, "count": d.summary.n, "mean": finite(d.summary.mean),
                "declustered_mean": finite(d.declustered_mean), "cv": finite(d.summary.cv),
                "min": finite(d.summary.min), "max": finite(d.summary.max), "capped": d.capped,
            })
        })
        .collect();
    let data = json!({
        "variable": rs::var_name(&cfg.dh.assay_attr),
        "composites": out.composites.len(),
        "holes": out.holes,
        "capped": out.capped,
        "domains": stats,
        "file": file,
    });
    cfg.rs.composited = rs::composite_stamp(cfg);
    let names = doms.iter().map(|d| d.name.clone()).collect();
    state::with_tab(tab, |t| {
        t.rs.set_composites(out.composites, out.weights, names, rs::var_name(&cfg.dh.assay_attr))
    });
    finish(host, tab, cfg, "MIN_RS_COMPOSITE");
    session::refresh_chart(host, cfg, rspanels::COMP, &svg);
    session::set_label(host, rspanels::COMP, rspanels::COMP_STATUS, status);
    Ok(OpResult {
        warnings,
        ..OpResult::data(data)
    })
}

fn set_solids(host: &mut dyn HostApi, cfg: &ProjectConfig, handles: &[Handle], r: &RsStrings) -> Outcome {
    let doms = rs::effective_domains(cfg);
    let target = doms[cfg.rs.target.min(doms.len() - 1)].name.clone();
    host.begin_undo_group("MIN_RS_SETSOLID");
    let n = assign_solids(host, handles, &target);
    host.end_undo_group();
    if n == 0 {
        return Err(OpError::new(ErrorCode::NotFound, r.m_no_solids_sel).at("solids"));
    }
    host.push_info(&fmt(r.m_solids_assigned, &[&n, &target]));
    Ok(OpResult::data(json!({ "domain": target, "solids": n })))
}

fn estimate(host: &mut dyn HostApi, name: &str, tab: u64, cfg: &mut ProjectConfig, r: &RsStrings) -> Outcome {
    ready(host, tab, cfg, r)?;
    let (comps, weights, surface) = state::with_tab(tab, |t| {
        (t.rs.composites.clone(), t.rs.weights.clone(), t.rs.surfaces.clone())
    });
    let doms = rs::effective_domains(cfg);
    let meshes = domain_meshes(host);
    // A contact surface is needed when any domain uses one and none is computed yet.
    let mut surface = surface;
    if surface.is_empty() && doms.iter().any(|d| !d.conditions().is_empty()) {
        if let Some(data) = state::with_tab(tab, |t| t.data.clone()) {
            let list = rs::contact_surfaces(&drillhole_engine::pipeline::build_holes(&data), cfg, r).map_err(failed)?;
            surface = list.into_iter().map(|(n, s, _)| (n, s)).collect();
        }
    }
    let regions = rs::build_regions(&doms, &meshes, &surface, r).map_err(failed)?;
    let mut model = rs::create_model(&comps, cfg, &regions).map_err(failed)?;
    let g = model.geometry;
    host.push_info(&fmt(
        r.m_bm_created,
        &[&model.len(), &g.n[0], &g.n[1], &g.n[2], &model.assigned()],
    ));
    let mut data = json!({ "blocks": model.len(), "counts": g.n, "assigned": model.assigned() });
    let mut warnings = Vec::new();
    if name == "rs_estimate" {
        let t0 = Instant::now();
        let rep = rs::run_estimation(&mut model, &comps, &weights, cfg, r).map_err(no_data)?;
        let var = rs::var_name(&cfg.dh.assay_attr);
        let est: usize = rep.domains.iter().map(|d| d.estimated).sum();
        let secs = t0.elapsed().as_secs_f64();
        host.push_info(&fmt(r.m_bm_est_done, &[&est, &format!("{secs:.1}")]));
        let mut lines = rs::estimation_lines(r, &model, &rep);
        lines.extend(rs::resource_lines(r, &model, &var, cfg.rs.bm.view_cutoff, &cfg.rs.unit));
        lines.iter().for_each(|l| host.push_info(l));
        let saved = match save_model(host, cfg, &model, r) {
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
        let resources: Vec<Value> = rs::grade_column(&model, &var)
            .map(|col| {
                blockmodel_engine::report::summary_by_domain(&model, &col, cfg.rs.bm.view_cutoff)
                    .into_iter()
                    .map(|row| json!({"domain": row.domain, "blocks": row.blocks, "tonnes": finite(row.tonnes), "grade": finite(row.grade), "volume": finite(row.volume)}))
                    .collect()
            })
            .unwrap_or_default();
        data["estimated"] = json!(est);
        data["seconds"] = json!((secs * 10.0).round() / 10.0);
        data["domains"] = json!(rep
            .domains
            .iter()
            .map(|d| json!({"name": model.domain_name(d.id), "blocks": d.blocks, "estimated": d.estimated, "per_pass": d.per_pass, "samples": d.samples}))
            .collect::<Vec<_>>());
        data["cutoff"] = json!(cfg.rs.bm.view_cutoff);
        data["resources"] = json!(resources);
        data["saved"] = json!(saved);
        let status = format!("{}\n{}", fmt(r.m_status_bm, &[&model.len(), &est]), lines.join("\n"));
        session::set_label(host, rspanels::BM, rspanels::BM_STATUS, status);
    }
    state::with_tab(tab, |t| {
        t.rs.model = Some(model);
        t.rs.surfaces = surface;
    });
    finish(
        host,
        tab,
        cfg,
        if name == "rs_estimate" {
            "MIN_RS_ESTIMATE"
        } else {
            "MIN_RS_BMCREATE"
        },
    );
    Ok(OpResult {
        warnings,
        ..OpResult::data(data)
    })
}

/// Draws a block view from the pending inputs.
fn view_exec(host: &mut dyn HostApi, tab: u64, cfg: &ProjectConfig, r: &RsStrings) -> Outcome {
    let Some(Pending::BmView { axis, points }) = state::take_pending() else {
        return Err(OpError::new(ErrorCode::NeedsInput, r.pick_view_axis));
    };
    let model = need_model(host, tab, cfg, r)?;
    let axis_pts = match axis {
        Some(h) => Some(
            crate::fetch_points(host, &[h])
                .and_then(|mut v| v.pop())
                .ok_or_else(|| OpError::new(ErrorCode::NotFound, crate::i18n::s().m_not_polyline).at("axis"))?,
        ),
        None => None,
    };
    let input = ViewInput {
        axis: axis_pts.as_deref(),
        points: &points,
    };
    let out = rs::view(&model, cfg, &input, r, crate::drillholes::origin(cfg)).map_err(failed)?;
    let (planes, polygons, limited, cubes) = (out.planes, out.polygons, out.limited, out.cubes);
    let mut sink = crate::host_sink::OpSink::new(host);
    sink.begin("MIN_RS_BMVIEW");
    let n = rs::draw_view(&mut sink, r, out);
    session::commit(&mut sink, crate::i18n::s(), tab, cfg);
    sink.end();
    if n == 0 {
        return Err(no_data(r.m_view_empty));
    }
    let created = std::mem::take(&mut sink.inner.created);
    Ok(OpResult {
        created,
        ..OpResult::data(json!({ "planes": planes, "polygons": polygons, "limited": limited, "cubes": cubes }))
    })
}

fn point(args: &Args, key: &str) -> Option<mining_core::Vec3> {
    let a = args.get(key)?.as_array()?;
    Some([
        a.first()?.as_f64()?,
        a.get(1)?.as_f64()?,
        a.get(2).and_then(Value::as_f64).unwrap_or(0.0),
    ])
}

/// Follow-ups of the interactive picks: `(operation, outcome)`.
pub fn follow_up(
    host: &mut dyn HostApi,
    command: &str,
    tab: u64,
    cfg: &mut ProjectConfig,
) -> Option<(&'static str, Outcome)> {
    let r = r();
    match command {
        "MIN_RS_SETSOLID_EXEC" => {
            let Some(Pending::DomainSolids(handles)) = state::take_pending() else {
                return None;
            };
            Some(("rs_setsolid", set_solids(host, cfg, &handles, r)))
        }
        "MIN_RS_BMVIEW_EXEC" => Some(("rs_bmview", view_exec(host, tab, cfg, r))),
        _ => None,
    }
}

/// Runs the operation `name` (setting arguments are already in `cfg`).
pub fn run(host: &mut dyn HostApi, name: &str, args: &Args, tab: u64, cfg: &mut ProjectConfig) -> Outcome {
    let r = r();
    match name {
        "rs_composite" => composite(host, tab, cfg, r),
        "rs_drawcomp" => {
            need_composites(host, tab, cfg, r)?;
            let comps = state::with_tab(tab, |t| t.rs.composites.clone());
            let mut sink = HostSink::new(host);
            let n = rs::draw_composites(&mut sink, r, cfg, &comps, crate::drillholes::origin(cfg));
            Ok(OpResult::data(json!({ "composites": n })))
        }
        "rs_setsolid" => {
            if let Some(h) = crate::gtcommands::handles_arg(args, "solids") {
                return set_solids(host, cfg, &h, r);
            }
            let sel = host.selection();
            if sel.is_empty() {
                state::with_tab(tab, |t| t.config = cfg.clone());
                let prompt = crate::i18n::s().pick_solids;
                host.start_interactive(Box::new(PickSelection::new(
                    prompt.to_string(),
                    SelectionKind::DomainSolids,
                )));
                Ok(OpResult::waiting(prompt))
            } else {
                set_solids(host, cfg, &sel, r)
            }
        }
        "rs_contact" => {
            let data = need_data(tab, r)?;
            let holes = drillhole_engine::pipeline::build_holes(&data);
            let list = rs::contact_surfaces(&holes, cfg, r).map_err(failed)?;
            let info: Vec<Value> = list.iter().map(|(n, s, pts)| json!({"name": n, "points": pts, "nodes": s.z.iter().filter(|v| v.is_finite()).count()})).collect();
            let mut sink = HostSink::new(host);
            rs::draw_surfaces(&mut sink, r, &list, crate::drillholes::origin(cfg));
            let surfaces = list.into_iter().map(|(n, s, _)| (n, s)).collect();
            state::with_tab(tab, |t| {
                t.rs.surfaces = surfaces;
                t.rs.model = None;
            });
            finish(host, tab, cfg, "MIN_RS_CONTACT");
            Ok(OpResult::data(json!({ "surfaces": info })))
        }
        "rs_vario" | "rs_varioupdate" => {
            ready(host, tab, cfg, r)?;
            let (idx, dom, samples) = domain_samples(tab, cfg, r)?;
            // Cached variograms are reused only for the domain they were computed for.
            let cached = state::with_tab(tab, |t| t.rs.exp_for(idx).cloned());
            let mut warnings = Vec::new();
            let exps = match cached.filter(|_| name == "rs_varioupdate") {
                Some(e) => e,
                None => {
                    let exps = rs::compute_variograms(&samples, cfg);
                    let pairs: u64 = exps.iter().flat_map(|(_, e)| e.points.iter().map(|p| p.pairs)).sum();
                    if pairs == 0 {
                        let w = fmt(r.m_no_pairs, &[&dom]);
                        host.push_error(&w);
                        warnings.push(w);
                    } else {
                        host.push_info(&fmt(r.m_vario_done, &[&dom, &pairs, &exps.len()]));
                    }
                    state::with_tab(tab, |t| {
                        t.rs.exp = exps.clone();
                        t.rs.exp_domain = Some(idx);
                    });
                    exps
                }
            };
            let pairs: Vec<Value> = exps
                .iter()
                .map(|(d, e)| json!({"direction": d, "pairs": e.points.iter().map(|p| p.pairs).sum::<u64>(), "lags": e.points.len()}))
                .collect();
            let svg = rs::variogram_chart(r, &exps, &cfg.rs.vario.work.model(), &dom);
            finish(
                host,
                tab,
                cfg,
                if name == "rs_vario" {
                    "MIN_RS_VARIO"
                } else {
                    "MIN_RS_VARIOUPDATE"
                },
            );
            session::refresh_chart(host, cfg, rspanels::VARIO, &svg);
            Ok(OpResult {
                warnings,
                ..OpResult::data(json!({ "domain": dom, "directions": pairs, "model": model_json(cfg) }))
            })
        }
        "rs_fit" => {
            ready(host, tab, cfg, r)?;
            let (idx, dom, samples) = domain_samples(tab, cfg, r)?;
            let exps = state::with_tab(tab, |t| t.rs.exp_for(idx).cloned())
                .unwrap_or_else(|| rs::compute_variograms(&samples, cfg));
            let fit = rs::fit_working_model(&exps, &samples, cfg);
            // No experimental points: keep the user's model untouched.
            if !fit.score.is_finite() {
                return Err(no_data(fmt(r.m_no_pairs, &[&dom])));
            }
            cfg.rs.vario.work = crate::rsconfig::ModelCfg::from_model(&fit.model);
            host.push_info(&fmt(r.m_fit_done, &[&rs::num(fit.score)]));
            let svg = rs::variogram_chart(r, &exps, &fit.model, &dom);
            state::with_tab(tab, |t| {
                t.rs.exp = exps;
                t.rs.exp_domain = Some(idx);
            });
            finish(host, tab, cfg, "MIN_RS_FIT");
            session::refresh_chart(host, cfg, rspanels::VARIO, &svg);
            Ok(OpResult::data(
                json!({ "domain": dom, "model": model_json(cfg), "error": finite(fit.score) }),
            ))
        }
        "rs_modelsave" => {
            let dom = domain_name(cfg);
            cfg.rs.vario.models.insert(dom.clone(), cfg.rs.vario.work.clone());
            host.push_info(&fmt(r.m_model_saved, &[&dom]));
            finish(host, tab, cfg, "MIN_RS_MODELSAVE");
            Ok(OpResult::data(json!({ "domain": dom })))
        }
        "rs_modelload" => {
            let dom = domain_name(cfg);
            let m = cfg
                .rs
                .vario
                .models
                .get(&dom)
                .cloned()
                .ok_or_else(|| no_data(fmt(r.m_no_model, &[&dom])))?;
            cfg.rs.vario.work = m;
            host.push_info(&fmt(r.m_model_loaded, &[&dom]));
            state::with_tab(tab, |t| t.config = cfg.clone());
            session::push_rs(host, cfg);
            Ok(OpResult::data(json!({ "domain": dom, "model": model_json(cfg) })))
        }
        "rs_crossval" => {
            ready(host, tab, cfg, r)?;
            let (_, dom, samples) = domain_samples(tab, cfg, r)?;
            let (st, line) = rs::run_crossval(&samples, cfg, &dom, r).map_err(failed)?;
            host.push_info(&line);
            session::set_label(host, rspanels::VARIO, rspanels::VARIO_STATUS, line);
            Ok(OpResult::data(json!({
                "domain": dom, "n": st.n, "mean_error": finite(st.mean_error), "mse": finite(st.mse),
                "correlation": finite(st.correlation), "z_mean": finite(st.z_mean), "z_var": finite(st.z_var),
            })))
        }
        "rs_bmcreate" | "rs_estimate" => estimate(host, name, tab, cfg, r),
        "rs_validate" => {
            let model = need_model(host, tab, cfg, r)?;
            ready(host, tab, cfg, r)?;
            let (comps, weights) = state::with_tab(tab, |t| (t.rs.composites.clone(), t.rs.weights.clone()));
            let title = doc_path(host)
                .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()))
                .unwrap_or_else(|| "model".into());
            let (lines, svg) = rs::validate(r, &model, &comps, &weights, cfg, &title).map_err(failed)?;
            lines.iter().for_each(|l| host.push_info(l));
            finish(host, tab, cfg, "MIN_RS_VALIDATE");
            session::refresh_chart(host, cfg, rspanels::BM, &svg);
            Ok(OpResult::data(json!({ "lines": lines })))
        }
        "rs_bmview" => {
            need_model(host, tab, cfg, r)?;
            state::with_tab(tab, |t| t.config = cfg.clone());
            let axis = args
                .get("axis")
                .and_then(Value::as_str)
                .and_then(|h| u64::from_str_radix(h, 16).ok())
                .map(Handle::new);
            let pts: Vec<mining_core::Vec3> = ["p1", "p2"].iter().filter_map(|k| point(args, k)).collect();
            let given = match cfg.rs.bm.view_mode {
                1 => (pts.len() == 2).then_some(Pending::BmView {
                    axis: None,
                    points: pts,
                }),
                2 => axis.map(|a| Pending::BmView {
                    axis: Some(a),
                    points: Vec::new(),
                }),
                _ => Some(Pending::BmView {
                    axis: None,
                    points: Vec::new(),
                }),
            };
            if let Some(p) = given {
                state::set_pending(p);
                return view_exec(host, tab, cfg, r);
            }
            if cfg.rs.bm.view_mode == 1 {
                host.start_interactive(Box::new(PickViewPoints::new(r.pick_view_points.map(str::to_string))));
                Ok(OpResult::waiting(r.pick_view_points[0]))
            } else {
                host.start_interactive(Box::new(PickEntities::new(
                    PickKind::ViewAxis,
                    vec![r.pick_view_axis.to_string()],
                )));
                Ok(OpResult::waiting(r.pick_view_axis))
            }
        }
        "rs_bmload" => {
            let path = rs::model_file(doc_path(host).as_deref(), &cfg.rs.bm.folder, ".ombm")
                .ok_or_else(|| no_data(r.m_bm_need_saved))?;
            let m = blockmodel_engine::file::load(&path)
                .map_err(|e| OpError::new(ErrorCode::NotFound, fmt(r.m_bm_load_failed, &[&e])))?;
            host.push_info(&fmt(r.m_bm_loaded, &[&m.len()]));
            let blocks = m.len();
            state::with_tab(tab, |t| t.rs.model = Some(m));
            Ok(OpResult::data(
                json!({ "blocks": blocks, "file": path.display().to_string() }),
            ))
        }
        "rs_bmexport" => {
            let model = need_model(host, tab, cfg, r)?;
            let path = rs::model_file(doc_path(host).as_deref(), &cfg.rs.bm.folder, "_bloques.csv")
                .ok_or_else(|| no_data(r.m_bm_need_saved))?;
            if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
                let _ = std::fs::create_dir_all(dir);
            }
            std::fs::File::create(&path)
                .and_then(|f| blockmodel_engine::file::write_csv(&model, &mut std::io::BufWriter::new(f)))
                .map_err(|e| failed(fmt(r.m_export_failed, &[&e])))?;
            host.push_info(&fmt(r.m_exported, &[&path.display()]));
            Ok(OpResult::data(json!({ "file": path.display().to_string() })))
        }
        other => Err(OpError::new(
            ErrorCode::UnknownCommand,
            format!("unknown resource operation {other}"),
        )),
    }
}

pub(crate) fn save_model(
    host: &dyn HostApi,
    cfg: &ProjectConfig,
    model: &blockmodel_engine::BlockModel,
    r: &RsStrings,
) -> Result<String, String> {
    let path = rs::model_file(doc_path(host).as_deref(), &cfg.rs.bm.folder, ".ombm")
        .ok_or_else(|| r.m_bm_need_saved.to_string())?;
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    blockmodel_engine::file::save(model, &path).map_err(|e| fmt(r.m_bm_save_failed, &[&e]))?;
    Ok(path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ocs_plugin_api::host::acadrust::entities::Face3D;
    use ocs_plugin_api::host::acadrust::types::Vector3;
    use solids_engine::inside::MeshIndex;

    /// A box drawn as six 3DFACEs and tagged with one domain must be a closed
    /// solid (it used to become six open one-face meshes).
    #[test]
    fn domain_3dfaces_are_merged_into_one_closed_mesh() {
        let v = |x: f64, y: f64, z: f64| Vector3::new(x, y, z);
        let c = [
            v(0., 0., 0.),
            v(4., 0., 0.),
            v(4., 4., 0.),
            v(0., 4., 0.),
            v(0., 0., 4.),
            v(4., 0., 4.),
            v(4., 4., 4.),
            v(0., 4., 4.),
        ];
        let quads = [
            [0, 3, 2, 1],
            [4, 5, 6, 7],
            [0, 1, 5, 4],
            [1, 2, 6, 5],
            [2, 3, 7, 6],
            [3, 0, 4, 7],
        ];
        let mut found: Vec<(String, EntityType)> = quads
            .iter()
            .map(|q| {
                (
                    "Veta".to_string(),
                    EntityType::Face3D(Face3D::new(c[q[0]], c[q[1]], c[q[2]], c[q[3]])),
                )
            })
            .collect();
        found.push((
            "Otro".to_string(),
            EntityType::Face3D(Face3D::new(c[0], c[1], c[2], c[3])),
        ));
        let m = group_domain_entities(found);
        assert_eq!(m["Veta"].len(), 1, "all faces of a domain form one mesh");
        let idx = MeshIndex::new(&m["Veta"][0]);
        assert!(idx.contains([2.0, 2.0, 2.0]));
        assert!(!idx.contains([5.0, 2.0, 2.0]));
        assert_eq!(m["Otro"].len(), 1);
    }

    #[test]
    fn files_are_written_into_new_folders() {
        let dir = std::env::temp_dir()
            .join(format!("om_wb_{}", std::process::id()))
            .join("nueva/carpeta");
        let p = dir.join("x.csv");
        write_beside(&p, b"a,b\n").unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), b"a,b\n");
        std::fs::remove_dir_all(dir.parent().unwrap().parent().unwrap()).ok();
    }
}
