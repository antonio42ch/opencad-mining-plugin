//! Prism model of a vein (`vn_variogram`, `vn_estimate`, `vn_long`,
//! `vn_edit`): stage B2.

use drillhole_engine::color::{grade_color, ColorScheme};
use geostat_engine::model::Kind;
use ocs_plugin_api::host::{Handle, HostApi};
use openmining_ops::parse::Args;
use openmining_ops::result::{ErrorCode, OpError, OpResult};
use serde_json::{json, Value};
use vein_engine::contacts::intercepts;
use vein_engine::model::{
    composites, estimate_prisms, plane_model, validate, variography, Composite, Edit, EditAction, ModelParams, Prism,
    Summary, Validation,
};
use vein_engine::surfaces::Surfaces;

use crate::cad::{CadSink, Geom, LayerDef, Prim};
use crate::charts::{render, Chart, Series, Style};
use crate::config::ProjectConfig;
use crate::host_sink::HostSink;
use crate::i18n::{fmt, lang, Lang};
use crate::interactive::{PickSelection, SelectionKind};
use crate::opsrun::Outcome;
use crate::state::{self, Pending};
use crate::veinconfig::{EditCfg, LongPlacement};
use crate::xdata::{Class, Tag};
use crate::{session, MANIFEST};

pub const PANEL: &str = "min_vein_model";
const STATUS: &str = "vn_model_status";
const CHART: &str = "vn_vario_chart";
pub const LAYER_PRISMS: &str = "MIN_VN_PRISMAS";
pub const LAYER_LONG: &str = "MIN_VN_LONG";

pub struct MStrings {
    pub tool_panel: &'static str,
    pub cls_prism: &'static str,
    pub cls_long: &'static str,
    pub m_no_surfaces: &'static str,
    pub m_no_data: &'static str,
    pub m_few: &'static str,
    pub m_comps: &'static str,
    pub m_model: &'static str,
    pub m_summary: &'static str,
    pub m_out: &'static str,
    pub m_cv: &'static str,
    pub m_check: &'static str,
    pub m_worst: &'static str,
    pub m_saved: &'static str,
    pub m_no_long: &'static str,
    pub m_pick_poly: &'static str,
    pub m_not_poly: &'static str,
    pub m_edits: &'static str,
    pub m_no_model: &'static str,
    pub m_long: &'static str,
    pub chart: [&'static str; 6],
}

pub static ES: MStrings = MStrings {
    tool_panel: "Modelo",
    cls_prism: "Prisma de veta",
    cls_long: "Longitudinal de veta",
    m_no_surfaces: "Veta {0}: primero construya las superficies de techo y piso (MIN_VN_SURFACES).",
    m_no_data: "Cargue los sondajes o canales con la geología de la veta.",
    m_few: "Veta {0}: {1} intersecciones con ley de {2} (se necesitan al menos 3; {3} sin ley).",
    m_comps: "Veta {0}: {1} compósitos de {2}; ley media {3}, potencia media {4} m, acumulación media {5}.",
    m_model: "Modelo de la acumulación: pepita {0}, meseta {1}, alcance en rumbo {2} m, en buzamiento {3} m.",
    m_summary: "{0} prismas: área {1} m², volumen {2} m³, {3} t, ley media {4}, potencia media {5} m ({6} editados).",
    m_out: "Fuera del modelo: {0} por el límite, {1} por la distancia máxima, {2} sin las dos superficies, {3} excluidos, {4} sin muestras suficientes.",
    m_cv: "Validación cruzada de la acumulación ({0} compósitos, sin su propio sondaje): sesgo {1}, correlación {2}.",
    m_check: "Compósitos contra su prisma ({0}): ley media {1} / {2}, correlación {3}.",
    m_worst: "Mayor diferencia: {0} ({1} contra {2} del prisma).",
    m_saved: "Prismas guardados en {0}.",
    m_no_long: "Veta {0}: dibuje primero la longitudinal (MIN_VN_LONG) y trace el polígono sobre ella.",
    m_pick_poly: "Seleccione el polígono dibujado sobre la longitudinal",
    m_not_poly: "La entidad no es una polilínea cerrada de al menos 3 vértices.",
    m_edits: "Veta {0}: {1} ediciones, límite de {2} vértices.",
    m_no_model: "Veta {0}: estime primero los prismas (MIN_VN_ESTIMATE).",
    m_long: "Longitudinal de {0}: {1} prismas y {2} compósitos dibujados en {3}.",
    chart: ["Variograma de la acumulación", "Distancia (m)", "γ", "Rumbo", "Buzamiento", "Omni"],
};

pub static EN: MStrings = MStrings {
    tool_panel: "Model",
    cls_prism: "Vein prism",
    cls_long: "Vein long section",
    m_no_surfaces: "Vein {0}: build the hanging-wall and footwall surfaces first (MIN_VN_SURFACES).",
    m_no_data: "Load the drillholes or channels with the vein geology.",
    m_few: "Vein {0}: {1} intercepts with a {2} grade (at least 3 are needed; {3} without grade).",
    m_comps: "Vein {0}: {1} composites of {2}; mean grade {3}, mean thickness {4} m, mean accumulation {5}.",
    m_model: "Accumulation model: nugget {0}, sill {1}, range along strike {2} m, down dip {3} m.",
    m_summary: "{0} prisms: area {1} m², volume {2} m³, {3} t, mean grade {4}, mean thickness {5} m ({6} edited).",
    m_out: "Out of the model: {0} by the boundary, {1} by the maximum distance, {2} without both surfaces, {3} excluded, {4} without enough samples.",
    m_cv: "Cross-validation of the accumulation ({0} composites, own hole hidden): bias {1}, correlation {2}.",
    m_check: "Composites against their prism ({0}): mean grade {1} / {2}, correlation {3}.",
    m_worst: "Largest difference: {0} ({1} against {2} in the prism).",
    m_saved: "Prisms saved to {0}.",
    m_no_long: "Vein {0}: draw the long section first (MIN_VN_LONG) and draw the polygon on it.",
    m_pick_poly: "Select the polygon drawn on the long section",
    m_not_poly: "The entity is not a closed polyline of at least 3 vertices.",
    m_edits: "Vein {0}: {1} edits, boundary of {2} vertices.",
    m_no_model: "Vein {0}: estimate the prisms first (MIN_VN_ESTIMATE).",
    m_long: "Long section of {0}: {1} prisms and {2} composites drawn at {3}.",
    chart: ["Accumulation variogram", "Distance (m)", "γ", "Strike", "Dip", "Omni"],
};

pub static PT: MStrings = MStrings {
    tool_panel: "Modelo",
    cls_prism: "Prisma de veio",
    cls_long: "Longitudinal de veio",
    m_no_surfaces: "Veio {0}: construa primeiro as superfícies de capa e lapa (MIN_VN_SURFACES).",
    m_no_data: "Carregue os furos ou canais com a geologia do veio.",
    m_few: "Veio {0}: {1} interseções com teor de {2} (são necessárias ao menos 3; {3} sem teor).",
    m_comps: "Veio {0}: {1} compósitos de {2}; teor médio {3}, espessura média {4} m, acumulação média {5}.",
    m_model: "Modelo da acumulação: pepita {0}, patamar {1}, alcance na direção {2} m, no mergulho {3} m.",
    m_summary: "{0} prismas: área {1} m², volume {2} m³, {3} t, teor médio {4}, espessura média {5} m ({6} editados).",
    m_out: "Fora do modelo: {0} pelo limite, {1} pela distância máxima, {2} sem as duas superfícies, {3} excluídos, {4} sem amostras suficientes.",
    m_cv: "Validação cruzada da acumulação ({0} compósitos, sem o próprio furo): viés {1}, correlação {2}.",
    m_check: "Compósitos contra o seu prisma ({0}): teor médio {1} / {2}, correlação {3}.",
    m_worst: "Maior diferença: {0} ({1} contra {2} do prisma).",
    m_saved: "Prismas salvos em {0}.",
    m_no_long: "Veio {0}: desenhe primeiro a longitudinal (MIN_VN_LONG) e trace o polígono sobre ela.",
    m_pick_poly: "Selecione o polígono desenhado sobre a longitudinal",
    m_not_poly: "A entidade não é uma polilinha fechada de ao menos 3 vértices.",
    m_edits: "Veio {0}: {1} edições, limite de {2} vértices.",
    m_no_model: "Veio {0}: estime primeiro os prismas (MIN_VN_ESTIMATE).",
    m_long: "Longitudinal de {0}: {1} prismas e {2} compósitos desenhados em {3}.",
    chart: ["Variograma da acumulação", "Distância (m)", "γ", "Direção", "Mergulho", "Omni"],
};

pub fn m() -> &'static MStrings {
    match lang() {
        Lang::Es => &ES,
        Lang::En => &EN,
        Lang::Pt => &PT,
    }
}

/// Prism model of the tab's vein (last estimate).
#[derive(Debug, Clone, Default)]
pub struct ModelState {
    pub vein: String,
    pub composites: Vec<Composite>,
    pub prisms: Vec<Prism>,
    pub block: [f64; 2],
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

fn kind(i: usize) -> Kind {
    match i {
        1 => Kind::Exponential,
        2 => Kind::Gaussian,
        _ => Kind::Spherical,
    }
}

const KIND_KEYS: [&str; 3] = ["spherical", "exponential", "gaussian"];
const ACTION_KEYS: [&str; 7] = ["exclude", "grade", "thickness", "boundary", "undo", "clear", "list"];

fn no_data(msg: impl Into<String>) -> OpError {
    OpError::new(ErrorCode::NoData, msg)
}

/// Surfaces of the vein: in memory, or read from the file saved next to the drawing.
fn surfaces(host: &dyn HostApi, tab: u64, name: &str) -> Option<Surfaces> {
    let mem = state::with_tab(tab, |t| {
        (t.vein.name == name).then(|| t.vein.surfaces.clone()).flatten()
    });
    mem.or_else(|| {
        let file = crate::veincommands::surface_file(host, name)?;
        let v: Value = serde_json::from_str(&std::fs::read_to_string(file).ok()?).ok()?;
        crate::veincommands::surfaces_from(&v)
    })
}

fn load_composites(host: &dyn HostApi, tab: u64, cfg: &ProjectConfig) -> Result<(Surfaces, Vec<Composite>), OpError> {
    let s = m();
    let x = &cfg.vn;
    let name = x.name.trim();
    let surf = surfaces(host, tab, name).ok_or_else(|| no_data(fmt(s.m_no_surfaces, &[&name])))?;
    let data = state::with_tab(tab, |t| t.data.clone()).ok_or_else(|| no_data(s.m_no_data))?;
    let holes = drillhole_engine::pipeline::build_holes(&data);
    let ints = intercepts(&holes, &x.vein_attr, &x.values());
    let (c, missing) = composites(&holes, &ints, &surf.frame, &x.grade, &x.thick_col);
    if c.len() < 3 {
        return Err(no_data(fmt(s.m_few, &[&name, &c.len(), &x.grade, &missing])));
    }
    Ok((surf, c))
}

fn means(c: &[Composite]) -> (f64, f64, f64) {
    let n = c.len().max(1) as f64;
    let t: f64 = c.iter().map(|x| x.thickness).sum();
    let a: f64 = c.iter().map(|x| x.acc).sum();
    (if t > 0.0 { a / t } else { f64::NAN }, t / n, a / n)
}

fn refresh(host: &mut dyn HostApi, cfg: &ProjectConfig, svg: Option<&str>, lines: &[String]) {
    if let Some(svg) = svg {
        let lists = crate::opsbind::Lists::of(host.tab_id(), cfg);
        if host.register_panel_for_plugin(
            MANIFEST.id,
            crate::opsbind::panel_with(PANEL, cfg, &lists, &[(CHART, svg)]),
        ) {
            host.toggle_panel_for_plugin(MANIFEST.id, PANEL, Some(true));
        }
    }
    crate::opsbind::push_one(host, cfg, PANEL, Some((STATUS, lines.join("\n"))));
}

fn variogram_op(host: &mut dyn HostApi, tab: u64, cfg: &mut ProjectConfig) -> Outcome {
    let s = m();
    let (_, c) = load_composites(host, tab, cfg)?;
    let x = cfg.vn.clone();
    let vg = variography(&c, x.lag, x.nlags, kind(x.model_kind));
    let st = vg.model.structures.first().copied();
    let (nug, sill, rs, rd) = (
        vg.model.nugget,
        st.map_or(0.0, |s| s.sill),
        st.map_or(x.range_strike, |s| s.ranges[0]),
        st.map_or(x.range_dip, |s| s.ranges[1]),
    );
    cfg.vn.nugget = (nug * 1e4).round() / 1e4;
    cfg.vn.sill = (sill * 1e4).round() / 1e4;
    cfg.vn.range_strike = (rs * 10.0).round() / 10.0;
    cfg.vn.range_dip = (rd * 10.0).round() / 10.0;
    let (g, t, a) = means(&c);
    let lines = vec![
        fmt(s.m_comps, &[&x.name, &c.len(), &x.grade, &f(g, 3), &f(t, 2), &f(a, 3)]),
        fmt(
            s.m_model,
            &[
                &f(cfg.vn.nugget, 4),
                &f(cfg.vn.sill, 4),
                &f(cfg.vn.range_strike, 1),
                &f(cfg.vn.range_dip, 1),
            ],
        ),
    ];
    let pts = |e: &geostat_engine::vario::ExpVariogram| -> Vec<(f64, f64)> {
        e.points
            .iter()
            .filter(|p| p.pairs > 0)
            .map(|p| (p.lag, p.gamma))
            .collect()
    };
    let maxh = x.lag * x.nlags as f64;
    let curve = |dir: mining_core::Vec3| -> Vec<(f64, f64)> {
        (0..=40)
            .map(|k| maxh * k as f64 / 40.0)
            .map(|h| (h, vg.model.gamma_along(dir, h)))
            .collect()
    };
    let series = vec![
        Series::new(s.chart[3], pts(&vg.strike), Style::Dots, 0),
        Series::new(s.chart[4], pts(&vg.dip), Style::Dots, 1),
        Series::new(s.chart[5], pts(&vg.omni), Style::Dots, 2),
        Series::new(
            format!("{} ({})", s.chart[3], KIND_KEYS[x.model_kind.min(2)]),
            curve([1.0, 0.0, 0.0]),
            Style::Line,
            0,
        ),
        Series::new(
            format!("{} ({})", s.chart[4], KIND_KEYS[x.model_kind.min(2)]),
            curve([0.0, 1.0, 0.0]),
            Style::Line,
            1,
        ),
        Series::new("σ²", vec![(0.0, vg.variance), (maxh, vg.variance)], Style::Dashed, 3),
    ];
    let svg = render(&Chart {
        title: s.chart[0],
        x_label: s.chart[1],
        y_label: s.chart[2],
        series: &series,
        width: 320.0,
        height: 230.0,
        y_from_zero: true,
    });
    lines.iter().for_each(|l| host.push_info(l));
    state::with_tab(tab, |t| t.config = cfg.clone());
    refresh(host, cfg, Some(&svg), &lines);
    let exp = |e: &geostat_engine::vario::ExpVariogram| -> Vec<Value> {
        e.points
            .iter()
            .map(|p| json!({"lag": p.lag, "gamma": p.gamma, "pairs": p.pairs}))
            .collect()
    };
    Ok(OpResult::data(json!({
        "vein": x.name,
        "composites": c.len(),
        "stats": {"grade_mean": fin(g), "thickness_mean": fin(t), "acc_mean": fin(a), "acc_variance": vg.variance},
        "experimental": {"omni": exp(&vg.omni), "strike": exp(&vg.strike), "dip": exp(&vg.dip)},
        "model": {"kind": KIND_KEYS[x.model_kind.min(2)], "nugget": cfg.vn.nugget, "sill": cfg.vn.sill, "range_strike": cfg.vn.range_strike, "range_dip": cfg.vn.range_dip},
    })))
}

fn params(cfg: &ProjectConfig) -> ModelParams {
    let x = &cfg.vn;
    let e = x.edits.get(x.name.trim()).cloned().unwrap_or_default();
    let edits = e
        .list
        .iter()
        .filter_map(|ed| {
            let action = match ed.action.as_str() {
                "exclude" => EditAction::Exclude,
                "grade" => EditAction::Grade(ed.value),
                "thickness" => EditAction::Thickness(ed.value),
                _ => return None,
            };
            Some(Edit {
                polygon: ed.polygon.clone(),
                action,
            })
        })
        .collect();
    ModelParams {
        block_u: x.block_u,
        block_v: x.block_v,
        model: plane_model(
            kind(x.model_kind),
            x.nugget,
            x.sill.max(1e-9),
            x.range_strike,
            x.range_dip,
        ),
        radius: x.est_radius,
        min_samples: x.est_min,
        max_samples: x.est_max,
        max_dist: x.max_dist,
        disc: x.disc,
        density: x.density,
        boundary: e.boundary,
        edits,
    }
}

fn attr(p: &Prism, by: usize) -> f64 {
    match by {
        1 => p.thickness,
        2 => p.acc,
        3 => p.dist,
        4 => p.variance,
        _ => p.grade,
    }
}

fn colors(prisms: &[Prism], by: usize) -> Vec<u8> {
    let vals: Vec<f64> = prisms.iter().map(|p| attr(p, by)).collect();
    let (lo, hi) = vals
        .iter()
        .filter(|v| v.is_finite())
        .fold((f64::MAX, f64::MIN), |(a, b), v| (a.min(*v), b.max(*v)));
    vals.iter()
        .map(|v| {
            if v.is_finite() {
                grade_color(ColorScheme::BlueToRed, *v, lo, hi.max(lo + 1e-12), &[])
            } else {
                8
            }
        })
        .collect()
}

fn csv(prisms: &[Prism], grade: &str) -> String {
    let mut out =
        format!("i,j,u,v,x,y,z,thickness,area,volume,tonnes,{grade},accumulation,distance,samples,variance,edited\n");
    for p in prisms {
        out.push_str(&format!(
            "{},{},{:.2},{:.2},{:.3},{:.3},{:.3},{:.3},{:.2},{:.2},{:.1},{:.4},{:.4},{:.2},{},{},{}\n",
            p.i,
            p.j,
            p.u,
            p.v,
            p.centre[0],
            p.centre[1],
            p.centre[2],
            p.thickness,
            p.area,
            p.volume,
            p.tonnes,
            p.grade,
            p.acc,
            p.dist,
            p.n,
            if p.variance.is_finite() {
                format!("{:.4}", p.variance)
            } else {
                String::new()
            },
            u8::from(p.edited)
        ));
    }
    out
}

/// Faces of a prism: hanging wall, footwall and the four sides.
const PRISM_FACES: [[usize; 4]; 6] = [
    [0, 1, 2, 3],
    [4, 7, 6, 5],
    [0, 4, 5, 1],
    [1, 5, 6, 2],
    [2, 6, 7, 3],
    [3, 7, 4, 0],
];

fn summary_lines(sum: &Summary, v: &Validation) -> Vec<String> {
    let s = m();
    let mut l = vec![
        fmt(
            s.m_summary,
            &[
                &sum.prisms,
                &f(sum.area, 0),
                &f(sum.volume, 0),
                &f(sum.tonnes, 0),
                &f(sum.grade, 3),
                &f(sum.thickness, 2),
                &sum.edited,
            ],
        ),
        fmt(
            s.m_out,
            &[
                &sum.out_boundary,
                &sum.out_distance,
                &sum.out_walls,
                &sum.out_edits,
                &sum.not_estimated,
            ],
        ),
    ];
    if v.cv_n > 0 {
        l.push(fmt(s.m_cv, &[&v.cv_n, &f(v.cv_mean_error, 3), &f(v.cv_correlation, 2)]));
    }
    if v.n > 0 {
        l.push(fmt(
            s.m_check,
            &[&v.n, &f(v.mean_composite, 3), &f(v.mean_prism, 3), &f(v.correlation, 2)],
        ));
    }
    if let Some((h, a, b)) = v.worst.first() {
        l.push(fmt(s.m_worst, &[h, &f(*a, 3), &f(*b, 3)]));
    }
    l
}

fn estimate_op(host: &mut dyn HostApi, tab: u64, cfg: &mut ProjectConfig) -> Outcome {
    let s = m();
    let (surf, c) = load_composites(host, tab, cfg)?;
    let x = &cfg.vn;
    let name = x.name.trim().to_string();
    let p = params(cfg);
    let (prisms, sum) = estimate_prisms(&surf, &c, &p);
    let v = validate(&c, &prisms, &p);
    let mut lines = summary_lines(&sum, &v);
    let origin = crate::drillholes::origin(cfg);
    let file = crate::resources::model_file(
        host.document_path(host.tab_id()).as_deref(),
        "",
        &format!("_veta_{}_prismas.csv", safe(&name)),
    )
    .and_then(|f| {
        std::fs::write(&f, csv(&prisms, &x.grade))
            .ok()
            .map(|_| f.display().to_string())
    });
    if let Some(fl) = &file {
        lines.push(fmt(s.m_saved, &[fl]));
    }
    let cols = colors(&prisms, x.color_by);
    let prims: Vec<Prim> = if x.draw_prisms {
        prisms
            .iter()
            .zip(&cols)
            .map(|(pr, col)| Prim {
                geom: Geom::Mesh {
                    vertices: pr.corners.iter().map(|q| origin.to_local(*q)).collect(),
                    faces: PRISM_FACES.iter().map(|q| q.to_vec()).collect(),
                },
                layer: LAYER_PRISMS.into(),
                color: Some(*col),
                tag: Tag::new(Class::VnPrism)
                    .with("vein", &name)
                    .with("i", pr.i)
                    .with("j", pr.j)
                    .with(&x.grade, format!("{:.4}", pr.grade))
                    .with("thickness", format!("{:.3}", pr.thickness))
                    .with("tonnes", format!("{:.1}", pr.tonnes)),
            })
            .collect()
    } else {
        Vec::new()
    };
    lines.iter().for_each(|l| host.push_info(l));
    let created = {
        let mut sink = HostSink::new(host);
        sink.begin("MIN_VN_ESTIMATE");
        sink.ensure_layers(&[LayerDef::new(LAYER_PRISMS, 7)]);
        sink.remove_where(&|t: &Tag| t.class == Class::VnPrism && t.get("vein") == Some(name.as_str()));
        sink.add(prims);
        session::commit(&mut sink, crate::i18n::s(), tab, cfg);
        sink.end();
        std::mem::take(&mut sink.created)
    };
    state::with_tab(tab, |t| {
        t.vein_model = ModelState {
            vein: name.clone(),
            composites: c.clone(),
            prisms: prisms.clone(),
            block: [x.block_u, x.block_v],
        }
    });
    // Keep the long section in step with the model.
    if x.long.contains_key(&name) {
        let _ = long_op(host, None, tab, cfg);
    }
    refresh(host, cfg, None, &lines);
    let worst: Vec<Value> = v
        .worst
        .iter()
        .map(|(h, a, b)| json!({"hole": h, "composite": a, "prism": b}))
        .collect();
    Ok(OpResult {
        created,
        ..OpResult::data(json!({
            "vein": name,
            "summary": {"prisms": sum.prisms, "area": sum.area, "volume": sum.volume, "tonnes": sum.tonnes, "grade": fin(sum.grade), "thickness": fin(sum.thickness),
                "edited": sum.edited, "out_boundary": sum.out_boundary, "out_distance": sum.out_distance, "out_walls": sum.out_walls, "out_edits": sum.out_edits, "not_estimated": sum.not_estimated},
            "validation": {"cv_n": v.cv_n, "cv_mean_error": fin(v.cv_mean_error), "cv_correlation": fin(v.cv_correlation), "n": v.n,
                "mean_composite": fin(v.mean_composite), "mean_prism": fin(v.mean_prism), "correlation": fin(v.correlation), "worst": worst},
            "composites": c.len(),
            "file": file,
        }))
    })
}

fn safe(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' })
        .collect()
}

fn long_op(host: &mut dyn HostApi, at: Option<mining_core::Vec3>, tab: u64, cfg: &mut ProjectConfig) -> Outcome {
    let s = m();
    let name = cfg.vn.name.trim().to_string();
    let st = state::with_tab(tab, |t| t.vein_model.clone());
    if st.vein != name || st.prisms.is_empty() {
        return Err(no_data(fmt(s.m_no_model, &[&name])));
    }
    let [bu, bv] = st.block;
    let (u0, v0) = st.prisms.iter().fold((f64::MAX, f64::MAX), |(a, b), p| {
        (a.min(p.u - bu / 2.0), b.min(p.v - bv / 2.0))
    });
    let (u1, v1) = st.prisms.iter().fold((f64::MIN, f64::MIN), |(a, b), p| {
        (a.max(p.u + bu / 2.0), b.max(p.v + bv / 2.0))
    });
    let origin = crate::drillholes::origin(cfg);
    let place = match (at, cfg.vn.long.get(&name)) {
        (Some(a), _) => LongPlacement { at: a, u0, v0 },
        (None, Some(old)) => LongPlacement {
            u0,
            v0,
            at: old.to_drawing(u0, v0),
        },
        (None, None) => {
            // Beside the model, in plan: right of the prisms' bounding box.
            let pts: Vec<mining_core::Vec3> = st
                .prisms
                .iter()
                .flat_map(|p| p.corners)
                .map(|q| origin.to_local(q))
                .collect();
            let (mx, my, mz) = pts.iter().fold((f64::MIN, f64::MAX, f64::MAX), |(a, b, c), q| {
                (a.max(q[0]), b.min(q[1]), c.min(q[2]))
            });
            LongPlacement {
                at: [mx + 0.2 * (u1 - u0).max(20.0), my, mz],
                u0,
                v0,
            }
        }
    };
    cfg.vn.long.insert(name.clone(), place);
    let x = &cfg.vn;
    let cols = colors(&st.prisms, x.color_by);
    let tag = || Tag::new(Class::VnLong).with("vein", &name);
    let pt = |u: f64, v: f64| place.to_drawing(u, v);
    let mut prims: Vec<Prim> = st
        .prisms
        .iter()
        .zip(&cols)
        .map(|(p, col)| Prim {
            geom: Geom::Face(vec![
                pt(p.u - bu / 2.0, p.v - bv / 2.0),
                pt(p.u + bu / 2.0, p.v - bv / 2.0),
                pt(p.u + bu / 2.0, p.v + bv / 2.0),
                pt(p.u - bu / 2.0, p.v + bv / 2.0),
            ]),
            layer: LAYER_LONG.into(),
            color: Some(*col),
            tag: tag()
                .with("i", p.i)
                .with("j", p.j)
                .with("value", format!("{:.4}", attr(p, x.color_by))),
        })
        .collect();
    let th = (bu.min(bv) * 0.3).max(0.2);
    for c in &st.composites {
        prims.push(Prim {
            geom: Geom::Point(pt(c.u, c.v)),
            layer: LAYER_LONG.into(),
            color: Some(7),
            tag: tag().with("hole", &c.hole),
        });
        if x.long_labels {
            prims.push(Prim {
                geom: Geom::Text {
                    at: pt(c.u + th * 0.3, c.v + th * 0.3),
                    height: th,
                    value: format!("{} {:.2}/{:.2}m", c.hole, c.grade, c.thickness),
                },
                layer: LAYER_LONG.into(),
                color: Some(7),
                tag: tag().with("hole", &c.hole),
            });
        }
    }
    let e = x.edits.get(&name).cloned().unwrap_or_default();
    let ring = |poly: &[[f64; 2]], color: u8, what: &str| Prim {
        geom: Geom::Polygon3d(poly.iter().map(|q| pt(q[0], q[1])).collect()),
        layer: LAYER_LONG.into(),
        color: Some(color),
        tag: tag().with("edit", what),
    };
    if e.boundary.len() >= 3 {
        prims.push(ring(&e.boundary, 2, "boundary"));
    }
    for ed in &e.list {
        prims.push(ring(&ed.polygon, 6, &ed.action));
    }
    prims.push(Prim {
        geom: Geom::Polygon3d(vec![pt(u0, v0), pt(u1, v0), pt(u1, v1), pt(u0, v1)]),
        layer: LAYER_LONG.into(),
        color: Some(8),
        tag: tag().with("edit", "frame"),
    });
    prims.push(Prim {
        geom: Geom::Text {
            at: pt(u0, v1 + th * 2.0),
            height: th * 2.0,
            value: format!(
                "{} — {}",
                name,
                ["ley", "potencia", "acumulación", "distancia", "varianza"][x.color_by.min(4)]
            ),
        },
        layer: LAYER_LONG.into(),
        color: Some(7),
        tag: tag().with("edit", "title"),
    });
    let at_txt = format!("{:.1},{:.1},{:.1}", place.at[0], place.at[1], place.at[2]);
    let line = fmt(s.m_long, &[&name, &st.prisms.len(), &st.composites.len(), &at_txt]);
    host.push_info(&line);
    let created = {
        let mut sink = HostSink::new(host);
        sink.begin("MIN_VN_LONG");
        sink.ensure_layers(&[LayerDef::new(LAYER_LONG, 7)]);
        sink.remove_where(&|t: &Tag| t.class == Class::VnLong && t.get("vein") == Some(name.as_str()));
        sink.add(prims);
        session::commit(&mut sink, crate::i18n::s(), tab, cfg);
        sink.end();
        std::mem::take(&mut sink.created)
    };
    Ok(OpResult {
        created,
        ..OpResult::data(
            json!({"vein": name, "at": place.at, "u0": u0, "v0": v0, "width": u1 - u0, "height": v1 - v0, "prisms": st.prisms.len(), "composites": st.composites.len()}),
        )
    })
}

fn polygon_of(host: &dyn HostApi, h: Handle, place: &LongPlacement) -> Option<Vec<[f64; 2]>> {
    let e = host.entities_batch(&[h]).ok()?.into_iter().next()??;
    let mut pts = crate::polyline_points(&e)?;
    if pts.len() >= 2 && mining_core::dist(pts[0], *pts.last()?) < 1e-9 {
        pts.pop();
    }
    (pts.len() >= 3).then(|| pts.iter().map(|q| place.to_plane(*q)).collect())
}

fn edit_with(host: &mut dyn HostApi, tab: u64, cfg: &mut ProjectConfig, polygon: Option<Handle>) -> Outcome {
    let s = m();
    let name = cfg.vn.name.trim().to_string();
    let action = ACTION_KEYS[cfg.vn.edit_action.min(6)];
    let value = cfg.vn.edit_value;
    let needs_poly = matches!(action, "exclude" | "grade" | "thickness" | "boundary");
    if needs_poly {
        let place = *cfg
            .vn
            .long
            .get(&name)
            .ok_or_else(|| no_data(fmt(s.m_no_long, &[&name])))?;
        let h = polygon.ok_or_else(|| OpError::new(ErrorCode::MissingArgument, s.m_pick_poly).at("polygon"))?;
        let poly = polygon_of(host, h, &place)
            .ok_or_else(|| OpError::new(ErrorCode::InvalidArgument, s.m_not_poly).at("polygon"))?;
        let e = cfg.vn.edits.entry(name.clone()).or_default();
        if action == "boundary" {
            e.boundary = poly;
        } else {
            e.list.push(EditCfg {
                action: action.into(),
                value,
                polygon: poly,
            });
        }
    } else if action == "undo" {
        if let Some(e) = cfg.vn.edits.get_mut(&name) {
            e.list.pop();
        }
    } else if action == "clear" {
        cfg.vn.edits.remove(&name);
    }
    let e = cfg.vn.edits.get(&name).cloned().unwrap_or_default();
    let line = fmt(s.m_edits, &[&name, &e.list.len(), &e.boundary.len()]);
    host.push_info(&line);
    let edits: Vec<Value> = e
        .list
        .iter()
        .map(|x| json!({"action": x.action, "value": x.value, "vertices": x.polygon.len()}))
        .collect();
    let estimate = if action == "list" {
        Value::Null
    } else {
        state::with_tab(tab, |t| t.config = cfg.clone());
        let r = estimate_op(host, tab, cfg)?;
        r.data
    };
    Ok(OpResult::data(
        json!({"vein": name, "edits": edits, "boundary": e.boundary.len(), "estimate": estimate}),
    ))
}

pub fn run(host: &mut dyn HostApi, name: &str, args: &Args, tab: u64, cfg: &mut ProjectConfig) -> Outcome {
    match name {
        "vn_variogram" => variogram_op(host, tab, cfg),
        "vn_estimate" => estimate_op(host, tab, cfg),
        "vn_long" => {
            let at = args.get("at").and_then(Value::as_array).and_then(|a| {
                Some([
                    a.first()?.as_f64()?,
                    a.get(1)?.as_f64()?,
                    a.get(2).and_then(Value::as_f64).unwrap_or(0.0),
                ])
            });
            long_op(host, at, tab, cfg)
        }
        "vn_edit" => {
            let needs = matches!(
                ACTION_KEYS[cfg.vn.edit_action.min(6)],
                "exclude" | "grade" | "thickness" | "boundary"
            );
            let given = crate::gtcommands::handles_arg(args, "polygon").and_then(|h| h.first().copied());
            if !needs || given.is_some() {
                return edit_with(host, tab, cfg, given);
            }
            let sel = host.selection();
            if let Some(h) = sel.first() {
                return edit_with(host, tab, cfg, Some(*h));
            }
            state::with_tab(tab, |t| t.config = cfg.clone());
            host.start_interactive(Box::new(PickSelection::new(
                m().m_pick_poly.to_string(),
                SelectionKind::VnPolygon,
            )));
            Ok(OpResult::waiting(m().m_pick_poly))
        }
        other => Err(OpError::new(
            ErrorCode::UnknownCommand,
            format!("unknown vein operation {other}"),
        )),
    }
}

/// Follow-up of the polygon pick of `vn_edit`.
pub fn follow_up(
    host: &mut dyn HostApi,
    command: &str,
    tab: u64,
    cfg: &mut ProjectConfig,
) -> Option<(&'static str, Outcome)> {
    if command != "MIN_VN_EDIT_EXEC" {
        return None;
    }
    let Some(Pending::VnPolygon(h)) = state::take_pending() else {
        return None;
    };
    Some(("vn_edit", edit_with(host, tab, cfg, h.first().copied())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_placement_round_trip_and_csv() {
        let p = LongPlacement {
            at: [100.0, 200.0, 5.0],
            u0: -30.0,
            v0: -10.0,
        };
        assert_eq!(p.to_plane(p.to_drawing(12.0, 7.0)), [12.0, 7.0]);
        let pr = Prism {
            i: 1,
            j: 2,
            u: 0.0,
            v: 0.0,
            centre: [1.0, 2.0, 3.0],
            thickness: 1.2,
            area: 25.0,
            volume: 30.0,
            tonnes: 81.0,
            grade: 5.5,
            acc: 6.6,
            dist: 3.0,
            n: 4,
            variance: f64::NAN,
            edited: true,
            corners: [[0.0; 3]; 8],
        };
        let t = csv(&[pr], "au");
        assert!(t.starts_with("i,j,u,v,x,y,z,thickness,area,volume,tonnes,au,"));
        assert!(t.lines().nth(1).unwrap().ends_with(",4,,1"), "{t}");
    }
}
