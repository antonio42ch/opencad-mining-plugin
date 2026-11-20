//! `MIN_RS_*` commands: they bind the resource engines (`resources`) to the
//! host: reading the drillhole data of the tab, solids tagged with a domain,
//! the model files next to the drawing and the panel feedback.

use std::time::Instant;

use ocs_plugin_api::dialog::DialogValue;
use ocs_plugin_api::host::acadrust::EntityType;
use ocs_plugin_api::host::{Handle, HostApi};

use crate::cad::CadSink;
use crate::config::ProjectConfig;
use crate::host_sink::{record_for, tag_of_app, HostSink};
use crate::i18n::fmt;
use crate::interactive::{PickEntities, PickKind, PickSelection, PickViewPoints, SelectionKind};
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
fn domains_ok(host: &mut dyn HostApi, tab: u64, cfg: &ProjectConfig, r: &RsStrings) -> bool {
    let names = state::with_tab(tab, |t| t.rs.domain_names.clone());
    match rs::check_domains(&names, cfg, r) {
        Ok(()) => true,
        Err(e) => {
            host.push_error(&e);
            false
        }
    }
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

fn need_data(host: &mut dyn HostApi, tab: u64, r: &RsStrings) -> Option<drillhole_engine::DataSet> {
    let d = state::with_tab(tab, |t| t.data.clone());
    if d.is_none() {
        host.push_error(r.m_need_data);
    }
    d
}

/// Composites of the tab; after reopening a drawing they are recovered from
/// `<drawing>_composites.csv` when they were made with the current grade
/// attribute and domain table.
fn need_composites(host: &mut dyn HostApi, tab: u64, cfg: &ProjectConfig, r: &RsStrings) -> bool {
    if state::with_tab(tab, |t| !t.rs.composites.is_empty()) {
        return true;
    }
    let path = rs::model_file(doc_path(host).as_deref(), &cfg.rs.bm.folder, "_composites.csv");
    let Some(text) = path.as_ref().and_then(|p| std::fs::read_to_string(p).ok()) else {
        host.push_error(r.m_no_composites);
        return false;
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
            true
        }
        _ => {
            host.push_error(r.m_comp_stale);
            false
        }
    }
}

/// Block model of the tab, recovered from `<drawing>.ombm` after reopening.
fn need_model(
    host: &mut dyn HostApi,
    tab: u64,
    cfg: &ProjectConfig,
    r: &RsStrings,
) -> Option<blockmodel_engine::BlockModel> {
    if let Some(m) = state::with_tab(tab, |t| t.rs.model.clone()) {
        return Some(m);
    }
    let Some(path) = rs::model_file(doc_path(host).as_deref(), &cfg.rs.bm.folder, ".ombm") else {
        host.push_error(r.m_bm_no_model);
        return None;
    };
    match blockmodel_engine::file::load(&path) {
        Ok(m) => {
            host.push_info(&fmt(r.m_reload_model, &[&path.display()]));
            state::with_tab(tab, |t| t.rs.model = Some(m.clone()));
            Some(m)
        }
        Err(_) => {
            host.push_error(r.m_bm_no_model);
            None
        }
    }
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

pub fn dispatch(host: &mut dyn HostApi, name: &str, tab: u64, cfg: &mut ProjectConfig) -> bool {
    let r = r();
    match name {
        "MIN_RS_COMPOSITE" => {
            let Some(data) = need_data(host, tab, r) else {
                return true;
            };
            match rs::composite(&data, cfg, r) {
                Err(e) => host.push_error(&e),
                Ok(out) => {
                    host.push_info(&fmt(
                        r.m_composited,
                        &[&out.composites.len(), &rs::num(out.length), &out.holes, &out.capped],
                    ));
                    let lines = rs::stats_lines(r, &out.stats);
                    lines.iter().for_each(|l| host.push_info(l));
                    let doms = rs::effective_domains(cfg);
                    let svg = rs::histogram_chart(r, &out.composites, &out.weights, &doms, &cfg.rs.unit);
                    if let Some(path) = rs::model_file(doc_path(host).as_deref(), &cfg.rs.bm.folder, "_composites.csv")
                    {
                        if let Err(e) = write_beside(&path, rs::composites_csv(&out.composites, &doms).as_bytes()) {
                            host.push_error(&fmt(r.m_export_failed, &[&e]));
                        }
                    }
                    let status = format!(
                        "{}\n{}",
                        fmt(r.m_status_comp, &[&out.composites.len(), &doms.len()]),
                        lines.join("\n")
                    );
                    cfg.rs.composited = rs::composite_stamp(cfg);
                    let names = doms.iter().map(|d| d.name.clone()).collect();
                    state::with_tab(tab, |t| {
                        t.rs.set_composites(out.composites, out.weights, names, rs::var_name(&cfg.dh.assay_attr))
                    });
                    finish(host, tab, cfg, "MIN_RS_COMPOSITE");
                    session::refresh_chart(host, cfg, rspanels::COMP, &svg);
                    session::set_label(host, rspanels::COMP, rspanels::COMP_STATUS, status);
                }
            }
        }
        "MIN_RS_DRAWCOMP" => {
            if !need_composites(host, tab, cfg, r) {
                return true;
            }
            let comps = state::with_tab(tab, |t| t.rs.composites.clone());
            let mut sink = HostSink::new(host);
            rs::draw_composites(&mut sink, r, cfg, &comps, crate::drillholes::origin(cfg));
        }
        "MIN_RS_SETSOLID" => {
            let sel = host.selection();
            if sel.is_empty() {
                host.start_interactive(Box::new(PickSelection::new(
                    crate::i18n::s().pick_solids.to_string(),
                    SelectionKind::DomainSolids,
                )));
            } else {
                state::set_pending(Pending::DomainSolids(sel));
                return dispatch(host, "MIN_RS_SETSOLID_EXEC", tab, cfg);
            }
        }
        "MIN_RS_SETSOLID_EXEC" => {
            let Some(Pending::DomainSolids(handles)) = state::take_pending() else {
                return true;
            };
            let target = match session::panel_value(host, rspanels::COMP, "rs_target") {
                Some(DialogValue::Choice { text, .. }) if !text.is_empty() => text,
                _ => rs::effective_domains(cfg)[0].name.clone(),
            };
            host.begin_undo_group("MIN_RS_SETSOLID");
            let n = assign_solids(host, &handles, &target);
            host.end_undo_group();
            if n == 0 {
                host.push_error(r.m_no_solids_sel);
            } else {
                host.push_info(&fmt(r.m_solids_assigned, &[&n, &target]));
            }
        }
        "MIN_RS_CONTACT" => {
            let Some(data) = need_data(host, tab, r) else {
                return true;
            };
            let holes = drillhole_engine::pipeline::build_holes(&data);
            match rs::contact_surfaces(&holes, cfg, r) {
                Err(e) => host.push_error(&e),
                Ok(list) => {
                    let mut sink = HostSink::new(host);
                    rs::draw_surfaces(&mut sink, r, &list, crate::drillholes::origin(cfg));
                    let surfaces = list.into_iter().map(|(n, s, _)| (n, s)).collect();
                    state::with_tab(tab, |t| {
                        t.rs.surfaces = surfaces;
                        t.rs.model = None;
                    });
                    finish(host, tab, cfg, "MIN_RS_CONTACT");
                }
            }
        }
        "MIN_RS_VARIO" | "MIN_RS_VARIOUPDATE" => {
            if !need_composites(host, tab, cfg, r) || !domains_ok(host, tab, cfg, r) {
                return true;
            }
            let (idx, dom) = (domain_index(cfg), domain_name(cfg));
            let samples = working_samples(tab, cfg);
            if samples.is_empty() {
                host.push_error(&fmt(r.m_no_domain_samples, &[&dom]));
                return true;
            }
            // Cached variograms are reused only for the domain they were computed for.
            let cached = state::with_tab(tab, |t| t.rs.exp_for(idx).cloned());
            let exps = match cached.filter(|_| name == "MIN_RS_VARIOUPDATE") {
                Some(e) => e,
                None => {
                    let exps = rs::compute_variograms(&samples, cfg);
                    let pairs: u64 = exps.iter().flat_map(|(_, e)| e.points.iter().map(|p| p.pairs)).sum();
                    if pairs == 0 {
                        host.push_error(&fmt(r.m_no_pairs, &[&dom]));
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
            let svg = rs::variogram_chart(r, &exps, &cfg.rs.vario.work.model(), &dom);
            finish(host, tab, cfg, name);
            session::refresh_chart(host, cfg, rspanels::VARIO, &svg);
        }
        "MIN_RS_FIT" => {
            if !need_composites(host, tab, cfg, r) || !domains_ok(host, tab, cfg, r) {
                return true;
            }
            let (idx, dom) = (domain_index(cfg), domain_name(cfg));
            let samples = working_samples(tab, cfg);
            if samples.is_empty() {
                host.push_error(&fmt(r.m_no_domain_samples, &[&dom]));
                return true;
            }
            let exps = state::with_tab(tab, |t| t.rs.exp_for(idx).cloned())
                .unwrap_or_else(|| rs::compute_variograms(&samples, cfg));
            let fit = rs::fit_working_model(&exps, &samples, cfg);
            // No experimental points: keep the user's model untouched.
            if !fit.score.is_finite() {
                host.push_error(&fmt(r.m_no_pairs, &[&dom]));
                return true;
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
        }
        "MIN_RS_MODELSAVE" => {
            let dom = domain_name(cfg);
            cfg.rs.vario.models.insert(dom.clone(), cfg.rs.vario.work.clone());
            host.push_info(&fmt(r.m_model_saved, &[&dom]));
            finish(host, tab, cfg, "MIN_RS_MODELSAVE");
        }
        "MIN_RS_MODELLOAD" => {
            let dom = domain_name(cfg);
            match cfg.rs.vario.models.get(&dom).cloned() {
                None => host.push_error(&fmt(r.m_no_model, &[&dom])),
                Some(m) => {
                    cfg.rs.vario.work = m;
                    host.push_info(&fmt(r.m_model_loaded, &[&dom]));
                    state::with_tab(tab, |t| t.config = cfg.clone());
                    session::push_rs(host, cfg);
                }
            }
        }
        "MIN_RS_CROSSVAL" => {
            if !need_composites(host, tab, cfg, r) || !domains_ok(host, tab, cfg, r) {
                return true;
            }
            let dom = domain_name(cfg);
            let samples = working_samples(tab, cfg);
            if samples.is_empty() {
                host.push_error(&fmt(r.m_no_domain_samples, &[&dom]));
                return true;
            }
            match rs::run_crossval(&samples, cfg, &dom, r) {
                Ok((_, line)) => {
                    host.push_info(&line);
                    session::set_label(host, rspanels::VARIO, rspanels::VARIO_STATUS, line);
                }
                Err(e) => host.push_error(&e),
            }
        }
        "MIN_RS_BMCREATE" | "MIN_RS_ESTIMATE" => {
            if !need_composites(host, tab, cfg, r) || !domains_ok(host, tab, cfg, r) {
                return true;
            }
            let (comps, weights, surface) = state::with_tab(tab, |t| {
                (t.rs.composites.clone(), t.rs.weights.clone(), t.rs.surfaces.clone())
            });
            let doms = rs::effective_domains(cfg);
            let meshes = domain_meshes(host);
            // A contact surface is needed when any domain uses one and none is computed yet.
            let mut surface = surface;
            if surface.is_empty() && doms.iter().any(|d| !d.conditions().is_empty()) {
                if let Some(data) = state::with_tab(tab, |t| t.data.clone()) {
                    match rs::contact_surfaces(&drillhole_engine::pipeline::build_holes(&data), cfg, r) {
                        Ok(list) => surface = list.into_iter().map(|(n, s, _)| (n, s)).collect(),
                        Err(e) => {
                            host.push_error(&e);
                            return true;
                        }
                    }
                }
            }
            let regions = match rs::build_regions(&doms, &meshes, &surface, r) {
                Ok(v) => v,
                Err(e) => {
                    host.push_error(&e);
                    return true;
                }
            };
            let mut model = match rs::create_model(&comps, cfg, &regions) {
                Ok(m) => m,
                Err(e) => {
                    host.push_error(&e);
                    return true;
                }
            };
            let g = model.geometry;
            host.push_info(&fmt(
                r.m_bm_created,
                &[&model.len(), &g.n[0], &g.n[1], &g.n[2], &model.assigned()],
            ));
            if name == "MIN_RS_ESTIMATE" {
                let t0 = Instant::now();
                match rs::run_estimation(&mut model, &comps, &weights, cfg, r) {
                    Err(e) => {
                        host.push_error(&e);
                        return true;
                    }
                    Ok(rep) => {
                        let var = rs::var_name(&cfg.dh.assay_attr);
                        let est: usize = rep.domains.iter().map(|d| d.estimated).sum();
                        host.push_info(&fmt(
                            r.m_bm_est_done,
                            &[&est, &format!("{:.1}", t0.elapsed().as_secs_f64())],
                        ));
                        let mut lines = rs::estimation_lines(r, &model, &rep);
                        lines.extend(rs::resource_lines(r, &model, &var, cfg.rs.bm.view_cutoff, &cfg.rs.unit));
                        lines.iter().for_each(|l| host.push_info(l));
                        match save_model(host, cfg, &model, r) {
                            Ok(path) => host.push_info(&fmt(r.m_bm_saved, &[&path])),
                            Err(e) => host.push_info(&e),
                        }
                        let status = format!("{}\n{}", fmt(r.m_status_bm, &[&model.len(), &est]), lines.join("\n"));
                        session::set_label(host, rspanels::BM, rspanels::BM_STATUS, status);
                    }
                }
            }
            state::with_tab(tab, |t| {
                t.rs.model = Some(model);
                t.rs.surfaces = surface;
            });
            finish(host, tab, cfg, name);
        }
        "MIN_RS_VALIDATE" => {
            let Some(model) = need_model(host, tab, cfg, r) else {
                return true;
            };
            if !need_composites(host, tab, cfg, r) || !domains_ok(host, tab, cfg, r) {
                return true;
            }
            let (comps, weights) = state::with_tab(tab, |t| (t.rs.composites.clone(), t.rs.weights.clone()));
            let title = doc_path(host)
                .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()))
                .unwrap_or_else(|| "model".into());
            match rs::validate(r, &model, &comps, &weights, cfg, &title) {
                Err(e) => host.push_error(&e),
                Ok((lines, svg)) => {
                    lines.iter().for_each(|l| host.push_info(l));
                    finish(host, tab, cfg, "MIN_RS_VALIDATE");
                    session::refresh_chart(host, cfg, rspanels::BM, &svg);
                }
            }
        }
        "MIN_RS_BMVIEW" => {
            if need_model(host, tab, cfg, r).is_none() {
                return true;
            }
            state::with_tab(tab, |t| t.config = cfg.clone());
            match cfg.rs.bm.view_mode {
                1 => host.start_interactive(Box::new(PickViewPoints::new(r.pick_view_points.map(str::to_string)))),
                2 => host.start_interactive(Box::new(PickEntities::new(
                    PickKind::ViewAxis,
                    vec![r.pick_view_axis.to_string()],
                ))),
                _ => {
                    state::set_pending(Pending::BmView {
                        axis: None,
                        points: Vec::new(),
                    });
                    return dispatch(host, "MIN_RS_BMVIEW_EXEC", tab, cfg);
                }
            }
        }
        "MIN_RS_BMVIEW_EXEC" => {
            let Some(Pending::BmView { axis, points }) = state::take_pending() else {
                return true;
            };
            let Some(model) = need_model(host, tab, cfg, r) else {
                return true;
            };
            let axis_pts = match axis {
                Some(h) => match crate::fetch_points(host, &[h]) {
                    Some(mut v) => Some(v.remove(0)),
                    None => {
                        host.push_error(crate::i18n::s().m_not_polyline);
                        return true;
                    }
                },
                None => None,
            };
            let input = ViewInput {
                axis: axis_pts.as_deref(),
                points: &points,
            };
            match rs::view(&model, cfg, &input, r, crate::drillholes::origin(cfg)) {
                Err(e) => host.push_error(&e),
                Ok(out) => {
                    let mut sink = HostSink::new(host);
                    sink.begin("MIN_RS_BMVIEW");
                    rs::draw_view(&mut sink, r, out);
                    session::commit(&mut sink, crate::i18n::s(), tab, cfg);
                    sink.end();
                }
            }
        }
        "MIN_RS_BMLOAD" => match rs::model_file(doc_path(host).as_deref(), &cfg.rs.bm.folder, ".ombm") {
            None => host.push_error(r.m_bm_need_saved),
            Some(path) => match blockmodel_engine::file::load(&path) {
                Err(e) => host.push_error(&fmt(r.m_bm_load_failed, &[&e])),
                Ok(m) => {
                    host.push_info(&fmt(r.m_bm_loaded, &[&m.len()]));
                    state::with_tab(tab, |t| t.rs.model = Some(m));
                }
            },
        },
        "MIN_RS_BMEXPORT" => {
            let Some(model) = need_model(host, tab, cfg, r) else {
                return true;
            };
            match rs::model_file(doc_path(host).as_deref(), &cfg.rs.bm.folder, "_bloques.csv") {
                None => host.push_error(r.m_bm_need_saved),
                Some(path) => {
                    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
                        let _ = std::fs::create_dir_all(dir);
                    }
                    let res = std::fs::File::create(&path)
                        .and_then(|f| blockmodel_engine::file::write_csv(&model, &mut std::io::BufWriter::new(f)));
                    match res {
                        Ok(()) => host.push_info(&fmt(r.m_exported, &[&path.display()])),
                        Err(e) => host.push_error(&fmt(r.m_export_failed, &[&e])),
                    }
                }
            }
        }
        _ => return false,
    }
    true
}

fn save_model(
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
