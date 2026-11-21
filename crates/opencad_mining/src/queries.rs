//! Read-only operations (`status`, `dh_list`, `rs_stats`, `rs_variogram`,
//! `bm_info`, `bm_sample`, `bm_resources`, `gt_summary`, `entities`): what
//! an AI client reads to decide the next step. They never change the drawing.

use std::collections::BTreeMap;

use drillhole_engine::model::hole_key;
use ocs_plugin_api::host::HostApi;
use openmining_ops::parse::Args;
use openmining_ops::result::{ErrorCode, OpError, OpResult};
use serde_json::{json, Value};

use crate::config::ProjectConfig;
use crate::host_sink::tag_of;
use crate::opsrun::Outcome;
use crate::resources as rs;
use crate::state;
use crate::xdata::APP;

fn no_data(msg: &str) -> OpError {
    OpError::new(ErrorCode::NoData, msg)
}

fn finite(v: f64) -> Value {
    if v.is_finite() {
        json!(v)
    } else {
        Value::Null
    }
}

fn int(args: &Args, key: &str, default: i64) -> i64 {
    args.get(key).and_then(Value::as_i64).unwrap_or(default)
}

fn point(args: &Args, key: &str) -> Option<mining_core::Vec3> {
    let a = args.get(key)?.as_array()?;
    Some([
        a.first()?.as_f64()?,
        a.get(1)?.as_f64()?,
        a.get(2).and_then(Value::as_f64).unwrap_or(0.0),
    ])
}

fn status(host: &mut dyn HostApi, tab: u64, cfg: &ProjectConfig) -> Value {
    let doc = host.document_path(tab);
    let (data, comps, rs_names, variable, model, surfaces, gt, axis) = state::with_tab(tab, |t| {
        (
            t.data.clone(),
            t.rs.composites.len(),
            t.rs.domain_names.clone(),
            t.rs.variable.clone(),
            t.rs.model
                .as_ref()
                .map(|m| (m.len(), m.columns.keys().cloned().collect::<Vec<_>>())),
            t.rs.surfaces.iter().map(|(n, _)| n.clone()).collect::<Vec<_>>(),
            (
                t.gt.logging.as_ref().map(|x| x.records.len()),
                t.gt.mapping.as_ref().map(|x| x.records.len()),
                t.gt.lab_rows,
                t.gt.joints.as_ref().map(|x| x.records.len()),
                t.gt.results.len(),
            ),
            t.axis
                .as_ref()
                .map(|a| json!({"axis": a.id, "elements": a.design.elements.len(), "length": a.design.length()})),
        )
    });
    let doms = rs::effective_domains(cfg);
    let stale = comps > 0 && rs::check_domains(&rs_names, cfg, crate::rsi18n::r()).is_err();
    let mut next: Vec<Value> = Vec::new();
    let mut suggest = |op: &str, why: String| next.push(json!({"op": op, "why": why}));
    let model_file = rs::model_file(doc.as_deref(), &cfg.rs.bm.folder, ".ombm").filter(|p| p.exists());
    match &data {
        None => suggest("dh_load", "no drillholes are loaded".into()),
        Some(_) if comps == 0 => suggest(
            "rs_composite",
            "drillholes are loaded but there are no composites".into(),
        ),
        Some(_) if stale => suggest(
            "rs_composite",
            "the domain table or the grade attribute changed since compositing".into(),
        ),
        Some(_) => {
            let missing: Vec<&str> = doms
                .iter()
                .filter(|d| !cfg.rs.vario.models.contains_key(&d.name))
                .map(|d| d.name.as_str())
                .collect();
            if !missing.is_empty() {
                suggest(
                    "rs_vario",
                    format!(
                        "no saved variogram model for domain(s) {}: rs_vario, rs_fit, rs_modelsave",
                        missing.join(", ")
                    ),
                );
            } else if model.is_none() && model_file.is_none() {
                suggest(
                    "rs_estimate",
                    "every domain has a model but there is no block model".into(),
                );
            }
        }
    }
    let (log, map, lab, joints, classified) = gt;
    if log.is_some() || map.is_some() {
        if classified == 0 {
            suggest(
                "gt_classify",
                "geotechnical tables are loaded but not classified".into(),
            );
        } else if !model.as_ref().is_some_and(|(_, c)| c.iter().any(|k| k == "gt_rmr")) {
            suggest(
                "gt_estimate",
                "records are classified but the block model has no geotechnical columns".into(),
            );
        }
    }
    json!({
        "drawing": doc.map(|p| p.display().to_string()),
        "drillholes": match &data {
            None => json!({"loaded": false}),
            Some(d) => json!({
                "loaded": true, "holes": d.collars.len(), "assays": d.assays.len(), "geo": d.geo.len(),
                "assay_attributes": drillhole_engine::import::assay_attributes(&d.assays),
                "geo_attributes": drillhole_engine::import::geo_attributes(&d.geo),
                "assay_attr": cfg.dh.assay_attr, "geo_attr": cfg.dh.geo_attr,
            }),
        },
        "composites": {"count": comps, "variable": variable, "stale": stale},
        "domains": doms.iter().map(|d| json!({"name": d.name, "values": d.values, "density": d.density, "surface": d.surface, "model_saved": cfg.rs.vario.models.contains_key(&d.name)})).collect::<Vec<_>>(),
        "surfaces": surfaces,
        "block_model": match &model {
            Some((n, cols)) => json!({"blocks": n, "columns": cols}),
            None => model_file.map_or(Value::Null, |p| json!({"file": p.display().to_string(), "loaded": false})),
        },
        "geotechnics": {"tables": {"logging": log, "mapping": map, "lab_lithologies": lab, "joints": joints}, "classified": classified},
        "axis": axis,
        "next": next,
    })
}

fn help(args: &Args) -> Outcome {
    let text = |k: &str| args.get(k).and_then(Value::as_str);
    let mut v = openmining_ops::help::help(text("op"), text("group")).ok_or_else(|| {
        let what = text("op")
            .map(|o| format!("operation '{o}'"))
            .unwrap_or_else(|| format!("group '{}'", text("group").unwrap_or("")));
        OpError::new(
            ErrorCode::NotFound,
            format!("no {what}: call MIN_HELP without arguments for the list"),
        )
        .at(if text("op").is_some() { "op" } else { "group" })
    })?;
    if args.get("workflows").and_then(Value::as_bool).unwrap_or(false) {
        v["workflows"] = json!(openmining_ops::doc::WORKFLOWS);
        v["error_codes"] = json!(openmining_ops::result::ErrorCode::ALL
            .iter()
            .map(|c| (c.as_str().to_string(), json!(c.describe())))
            .collect::<serde_json::Map<_, _>>());
    }
    Ok(OpResult::data(v))
}

fn dh_list(tab: u64, args: &Args) -> Outcome {
    let data = state::with_tab(tab, |t| t.data.clone()).ok_or_else(|| no_data(crate::i18n::s().m_no_data))?;
    let limit = int(args, "limit", 500) as usize;
    if let Some(id) = args.get("hole").and_then(Value::as_str) {
        let k = hole_key(id);
        let c = data
            .collars
            .iter()
            .find(|c| hole_key(&c.hole_id) == k)
            .ok_or_else(|| OpError::new(ErrorCode::NotFound, format!("hole '{id}' is not loaded")).at("hole"))?;
        let assays: Vec<Value> = data
            .assays
            .iter()
            .filter(|a| hole_key(&a.hole_id) == k)
            .take(limit)
            .map(|a| json!({"from": a.from, "to": a.to, "values": a.attributes}))
            .collect();
        let geo: Vec<Value> = data
            .geo
            .iter()
            .filter(|g| hole_key(&g.hole_id) == k)
            .take(limit)
            .map(|g| json!({"from": g.from, "to": g.to, "values": g.attributes}))
            .collect();
        return Ok(OpResult::data(
            json!({"id": c.hole_id, "x": c.x, "y": c.y, "z": c.z, "depth": c.max_depth, "assays": assays, "geo": geo}),
        ));
    }
    let mut na: BTreeMap<String, usize> = BTreeMap::new();
    let mut ng: BTreeMap<String, usize> = BTreeMap::new();
    for a in &data.assays {
        *na.entry(hole_key(&a.hole_id)).or_default() += 1;
    }
    for g in &data.geo {
        *ng.entry(hole_key(&g.hole_id)).or_default() += 1;
    }
    let holes: Vec<Value> = data
        .collars
        .iter()
        .take(limit)
        .map(|c| {
            let k = hole_key(&c.hole_id);
            json!({"id": c.hole_id, "x": c.x, "y": c.y, "z": c.z, "depth": c.max_depth, "azimuth": c.azimuth, "dip": c.dip,
                   "assays": na.get(&k).copied().unwrap_or(0), "geo": ng.get(&k).copied().unwrap_or(0)})
        })
        .collect();
    Ok(OpResult::data(json!({"total": data.collars.len(), "holes": holes})))
}

fn rs_stats(tab: u64) -> Outcome {
    let (comps, weights, names, variable) = state::with_tab(tab, |t| {
        (
            t.rs.composites.clone(),
            t.rs.weights.clone(),
            t.rs.domain_names.clone(),
            t.rs.variable.clone(),
        )
    });
    if comps.is_empty() {
        return Err(no_data(crate::rsi18n::r().m_no_composites));
    }
    let doms: Vec<Value> = names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let idx: Vec<usize> = (0..comps.len()).filter(|&k| comps[k].domain == i as u32 + 1).collect();
            let v: Vec<f64> = idx.iter().map(|&k| comps[k].value).collect();
            let w: Vec<f64> = idx.iter().map(|&k| weights.get(k).copied().unwrap_or(1.0)).collect();
            let s = geostat_engine::stats::summarize(&v, None);
            let dw = geostat_engine::stats::summarize(&v, Some(&w));
            json!({
                "name": name, "count": s.n, "length": idx.iter().map(|&k| comps[k].length()).sum::<f64>(),
                "mean": finite(s.mean), "declustered_mean": finite(dw.mean), "cv": finite(s.cv),
                "min": finite(s.min), "p25": finite(s.p25), "median": finite(s.median), "p75": finite(s.p75), "max": finite(s.max),
            })
        })
        .collect();
    Ok(OpResult::data(json!({"variable": variable, "domains": doms})))
}

fn rs_variogram(tab: u64, cfg: &ProjectConfig, args: &Args) -> Outcome {
    let doms = rs::effective_domains(cfg);
    let idx = args
        .get("domain")
        .and_then(Value::as_u64)
        .map_or(cfg.rs.vario.domain, |i| i as usize)
        .min(doms.len() - 1);
    let name = doms[idx].name.clone();
    let exps = state::with_tab(tab, |t| t.rs.exp_for(idx).cloned());
    let dirs: Vec<Value> = exps
        .unwrap_or_default()
        .iter()
        .map(|(d, e)| json!({"direction": d, "points": e.points.iter().map(|p| json!({"lag": p.lag, "gamma": finite(p.gamma), "pairs": p.pairs})).collect::<Vec<_>>()}))
        .collect();
    Ok(OpResult::data(json!({
        "domain": name,
        "directions": dirs,
        "working_model": serde_json::to_value(&cfg.rs.vario.work).unwrap_or(Value::Null),
        "saved_model": cfg.rs.vario.models.get(&name).map(|m| serde_json::to_value(m).unwrap_or(Value::Null)),
    })))
}

fn model(host: &mut dyn HostApi, tab: u64, cfg: &ProjectConfig) -> Result<blockmodel_engine::BlockModel, OpError> {
    crate::rscommands::current_model(host, tab, cfg).ok_or_else(|| no_data(crate::rsi18n::r().m_bm_no_model))
}

fn bm_info(m: &blockmodel_engine::BlockModel) -> Value {
    let g = m.geometry;
    let mut doms: BTreeMap<String, usize> = BTreeMap::new();
    for &d in &m.domain {
        if d != 0 {
            *doms.entry(m.domain_name(d).to_string()).or_default() += 1;
        }
    }
    let cols: serde_json::Map<String, Value> = m
        .columns
        .iter()
        .map(|(k, v)| {
            let f: Vec<f64> = v.iter().filter(|x| x.is_finite()).map(|x| *x as f64).collect();
            let (lo, hi) = f.iter().fold((f64::MAX, f64::MIN), |(a, b), x| (a.min(*x), b.max(*x)));
            let mean = if f.is_empty() {
                f64::NAN
            } else {
                f.iter().sum::<f64>() / f.len() as f64
            };
            (
                k.clone(),
                json!({"estimated": f.len(), "min": finite(lo), "max": finite(hi), "mean": finite(mean)}),
            )
        })
        .collect();
    json!({"origin": g.origin, "size": g.size, "counts": g.n, "rotation": g.rotation, "blocks": m.len(), "assigned": m.assigned(), "domains": doms, "columns": cols})
}

/// Block containing a world point.
fn block_at(m: &blockmodel_engine::BlockModel, p: mining_core::Vec3) -> Option<usize> {
    let g = m.geometry;
    let l = g.world_to_local(p);
    let ijk: Vec<i64> = (0..3).map(|a| (l[a] / g.size[a]).floor() as i64).collect();
    if (0..3).all(|a| ijk[a] >= 0 && (ijk[a] as usize) < g.n[a]) {
        Some(g.index(ijk[0] as usize, ijk[1] as usize, ijk[2] as usize))
    } else {
        None
    }
}

fn bm_sample(m: &blockmodel_engine::BlockModel, args: &Args) -> Outcome {
    let wanted: Option<Vec<String>> = args.get("columns").and_then(Value::as_str).map(|s| {
        s.split(',')
            .map(|c| c.trim().to_string())
            .filter(|c| !c.is_empty())
            .collect()
    });
    let points: Vec<mining_core::Vec3> = match (point(args, "point"), point(args, "from"), point(args, "to")) {
        (Some(p), _, _) => vec![p],
        (None, Some(a), Some(b)) => {
            let step = args
                .get("step")
                .and_then(Value::as_f64)
                .unwrap_or_else(|| m.geometry.size.iter().cloned().fold(f64::MAX, f64::min));
            let len = mining_core::dist(a, b);
            let n = ((len / step).ceil() as usize).clamp(1, 100_000);
            (0..=n).map(|k| mining_core::lerp(a, b, k as f64 / n as f64)).collect()
        }
        _ => return Err(OpError::new(ErrorCode::MissingArgument, "bm_sample needs point, or from and to").at("point")),
    };
    let mut out = Vec::new();
    let mut last: Option<Option<usize>> = None;
    for p in points {
        let b = block_at(m, p);
        if last == Some(b) {
            continue;
        }
        last = Some(b);
        let values: serde_json::Map<String, Value> = match b {
            None => Default::default(),
            Some(i) => m
                .columns
                .iter()
                .filter(|(k, _)| wanted.as_ref().is_none_or(|w| w.iter().any(|c| c == *k)))
                .map(|(k, v)| (k.clone(), finite(v[i] as f64)))
                .collect(),
        };
        out.push(json!({
            "at": p, "block": b, "ijk": b.map(|i| m.geometry.ijk(i)),
            "domain": b.map(|i| m.domain_name(m.domain[i]).to_string()), "values": values,
        }));
    }
    Ok(OpResult::data(json!({"samples": out})))
}

fn bm_resources(m: &blockmodel_engine::BlockModel, cfg: &ProjectConfig, args: &Args) -> Outcome {
    let column = match args.get("column").and_then(Value::as_str) {
        Some(c) => {
            if m.column(c).is_none() {
                return Err(OpError::new(
                    ErrorCode::NotFound,
                    format!(
                        "column '{c}' is not in the model ({})",
                        m.columns.keys().cloned().collect::<Vec<_>>().join(", ")
                    ),
                )
                .at("column"));
            }
            c.to_string()
        }
        None => rs::grade_column(m, &rs::var_name(&cfg.dh.assay_attr))
            .ok_or_else(|| no_data(crate::rsi18n::r().m_bm_no_model))?,
    };
    let cutoffs: Vec<f64> = match args.get("cutoffs").and_then(Value::as_str) {
        Some(t) => t
            .split([',', ';'])
            .filter_map(|x| x.trim().parse::<f64>().ok())
            .collect(),
        None => vec![cfg.rs.bm.view_cutoff],
    };
    if cutoffs.is_empty() {
        return Err(OpError::new(ErrorCode::InvalidArgument, "cutoffs: no number").at("cutoffs"));
    }
    if m.column("tonnage").is_none() {
        return Err(no_data("the block model has no tonnage column: run rs_estimate"));
    }
    let table: Vec<Value> = cutoffs
        .iter()
        .map(|&c| {
            let rows: Vec<Value> = blockmodel_engine::report::summary_by_domain(m, &column, c)
                .into_iter()
                .map(|r| json!({"domain": r.domain, "blocks": r.blocks, "tonnes": finite(r.tonnes), "grade": finite(r.grade), "metal": finite(r.metal), "volume": finite(r.volume)}))
                .collect();
            json!({"cutoff": c, "rows": rows})
        })
        .collect();
    Ok(OpResult::data(json!({"column": column, "table": table})))
}

fn entities(host: &mut dyn HostApi, args: &Args) -> Outcome {
    let classes: Option<Vec<String>> = args.get("class").and_then(Value::as_str).map(|s| {
        s.split(',')
            .map(|c| c.trim().to_ascii_uppercase())
            .filter(|c| !c.is_empty())
            .collect()
    });
    let layer = args.get("layer").and_then(Value::as_str).map(str::to_string);
    let limit = int(args, "limit", 500) as usize;
    let skip = int(args, "offset", 0) as usize;
    let (mut total, mut out) = (0usize, Vec::new());
    let mut offset = 0u64;
    loop {
        let q = format!(r#"{{"xdata_app":"{APP}","offset":{offset},"limit":5000}}"#);
        let Ok(page) = host.query_entities(&q) else { break };
        for e in &page.entities {
            let Some(tag) = tag_of(e) else { continue };
            let c = e.common();
            if classes
                .as_ref()
                .is_some_and(|cl| !cl.iter().any(|k| k == tag.class.key()))
                || layer.as_ref().is_some_and(|l| *l != c.layer)
            {
                continue;
            }
            if total >= skip && out.len() < limit {
                let attrs: serde_json::Map<String, Value> =
                    tag.fields.iter().map(|(k, v)| (k.clone(), json!(v))).collect();
                out.push(json!({"handle": format!("{:X}", c.handle.value()), "class": tag.class.key(), "layer": c.layer, "attributes": attrs}));
            }
            total += 1;
        }
        match page.next_offset {
            Some(n) if n > offset => offset = n,
            _ => break,
        }
    }
    let next = (skip + out.len() < total).then_some(skip + out.len());
    Ok(OpResult::data(
        json!({"total": total, "entities": out, "next_offset": next}),
    ))
}

/// Runs the read-only operation `name`.
pub fn run(host: &mut dyn HostApi, name: &str, args: &Args, tab: u64, cfg: &mut ProjectConfig) -> Outcome {
    match name {
        "status" => Ok(OpResult::data(status(host, tab, cfg))),
        "help" => help(args),
        "dh_list" => dh_list(tab, args),
        "rs_stats" => rs_stats(tab),
        "rs_variogram" => rs_variogram(tab, cfg, args),
        "bm_info" => Ok(OpResult::data(bm_info(&model(host, tab, cfg)?))),
        "bm_sample" => bm_sample(&model(host, tab, cfg)?, args),
        "bm_resources" => bm_resources(&model(host, tab, cfg)?, cfg, args),
        "gt_summary" => {
            let results = state::with_tab(tab, |t| t.gt.results.clone());
            if results.is_empty() {
                return Err(no_data(crate::gti18n::g().m_est_none));
            }
            let mut d = crate::gtcommands::classify_data(crate::gti18n::g(), &results, 0);
            d["drawn"] = Value::Null;
            Ok(OpResult::data(d))
        }
        "entities" => entities(host, args),
        other => Err(OpError::new(
            ErrorCode::UnknownCommand,
            format!("unknown query {other}"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blockmodel_engine::{BlockGeometry, BlockModel};

    fn model() -> BlockModel {
        let mut m = BlockModel::new(BlockGeometry {
            origin: [0.0, 0.0, 0.0],
            size: [10.0, 10.0, 10.0],
            n: [3, 2, 1],
            rotation: 0.0,
        });
        m.domain_names = vec!["A".into()];
        m.domain = vec![1; 6];
        m.fraction = vec![1.0; 6];
        *m.column_mut("cu_ok") = vec![0.1, 0.5, 1.0, 2.0, f32::NAN, 0.3];
        *m.column_mut("tonnage") = vec![1000.0; 6];
        m
    }

    #[test]
    fn sampling_at_a_point_and_along_a_line() {
        let m = model();
        let mut a = Args::new();
        a.insert("point".into(), json!([25.0, 5.0, 5.0]));
        let d = bm_sample(&m, &a).unwrap().data;
        assert_eq!(d["samples"][0]["block"], json!(2));
        assert_eq!(d["samples"][0]["values"]["cu_ok"], json!(1.0));
        let mut a = Args::new();
        a.insert("from".into(), json!([-5.0, 5.0, 5.0]));
        a.insert("to".into(), json!([29.0, 5.0, 5.0]));
        a.insert("columns".into(), json!("cu_ok"));
        let d = bm_sample(&m, &a).unwrap().data;
        let blocks: Vec<Value> = d["samples"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["block"].clone())
            .collect();
        assert_eq!(
            blocks,
            vec![Value::Null, json!(0), json!(1), json!(2)],
            "one entry per block crossed"
        );
        assert_eq!(
            bm_sample(&m, &Args::new()).unwrap_err().code,
            ErrorCode::MissingArgument
        );
    }

    #[test]
    fn info_and_resources() {
        let m = model();
        let i = bm_info(&m);
        assert_eq!(i["columns"]["cu_ok"]["estimated"], json!(5));
        assert_eq!(i["domains"]["A"], json!(6));
        let mut a = Args::new();
        a.insert("column".into(), json!("cu_ok"));
        a.insert("cutoffs".into(), json!("0, 0.4"));
        let d = bm_resources(&m, &ProjectConfig::default(), &a).unwrap().data;
        assert_eq!(d["table"].as_array().unwrap().len(), 2);
        let first = &d["table"][1]["rows"][0];
        assert_eq!(
            (first["domain"].as_str(), first["blocks"].as_u64()),
            (Some("A"), Some(3)),
            "{d}"
        );
        a.insert("column".into(), json!("nope"));
        assert_eq!(
            bm_resources(&m, &ProjectConfig::default(), &a).unwrap_err().code,
            ErrorCode::NotFound
        );
    }
}
