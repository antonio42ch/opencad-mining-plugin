//! Tonnage of a vein model (`vn_cubicate`, `vn_rules`, `vn_build`): stage B3.

use ocs_plugin_api::host::HostApi;
use openmining_ops::parse::Args;
use openmining_ops::result::{ErrorCode, OpError, OpResult};
use serde_json::{json, Value};
use vein_engine::reserves::{classify, cubicate, totals, Attrs, Block, BlockParams, Dilution, Line2, Rule, Workings};

use crate::cad::{CadSink, Geom, LayerDef, Prim};
use crate::config::ProjectConfig;
use crate::host_sink::HostSink;
use crate::i18n::{fmt, lang, Lang};
use crate::opsrun::Outcome;
use crate::state;
use crate::xdata::{Class, Tag};
use crate::{session, veinmodel};

pub const PANEL: &str = "min_vein_tonnage";
const STATUS: &str = "vn_tonnage_status";
pub const LAYER_BLOCKS: &str = "MIN_VN_BLOQUES";

pub struct TStrings {
    pub tool_panel: &'static str,
    pub cls_block: &'static str,
    pub m_workings: &'static str,
    pub m_block: &'static str,
    pub m_total: &'static str,
    pub m_unclassified: &'static str,
    pub m_pillars: &'static str,
    pub m_saved: &'static str,
    pub m_no_rules: &'static str,
    pub m_rule_hit: &'static str,
    pub m_rule_none: &'static str,
    pub m_step: &'static str,
}

pub static ES: TStrings = TStrings {
    tool_panel: "Cubicación",
    cls_block: "Bloque de veta",
    m_workings: "Veta {0}: {1} niveles y {2} chimeneas proyectados sobre el plano; {3} bloques.",
    m_block:
        "{0}: {1} — {2} t a {3} (diluido {4} t a {5}, recuperado {6} t); lados muestreados {7}, distancia media {8} m.",
    m_total: "{0}: {1} bloques, {2} t a {3}; diluido {4} t a {5}; recuperado {6} t.",
    m_unclassified: "Sin categoría",
    m_pillars: "Pilares y puentes: {0} t a {1} (no incluidos).",
    m_saved: "Bloques guardados en {0}.",
    m_no_rules: "No hay reglas de categorización: todos los bloques quedan sin categoría.",
    m_rule_hit: "Con esos valores el bloque es {0} (regla {1}).",
    m_rule_none: "Con esos valores el bloque no cumple ninguna regla.",
    m_step: "Construir todo: falló el paso {0}: {1}",
};

pub static EN: TStrings = TStrings {
    tool_panel: "Tonnage",
    cls_block: "Vein block",
    m_workings: "Vein {0}: {1} levels and {2} raises projected on the plane; {3} blocks.",
    m_block: "{0}: {1} — {2} t at {3} (diluted {4} t at {5}, recovered {6} t); sampled sides {7}, mean distance {8} m.",
    m_total: "{0}: {1} blocks, {2} t at {3}; diluted {4} t at {5}; recovered {6} t.",
    m_unclassified: "Unclassified",
    m_pillars: "Pillars: {0} t at {1} (not included).",
    m_saved: "Blocks saved to {0}.",
    m_no_rules: "There are no category rules: every block is unclassified.",
    m_rule_hit: "With those values the block is {0} (rule {1}).",
    m_rule_none: "With those values the block meets no rule.",
    m_step: "Build all: step {0} failed: {1}",
};

pub static PT: TStrings = TStrings {
    tool_panel: "Cubagem",
    cls_block: "Bloco de veio",
    m_workings: "Veio {0}: {1} níveis e {2} chaminés projetados no plano; {3} blocos.",
    m_block:
        "{0}: {1} — {2} t a {3} (diluído {4} t a {5}, recuperado {6} t); lados amostrados {7}, distância média {8} m.",
    m_total: "{0}: {1} blocos, {2} t a {3}; diluído {4} t a {5}; recuperado {6} t.",
    m_unclassified: "Sem categoria",
    m_pillars: "Pilares: {0} t a {1} (não incluídos).",
    m_saved: "Blocos salvos em {0}.",
    m_no_rules: "Não há regras de categorização: todos os blocos ficam sem categoria.",
    m_rule_hit: "Com esses valores o bloco é {0} (regra {1}).",
    m_rule_none: "Com esses valores o bloco não cumpre nenhuma regra.",
    m_step: "Construir tudo: falhou o passo {0}: {1}",
};

pub fn t() -> &'static TStrings {
    match lang() {
        Lang::Es => &ES,
        Lang::En => &EN,
        Lang::Pt => &PT,
    }
}

fn f(x: f64, d: usize) -> String {
    if x.is_finite() {
        format!("{x:.d$}")
    } else {
        "-".into()
    }
}

fn fin(x: f64) -> Value {
    if x.is_finite() {
        json!(x)
    } else {
        Value::Null
    }
}

fn rules(cfg: &ProjectConfig) -> Vec<Rule> {
    cfg.vn
        .rules
        .iter()
        .filter(|r| !r.category.trim().is_empty())
        .map(|r| r.rule())
        .collect()
}

fn rules_json(r: &[Rule]) -> Vec<Value> {
    r.iter().map(|x| json!({"category": x.category, "min_sides": x.min_sides, "max_dist": x.max_dist, "min_samples": x.min_samples, "min_grade": x.min_grade})).collect()
}

/// Level and raise polylines (layers + picked) on the vein plane.
fn workings(
    host: &mut dyn HostApi,
    args: &Args,
    cfg: &ProjectConfig,
    frame: &vein_engine::frame::VeinFrame,
) -> Workings {
    let origin = crate::drillholes::origin(cfg);
    let x = &cfg.vn;
    let mut lines = |layer: &str, key: &str| -> Vec<Line2> {
        let picked = crate::gtcommands::handles_arg(args, key).unwrap_or_default();
        let mut all = crate::veincommands::layer_lines(host, layer);
        all.extend(crate::veincommands::picked_lines(host, &picked));
        all.into_iter()
            .map(|(_, pts)| {
                pts.iter()
                    .map(|p| {
                        let l = frame.to_local(origin.to_world(*p));
                        [l[0], l[1]]
                    })
                    .collect()
            })
            .collect()
    };
    let levels = lines(&x.level_layer, "levels");
    let raises = lines(&x.raise_layer, "raises");
    Workings { levels, raises }.sorted()
}

fn cat_name(r: &[Rule], c: Option<usize>) -> String {
    c.and_then(|i| r.get(i))
        .map_or_else(|| t().m_unclassified.to_string(), |x| x.category.clone())
}

fn csv(blocks: &[Block], r: &[Rule], grade: &str) -> String {
    let mut out = format!("block,category,sides_sampled,samples,distance,prisms,area,thickness,tonnes,{grade},width,diluted_tonnes,diluted_{grade},recovered,pillar_tonnes\n");
    for b in blocks {
        out.push_str(&format!(
            "{},{},{},{},{:.1},{},{:.1},{:.3},{:.1},{:.4},{:.3},{:.1},{:.4},{:.1},{:.1}\n",
            b.name,
            cat_name(r, b.category),
            b.sampled,
            b.samples,
            b.dist,
            b.prisms,
            b.area,
            b.thickness,
            b.tonnes,
            b.grade,
            b.width,
            b.diluted_tonnes,
            b.diluted_grade,
            b.recovered,
            b.pillar_tonnes
        ));
    }
    out
}

pub(crate) fn cubicate_op(host: &mut dyn HostApi, args: &Args, tab: u64, cfg: &mut ProjectConfig) -> Outcome {
    let s = t();
    let name = cfg.vn.name.trim().to_string();
    let st = state::with_tab(tab, |t| t.vein_model.clone());
    if st.vein != name || st.prisms.is_empty() {
        return Err(OpError::new(
            ErrorCode::NoData,
            fmt(veinmodel::m().m_no_model, &[&name]),
        ));
    }
    let surf = veinmodel::surfaces(host, tab, &name)
        .ok_or_else(|| OpError::new(ErrorCode::NoData, fmt(veinmodel::m().m_no_surfaces, &[&name])))?;
    let w = workings(host, args, cfg, &surf.frame);
    let x = cfg.vn.clone();
    let d = Dilution {
        mmw: x.mmw,
        ob_hw: x.ob_hw,
        ob_fw: x.ob_fw,
        grade_hw: x.grade_hw,
        grade_fw: x.grade_fw,
        density_waste: x.density_waste,
        recovery: x.recovery / 100.0,
    };
    let bp = BlockParams {
        level_pillar: x.level_pillar,
        raise_pillar: x.raise_pillar,
        side_tol: x.side_tol,
        side_min: x.side_min,
    };
    let r = rules(cfg);
    let (blocks, assign) = cubicate(&st.prisms, &st.composites, &w, &d, &bp, &r);
    let tot = totals(&blocks, r.len());
    let mut lines = vec![fmt(
        s.m_workings,
        &[&name, &w.levels.len(), &w.raises.len(), &blocks.len()],
    )];
    let mut warnings = Vec::new();
    if r.is_empty() {
        warnings.push(s.m_no_rules.to_string());
        lines.push(s.m_no_rules.to_string());
    }
    for b in &blocks {
        let sides = b
            .sides
            .iter()
            .map(|x| match x {
                Some(true) => "■",
                Some(false) => "□",
                None => "·",
            })
            .collect::<String>();
        lines.push(fmt(
            s.m_block,
            &[
                &b.name,
                &cat_name(&r, b.category),
                &f(b.tonnes, 0),
                &f(b.grade, 3),
                &f(b.diluted_tonnes, 0),
                &f(b.diluted_grade, 3),
                &f(b.recovered, 0),
                &format!("{} {sides}", b.sampled),
                &f(b.dist, 1),
            ],
        ));
    }
    let mut tot_json = Vec::new();
    for (i, x) in tot.iter().enumerate() {
        let cat = if i < r.len() {
            r[i].category.clone()
        } else {
            s.m_unclassified.to_string()
        };
        if x.blocks > 0 {
            lines.push(fmt(
                s.m_total,
                &[
                    &cat,
                    &x.blocks,
                    &f(x.tonnes, 0),
                    &f(x.grade, 3),
                    &f(x.diluted_tonnes, 0),
                    &f(x.diluted_grade, 3),
                    &f(x.recovered, 0),
                ],
            ));
        }
        tot_json.push(json!({"category": cat, "blocks": x.blocks, "tonnes": x.tonnes, "grade": fin(x.grade), "diluted_tonnes": x.diluted_tonnes, "diluted_grade": fin(x.diluted_grade), "recovered": x.recovered}));
    }
    let pt: f64 = blocks.iter().map(|b| b.pillar_tonnes).sum();
    let pg = if pt > 0.0 {
        blocks.iter().map(|b| b.pillar_grade * b.pillar_tonnes).sum::<f64>() / pt
    } else {
        f64::NAN
    };
    if pt > 0.0 {
        lines.push(fmt(s.m_pillars, &[&f(pt, 0), &f(pg, 3)]));
    }
    let file = crate::resources::model_file(
        host.document_path(host.tab_id()).as_deref(),
        "",
        &format!(
            "_veta_{}_bloques.csv",
            name.chars()
                .map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' })
                .collect::<String>()
        ),
    )
    .and_then(|fl| {
        std::fs::write(&fl, csv(&blocks, &r, &x.grade))
            .ok()
            .map(|_| fl.display().to_string())
    });
    if let Some(fl) = &file {
        lines.push(fmt(s.m_saved, &[fl]));
    }
    // Categories of the prisms for the long section.
    let cats: Vec<i32> = assign
        .iter()
        .map(|a| {
            if a.pillar {
                -2
            } else {
                blocks[a.block].category.map_or(-1, |c| c as i32)
            }
        })
        .collect();
    state::with_tab(tab, |tb| tb.vein_model.cats = cats);
    lines.iter().for_each(|l| host.push_info(l));
    // Workings and block labels on the long section, when drawn.
    let mut created = Vec::new();
    if let Some(place) = cfg.vn.long.get(&name).copied() {
        let tag = || Tag::new(Class::VnBlock).with("vein", &name);
        let pt2 = |u: f64, v: f64| place.to_drawing(u, v);
        let [bu, bv] = st.block;
        let th = (bu.min(bv) * 0.35).max(0.25);
        let mut prims = Vec::new();
        let (umin, umax) = st
            .prisms
            .iter()
            .fold((f64::MAX, f64::MIN), |(a, b), p| (a.min(p.u - bu), b.max(p.u + bu)));
        let (vmin, vmax) = st
            .prisms
            .iter()
            .fold((f64::MAX, f64::MIN), |(a, b), p| (a.min(p.v - bv), b.max(p.v + bv)));
        for (k, l) in w.levels.iter().enumerate() {
            let pts: Vec<_> = (0..=20)
                .map(|i| umin + (umax - umin) * i as f64 / 20.0)
                .map(|u| pt2(u, vein_engine::reserves::v_at(l, u)))
                .collect();
            prims.push(Prim {
                geom: Geom::Polyline3d(pts.clone()),
                layer: LAYER_BLOCKS.into(),
                color: Some(1),
                tag: tag().with("working", format!("L{}", k + 1)),
            });
            prims.push(Prim {
                geom: Geom::Text {
                    at: pts[0],
                    height: th,
                    value: format!("L{}", k + 1),
                },
                layer: LAYER_BLOCKS.into(),
                color: Some(1),
                tag: tag().with("working", format!("L{}", k + 1)),
            });
        }
        for (k, rl) in w.raises.iter().enumerate() {
            let pts: Vec<_> = (0..=20)
                .map(|i| vmin + (vmax - vmin) * i as f64 / 20.0)
                .map(|v| pt2(vein_engine::reserves::u_at(rl, v), v))
                .collect();
            prims.push(Prim {
                geom: Geom::Polyline3d(pts.clone()),
                layer: LAYER_BLOCKS.into(),
                color: Some(1),
                tag: tag().with("working", format!("R{}", k + 1)),
            });
            prims.push(Prim {
                geom: Geom::Text {
                    at: pts[pts.len() - 1],
                    height: th,
                    value: format!("R{}", k + 1),
                },
                layer: LAYER_BLOCKS.into(),
                color: Some(1),
                tag: tag().with("working", format!("R{}", k + 1)),
            });
        }
        for (b, bl) in blocks.iter().enumerate() {
            let mine: Vec<_> = st
                .prisms
                .iter()
                .zip(&assign)
                .filter(|(_, a)| a.block == b && !a.pillar)
                .map(|(p, _)| p)
                .collect();
            if mine.is_empty() {
                continue;
            }
            let n = mine.len() as f64;
            let (cu, cv) = (
                mine.iter().map(|p| p.u).sum::<f64>() / n,
                mine.iter().map(|p| p.v).sum::<f64>() / n,
            );
            let col = veinmodel::cat_color(bl.category.map_or(-1, |c| c as i32));
            for (row, text) in [
                format!("{} {}", bl.name, cat_name(&r, bl.category)),
                format!("{} t {}", f(bl.diluted_tonnes, 0), f(bl.diluted_grade, 2)),
            ]
            .into_iter()
            .enumerate()
            {
                prims.push(Prim {
                    geom: Geom::Text {
                        at: pt2(cu - 3.0 * th, cv - row as f64 * th * 1.5),
                        height: th,
                        value: text,
                    },
                    layer: LAYER_BLOCKS.into(),
                    color: Some(col),
                    tag: tag()
                        .with("block", &bl.name)
                        .with("category", cat_name(&r, bl.category)),
                });
            }
        }
        let mut sink = HostSink::new(host);
        sink.begin("MIN_VN_CUBICATE");
        sink.ensure_layers(&[LayerDef::new(LAYER_BLOCKS, 1)]);
        sink.remove_where(&|tg: &Tag| tg.class == Class::VnBlock && tg.get("vein") == Some(name.as_str()));
        sink.add(prims);
        session::commit(&mut sink, crate::i18n::s(), tab, cfg);
        sink.end();
        created = std::mem::take(&mut sink.created);
        if cfg.vn.color_by == 5 {
            let _ = veinmodel::long_op(host, None, tab, cfg);
        }
    } else {
        state::with_tab(tab, |tb| tb.config = cfg.clone());
    }
    crate::opsbind::push_one(host, cfg, PANEL, Some((STATUS, lines.join("\n"))));
    let blocks_json: Vec<Value> = blocks
        .iter()
        .map(|b| {
            json!({"name": b.name, "category": b.category.map(|c| r[c].category.clone()), "sides": b.sides, "sampled": b.sampled, "samples": b.samples,
                "dist": fin(b.dist), "prisms": b.prisms, "area": b.area, "thickness": fin(b.thickness), "tonnes": b.tonnes, "grade": fin(b.grade),
                "width": fin(b.width), "diluted_tonnes": b.diluted_tonnes, "diluted_grade": fin(b.diluted_grade), "recovered": b.recovered,
                "pillar_tonnes": b.pillar_tonnes, "why": b.why})
        })
        .collect();
    Ok(OpResult {
        created,
        warnings,
        ..OpResult::data(json!({
            "vein": name,
            "workings": {"levels": w.levels.len(), "raises": w.raises.len()},
            "blocks": blocks_json,
            "totals": tot_json,
            "pillars": {"tonnes": pt, "grade": fin(pg)},
            "file": file,
        }))
    })
}

fn rules_op(host: &mut dyn HostApi, args: &Args, cfg: &ProjectConfig) -> Outcome {
    let s = t();
    let r = rules(cfg);
    let given = ["sides", "dist", "samples", "grade"]
        .iter()
        .any(|k| args.contains_key(*k));
    let test = if given {
        let num = |k: &str| args.get(k).and_then(Value::as_f64);
        let a = Attrs {
            sides: num("sides").unwrap_or(0.0) as usize,
            dist: num("dist").unwrap_or(0.0),
            samples: num("samples").unwrap_or(0.0) as usize,
            grade: num("grade").unwrap_or(f64::INFINITY),
        };
        let (hit, why) = classify(&r, &a);
        let line = match hit {
            Some(i) => fmt(s.m_rule_hit, &[&r[i].category, &(i + 1)]),
            None => s.m_rule_none.to_string(),
        };
        host.push_info(&line);
        why.iter().for_each(|w| host.push_info(&format!("  {w}")));
        json!({"values": {"sides": a.sides, "dist": a.dist, "samples": a.samples, "grade": fin(a.grade)}, "rule": hit.map(|i| i + 1), "category": hit.map(|i| r[i].category.clone()), "why": why})
    } else {
        Value::Null
    };
    Ok(OpResult::data(json!({"rules": rules_json(&r), "test": test})))
}

fn build_op(host: &mut dyn HostApi, args: &Args, tab: u64, cfg: &mut ProjectConfig) -> Outcome {
    let s = t();
    let step = |n: &str, e: OpError| OpError {
        message: fmt(s.m_step, &[&n, &e.message]),
        ..e
    };
    let surfaces = crate::veincommands::run(host, "vn_surfaces", args, tab, cfg)
        .map_err(|e| step("vn_surfaces", e))?
        .data;
    let variogram = if cfg.vn.build_fit {
        Some(
            veinmodel::variogram_op(host, tab, cfg)
                .map_err(|e| step("vn_variogram", e))?
                .data,
        )
    } else {
        None
    };
    let estimate = veinmodel::estimate_op(host, tab, cfg)
        .map_err(|e| step("vn_estimate", e))?
        .data;
    let cub = if cfg.vn.build_cubicate {
        Some(
            cubicate_op(host, args, tab, cfg)
                .map_err(|e| step("vn_cubicate", e))?
                .data,
        )
    } else {
        None
    };
    Ok(OpResult::data(
        json!({"vein": cfg.vn.name.trim(), "steps": {"surfaces": surfaces, "variogram": variogram, "estimate": estimate, "cubicate": cub}}),
    ))
}

pub fn run(host: &mut dyn HostApi, name: &str, args: &Args, tab: u64, cfg: &mut ProjectConfig) -> Outcome {
    match name {
        "vn_cubicate" => cubicate_op(host, args, tab, cfg),
        "vn_rules" => rules_op(host, args, cfg),
        "vn_build" => build_op(host, args, tab, cfg),
        other => Err(OpError::new(
            ErrorCode::UnknownCommand,
            format!("unknown vein operation {other}"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_csv_has_one_row_per_block() {
        let r = vec![Rule {
            category: "Medido".into(),
            ..Default::default()
        }];
        let b = Block {
            name: "L1-L2".into(),
            category: Some(0),
            sampled: 2,
            tonnes: 100.0,
            grade: 5.0,
            ..Default::default()
        };
        let u = Block {
            name: "<L1".into(),
            category: None,
            ..Default::default()
        };
        let text = csv(&[b, u], &r, "au");
        let rows: Vec<&str> = text.lines().collect();
        assert!(rows[0].starts_with("block,category,") && rows[0].contains(",au,") && rows[0].contains("diluted_au"));
        assert!(rows[1].starts_with("L1-L2,Medido,2,"));
        assert!(rows[2].starts_with("<L1,"));
        assert_eq!(rows.len(), 3);
    }
}
