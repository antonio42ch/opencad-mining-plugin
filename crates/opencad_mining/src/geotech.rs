//! Geotechnical module bound to the plugin: loading the tables, classifying
//! and locating every record, drawing them, the Hoek-Brown calculator and
//! the geotechnical columns of the block model. No host calls: commands in
//! `gtcommands` feed it and draw through a [`CadSink`].

use std::collections::{BTreeMap, HashMap};

use blockmodel_engine::estimate::{estimate_model, DomainPlan, Plan};
use blockmodel_engine::region::{assign_domains, Everything, Region};
use blockmodel_engine::{BlockGeometry, BlockModel};
use drillhole_engine::error::TableKind;
use drillhole_engine::pipeline::HoleModel;
use drillhole_engine::table::RawTable;
use geostat_engine::model::{Orientation, VariogramModel};
use geostat_engine::search::Neighborhood;
use geostat_engine::Sample;
use geotech_engine::classify::{classify, summarize, Classification, Lab};
use geotech_engine::data::{parse_table, GtError, GtField, GtKind, GtTable};
use geotech_engine::hoek_brown::{rock_mass_modulus, sigma3_max, HoekBrown};
use geotech_engine::locate::{locate_interval, locate_mapping, LocateError, Located};
use geotech_engine::q::q_class;
use geotech_engine::rmr_class;
use mining_core::{dist, LocalOrigin, Vec3};

use crate::cad::{CadSink, Geom, LayerDef, Prim};
use crate::charts::{render, Chart, Series, Style};
use crate::config::ProjectConfig;
use crate::gtconfig::GeotechConfig;
use crate::gti18n::GtStrings;
use crate::i18n::fmt;
use crate::resources::num;
use crate::xdata::{Class, Tag};

pub const LAYER_HOLES: &str = "MIN_GT_SONDAJE";
pub const LAYER_MAP: &str = "MIN_GT_MAPEO";
/// Block model columns of the geotechnical estimation (the view attributes follow this order):
/// RMR, GSI, log Q, class, mean distance to the records used, number of records used.
pub const COLUMNS: [&str; 6] = ["gt_rmr", "gt_gsi", "gt_logq", "gt_class", "gt_dist", "gt_n"];
const GRAY: u8 = 8;

/// ACI colour of rock mass class I..V (blue, green, yellow, orange, red).
pub fn class_color(class: u8) -> u8 {
    [5, 3, 2, 30, 1][(class.clamp(1, 5) - 1) as usize]
}

fn q_color(q: f64) -> u8 {
    match q_class(q) {
        1..=3 => 5,
        4 => 3,
        5 => 2,
        6 => 30,
        7 => 1,
        _ => 6,
    }
}

pub fn color_of(c: &Classification, by: usize) -> u8 {
    let v = match by {
        1 => c.q.map(q_color),
        2 => c.gsi.map(|g| class_color(rmr_class(g))),
        _ => c.class.map(class_color),
    };
    v.unwrap_or(GRAY)
}

/// Loaded tables of a tab.
#[derive(Debug, Clone, Default)]
pub struct GtState {
    pub logging: Option<GtTable>,
    pub mapping: Option<GtTable>,
    pub lab: Lab,
    pub lab_rows: usize,
    pub joints: Option<GtTable>,
    pub results: Vec<GtResult>,
}

impl GtState {
    pub fn is_empty(&self) -> bool {
        self.logging.is_none() && self.mapping.is_none() && self.lab_rows == 0 && self.joints.is_none()
    }
    /// Mapping records that need an axis (no coordinates).
    pub fn needs_axes(&self) -> bool {
        self.mapping.as_ref().is_some_and(|t| {
            t.records
                .iter()
                .any(|r| r.num(GtField::X).is_none() || r.num(GtField::Y).is_none())
        })
    }
}

/// One classified record.
#[derive(Debug, Clone, PartialEq)]
pub struct GtResult {
    pub kind: GtKind,
    pub row: usize,
    /// Hole id, or the mapping axis / row.
    pub label: String,
    pub lith: Option<String>,
    pub at: Result<Located, LocateError>,
    pub c: Classification,
    /// Interval length (support), 1 for points.
    pub length: f64,
}

pub fn kind_name(g: &GtStrings, k: GtKind) -> &'static str {
    g.kinds[match k {
        GtKind::Logging => 0,
        GtKind::Mapping => 1,
        GtKind::Lab => 2,
        GtKind::Joints => 3,
    }]
}

/// Column chooser: `(kind, headers, fields)` -> overrides, `None` = cancel.
pub type GtAsk<'a> = dyn FnMut(GtKind, &[String], &[GtField]) -> Option<BTreeMap<String, String>> + 'a;

/// Reads and parses one table, asking for the columns it cannot find; the
/// choice is remembered in `cfg.column_maps`.
pub fn read_table(
    g: &GtStrings,
    path: &str,
    kind: GtKind,
    cfg: &mut GeotechConfig,
    ask: &mut GtAsk,
) -> Result<GtTable, String> {
    let raw = RawTable::from_path(std::path::Path::new(path.trim()), TableKind::Geo)
        .map_err(|e| crate::i18n::s().import_error(&e))?;
    let name = kind_name(g, kind);
    let mut overrides = cfg.column_map(kind.key());
    for attempt in 0..2 {
        match parse_table(&raw, kind, &overrides) {
            Ok(t) => {
                cfg.column_maps.insert(kind.key().into(), overrides);
                return Ok(t);
            }
            // A remembered column that is no longer in the file: forget it.
            Err(GtError::UnknownHeader { .. }) if attempt == 0 => overrides.clear(),
            Err(GtError::MissingColumns { missing, headers }) if attempt == 0 => {
                let mut fields = missing.clone();
                if kind == GtKind::Mapping && !fields.contains(&GtField::Chainage) {
                    fields.push(GtField::Chainage);
                }
                match ask(kind, &headers, &fields) {
                    Some(chosen) => overrides.extend(chosen),
                    None => {
                        return Err(fmt(
                            g.m_load_err,
                            &[&name, &GtError::MissingColumns { missing, headers }],
                        ))
                    }
                }
            }
            Err(e) => return Err(fmt(g.m_load_err, &[&name, &e])),
        }
    }
    Err(fmt(g.m_load_err, &[&name, &GtError::Empty]))
}

/// Loads every configured table. Lines are `(is_error, text)`.
pub fn load(g: &GtStrings, cfg: &mut GeotechConfig, ask: &mut GtAsk) -> Result<(GtState, Vec<(bool, String)>), String> {
    let paths = [
        (GtKind::Logging, cfg.logging.clone()),
        (GtKind::Mapping, cfg.mapping.clone()),
        (GtKind::Lab, cfg.lab.clone()),
        (GtKind::Joints, cfg.joints.clone()),
    ];
    if paths.iter().all(|(_, p)| p.trim().is_empty()) {
        return Err(g.m_no_files.into());
    }
    let mut st = GtState::default();
    let mut lines = Vec::new();
    for (kind, path) in paths {
        if path.trim().is_empty() {
            continue;
        }
        match read_table(g, &path, kind, cfg, ask) {
            Err(e) => lines.push((true, e)),
            Ok(t) => {
                let cols: Vec<&str> = t.mapping.iter().map(|(f, _)| f.key()).collect();
                let unused = if t.unmapped.is_empty() {
                    "-".to_string()
                } else {
                    t.unmapped.join(", ")
                };
                lines.push((
                    false,
                    fmt(
                        g.m_loaded,
                        &[&kind_name(g, kind), &t.records.len(), &cols.join(", "), &unused],
                    ),
                ));
                match kind {
                    GtKind::Logging => st.logging = Some(t),
                    GtKind::Mapping => st.mapping = Some(t),
                    GtKind::Lab => {
                        st.lab = Lab::from_table(&t);
                        st.lab_rows = st.lab.0.len();
                        lines.push((false, fmt(g.m_lab_rows, &[&st.lab_rows])));
                    }
                    GtKind::Joints => {
                        lines.push((false, fmt(g.m_joints, &[&t.records.len()])));
                        st.joints = Some(t);
                    }
                }
            }
        }
    }
    if st.is_empty() {
        let first = lines
            .iter()
            .find(|(e, _)| *e)
            .map(|(_, l)| l.clone())
            .unwrap_or_else(|| g.m_no_files.into());
        return Err(first);
    }
    Ok((st, lines))
}

/// Classifies and locates every logging and mapping record.
pub fn classify_all(
    st: &GtState,
    holes: &[HoleModel],
    axes: &[(String, Vec<Vec3>)],
    cfg: &GeotechConfig,
) -> Vec<GtResult> {
    let opts = cfg.options();
    let mut out = Vec::new();
    for (kind, table) in [(GtKind::Logging, &st.logging), (GtKind::Mapping, &st.mapping)] {
        let Some(t) = table else { continue };
        for r in &t.records {
            let at = if kind == GtKind::Logging {
                locate_interval(r, holes)
            } else {
                locate_mapping(r, axes)
            };
            let label = match kind {
                GtKind::Logging => r.text(GtField::Hole).unwrap_or("").to_string(),
                _ => r
                    .text(GtField::Axis)
                    .map_or_else(|| format!("#{}", r.row), str::to_string),
            };
            let length = match (r.num(GtField::From), r.num(GtField::To)) {
                (Some(a), Some(b)) if b > a => b - a,
                _ => 1.0,
            };
            out.push(GtResult {
                kind,
                row: r.row,
                label,
                lith: r.text(GtField::Lith).map(str::to_string),
                at,
                c: classify(r, &st.lab, &opts),
                length,
            });
        }
    }
    out
}

/// Report of a classification (info lines).
pub fn report(g: &GtStrings, results: &[GtResult]) -> Vec<String> {
    let cs: Vec<Classification> = results.iter().map(|r| r.c.clone()).collect();
    let sm = summarize(&cs);
    let mut lines = vec![fmt(
        g.m_classified,
        &[&sm.total, &sm.with_rmr, &sm.with_q, &sm.with_gsi, &sm.unclassified],
    )];
    let mut classes = [0usize; 5];
    for c in &cs {
        if let Some(k) = c.class {
            classes[(k - 1) as usize] += 1;
        }
    }
    if sm.with_rmr > 0 {
        lines.push(fmt(
            g.m_classes,
            &[&classes[0], &classes[1], &classes[2], &classes[3], &classes[4]],
        ));
    }
    if !sm.missing.is_empty() {
        let mut m: Vec<(GtField, usize)> = sm.missing.into_iter().collect();
        m.sort_by_key(|a| std::cmp::Reverse(a.1));
        let list: Vec<String> = m.iter().map(|(f, n)| format!("{} {n}", g.field(*f))).collect();
        lines.push(fmt(g.m_missing, &[&list.join(", ")]));
    }
    let mut derived: BTreeMap<&str, usize> = BTreeMap::new();
    for c in &cs {
        for d in &c.derived {
            *derived.entry(g.derived(*d)).or_default() += 1;
        }
    }
    if !derived.is_empty() {
        let list: Vec<String> = derived.iter().map(|(k, n)| format!("{k} {n}")).collect();
        lines.push(fmt(g.m_derived, &[&list.join(", ")]));
    }
    let invalid: Vec<&GtResult> = results.iter().filter(|r| !r.c.invalid.is_empty()).collect();
    for r in invalid.iter().take(5) {
        let (f, v) = &r.c.invalid[0];
        lines.push(fmt(g.m_invalid, &[&kind_name(g, r.kind), &r.row, &g.field(*f), v]));
    }
    if invalid.len() > 5 {
        lines.push(fmt(g.m_invalid_more, &[&(invalid.len() - 5)]));
    }
    let unlocated: Vec<&GtResult> = results.iter().filter(|r| r.at.is_err()).collect();
    if let Some(first) = unlocated.first() {
        let why = g.locate_error(first.at.as_ref().unwrap_err());
        lines.push(fmt(
            g.m_unlocated,
            &[
                &unlocated.len(),
                &format!("{} {} {}: {why}", kind_name(g, first.kind), first.row, first.label),
            ],
        ));
    }
    lines
}

fn short(v: Option<f64>, digits: usize) -> String {
    v.map_or("-".into(), |x| format!("{x:.digits$}"))
}

fn tag_of(r: &GtResult) -> Tag {
    let c = &r.c;
    let mut t = Tag::new(Class::GtRec)
        .with("src", r.kind.key())
        .with("row", r.row)
        .with("id", &r.label);
    for (k, v) in [("rmr", c.rmr), ("q", c.q), ("qp", c.q_prime), ("gsi", c.gsi)] {
        if let Some(v) = v {
            t = t.with(k, num(v));
        }
    }
    if let Some(k) = c.class {
        t = t.with("class", k);
    }
    if let Some(l) = &r.lith {
        t = t.with("lith", l);
    }
    t
}

/// Entities of the classified records (located ones only).
pub fn scene(results: &[GtResult], cfg: &GeotechConfig, origin: LocalOrigin) -> Vec<Prim> {
    let mut out = Vec::new();
    for r in results {
        let Ok(at) = &r.at else { continue };
        let layer = if r.kind == GtKind::Logging {
            LAYER_HOLES
        } else {
            LAYER_MAP
        };
        let color = Some(color_of(&r.c, cfg.color_by));
        let (a, b) = (origin.to_local(at.from), origin.to_local(at.to));
        let geom = if dist(a, b) > 1e-6 {
            Geom::Polyline3d(vec![a, b])
        } else {
            Geom::Point(a)
        };
        out.push(Prim {
            geom,
            layer: layer.into(),
            color,
            tag: tag_of(r),
        });
        if r.kind == GtKind::Mapping && cfg.labels {
            let c = origin.to_local(at.centre);
            let text = format!(
                "RMR {} · Q {} · GSI {}",
                short(r.c.rmr, 0),
                short(r.c.q, 2),
                short(r.c.gsi, 0)
            );
            out.push(Prim {
                geom: Geom::Text {
                    at: [c[0], c[1], c[2] + cfg.label_height],
                    height: cfg.label_height,
                    value: text,
                },
                layer: LAYER_MAP.into(),
                color,
                tag: tag_of(r).with("label", 1),
            });
        }
    }
    out
}

pub fn draw(
    sink: &mut dyn CadSink,
    g: &GtStrings,
    cfg: &GeotechConfig,
    results: &[GtResult],
    origin: LocalOrigin,
) -> usize {
    let prims = scene(results, cfg, origin);
    sink.begin("MIN_GT_CLASSIFY");
    sink.ensure_layers(&[LayerDef::new(LAYER_HOLES, 7), LayerDef::new(LAYER_MAP, 7)]);
    sink.remove_where(&|t: &Tag| t.class == Class::GtRec);
    let n = sink.add(prims);
    sink.end();
    sink.info(&fmt(g.m_drawn, &[&n]));
    n
}

pub fn clear(sink: &mut dyn CadSink, g: &GtStrings) -> usize {
    sink.begin("MIN_GT_CLEAR");
    let n = sink.remove_where(&|t: &Tag| t.class == Class::GtRec);
    sink.end();
    sink.info(&fmt(g.m_cleared, &[&n]));
    n
}

// ─────────────────────────── Hoek-Brown ───────────────────────────

pub struct HbOutcome {
    pub hb: HoekBrown,
    pub erm: f64,
    pub s3max: f64,
    pub c: f64,
    pub phi: f64,
    pub text: String,
    pub svg: String,
}

/// Hoek-Brown parameters, strength and the equivalent Mohr-Coulomb fit of
/// the panel values, with the envelope chart.
pub fn hoek_brown(g: &GtStrings, cfg: &GeotechConfig) -> Result<HbOutcome, String> {
    let hb = HoekBrown::new(cfg.hb_sigci, cfg.hb_gsi, cfg.hb_mi, cfg.hb_d).map_err(|e| fmt(g.m_hb_invalid, &[&e]))?;
    if cfg.hb_gamma <= 0.0 || cfg.hb_depth <= 0.0 {
        return Err(fmt(
            g.m_hb_invalid,
            &[&format!("γ = {}, H = {}", cfg.hb_gamma, cfg.hb_depth)],
        ));
    }
    let s3max = sigma3_max(&hb, cfg.application(), cfg.hb_gamma / 1000.0, cfg.hb_depth);
    let (c, phi) = hb.mohr_coulomb(s3max);
    let erm = rock_mass_modulus(cfg.hb_gsi, cfg.hb_d, (cfg.hb_ei > 0.0).then_some(cfg.hb_ei));
    let text = fmt(
        g.m_hb,
        &[
            &format!("{:.3}", hb.mb),
            &format!("{:.5}", hb.s),
            &format!("{:.3}", hb.a),
            &format!("{:.2}", hb.sigma_c()),
            &format!("{:.3}", hb.sigma_t()),
            &format!("{:.2}", hb.sigma_cm()),
            &format!("{:.0}", erm),
            &format!("{:.2}", s3max),
            &format!("{:.3}", c),
            &format!("{:.1}", phi),
        ],
    );
    let n = 40;
    let hb_pts: Vec<(f64, f64)> = (0..=n)
        .map(|i| {
            let s3 = s3max * i as f64 / n as f64;
            (s3, hb.sigma1(s3))
        })
        .collect();
    let sp = phi.to_radians().sin();
    let mc = |s3: f64| 2.0 * c * phi.to_radians().cos() / (1.0 - sp) + (1.0 + sp) / (1.0 - sp) * s3;
    let series = [
        Series::new("Hoek-Brown", hb_pts, Style::Line, 0),
        Series::new(
            "Mohr-Coulomb",
            vec![(0.0, mc(0.0)), (s3max, mc(s3max))],
            Style::Dashed,
            1,
        ),
    ];
    let svg = render(&Chart {
        title: g.chart_title,
        x_label: "σ3 (MPa)",
        y_label: "σ1 (MPa)",
        series: &series,
        width: 320.0,
        height: 230.0,
        y_from_zero: true,
    });
    Ok(HbOutcome {
        hb,
        erm,
        s3max,
        c,
        phi,
        text,
        svg,
    })
}

/// Fills GSI (median of the classified records, of `hb_lith` when given)
/// and σci / mi from the laboratory row of `hb_lith`.
/// What `hb_from_data` used: message, records of the median, laboratory row found.
pub struct HbData {
    pub message: String,
    pub records: usize,
    pub lab: bool,
}

pub fn hb_from_data(g: &GtStrings, cfg: &mut GeotechConfig, st: &GtState) -> Result<HbData, String> {
    let lith = cfg.hb_lith.trim().to_lowercase();
    let pick = |filter: bool| -> Vec<f64> {
        st.results
            .iter()
            .filter(|r| !filter || r.lith.as_deref().is_some_and(|l| l.trim().to_lowercase() == lith))
            .filter_map(|r| r.c.gsi)
            .collect()
    };
    let mut gsis = if lith.is_empty() { pick(false) } else { pick(true) };
    if gsis.is_empty() {
        gsis = pick(false);
    }
    if gsis.is_empty() {
        return Err(g.m_hb_no_gsi.into());
    }
    gsis.sort_by(f64::total_cmp);
    let n = gsis.len();
    let median = if n % 2 == 1 {
        gsis[n / 2]
    } else {
        (gsis[n / 2 - 1] + gsis[n / 2]) / 2.0
    };
    cfg.hb_gsi = (median * 10.0).round() / 10.0;
    let mut lab = false;
    let lab_msg = if lith.is_empty() {
        String::new()
    } else {
        match st.lab.get(Some(&lith)) {
            Some(row) => {
                lab = true;
                if let Some(u) = row.ucs {
                    cfg.hb_sigci = u;
                }
                if let Some(m) = row.mi {
                    cfg.hb_mi = m;
                }
                if let Some(e) = row.ei {
                    cfg.hb_ei = e;
                }
                fmt(g.m_hb_lab, &[&cfg.hb_lith, &num(cfg.hb_sigci), &num(cfg.hb_mi)])
            }
            None => fmt(g.m_hb_no_lab, &[&cfg.hb_lith]),
        }
    };
    Ok(HbData {
        message: fmt(g.m_hb_data, &[&num(cfg.hb_gsi), &n, &lab_msg]),
        records: n,
        lab,
    })
}

// ─────────────────────────── Block model ───────────────────────────

/// A block model covering the located records when the tab has none (one
/// "Global" domain, geometry settings of the resource panel).
pub fn model_for(results: &[GtResult], cfg: &ProjectConfig) -> Result<BlockModel, String> {
    let pts: Vec<Vec3> = results
        .iter()
        .filter_map(|r| r.at.as_ref().ok().map(|a| a.centre))
        .collect();
    if pts.is_empty() {
        return Err(crate::gti18n::g().m_est_none.into());
    }
    let (lo, hi) = pts.iter().fold(([f64::MAX; 3], [f64::MIN; 3]), |(lo, hi), p| {
        (
            std::array::from_fn(|i| lo[i].min(p[i])),
            std::array::from_fn(|i| hi[i].max(p[i])),
        )
    });
    let b = &cfg.rs.bm;
    let geom = BlockGeometry::covering(lo, hi, b.size, b.rotation, b.pad.max(b.size[0])).map_err(|e| e.to_string())?;
    let mut m = BlockModel::new(geom);
    m.domain_names = vec!["Global".into()];
    let all: [&dyn Region; 1] = [&Everything];
    assign_domains(&mut m, &all, [1, 1, 1]);
    Ok(m)
}

/// Value of a classified record that goes into a block model column.
type Attr = fn(&Classification) -> Option<f64>;

pub struct EstOutcome {
    pub estimated: usize,
    pub total: usize,
    pub no_domain: usize,
    pub columns: Vec<&'static str>,
}

/// Interpolates RMR, GSI and log10 Q into `gt_rmr`, `gt_gsi`, `gt_logq` and
/// derives `gt_class` (from RMR). Density and tonnage are left untouched.
pub fn estimate(model: &mut BlockModel, results: &[GtResult], cfg: &ProjectConfig) -> Result<EstOutcome, String> {
    let gc = &cfg.gt;
    let n_dom = model.domain_names.len();
    // Domain ids of a record: its lithology's domain, or every domain.
    let doms = crate::resources::effective_domains(cfg);
    let mut no_domain = 0;
    let mut hole_ids: HashMap<String, u32> = HashMap::new();
    let mut points: Vec<(Vec3, f64, u32, Vec<u32>, &Classification)> = Vec::new();
    for (k, r) in results.iter().enumerate() {
        let Ok(at) = &r.at else { continue };
        if !r.c.is_classified() {
            continue;
        }
        let hole = match r.kind {
            GtKind::Logging => {
                let next = hole_ids.len() as u32;
                *hole_ids.entry(r.label.to_lowercase()).or_insert(next)
            }
            _ => 1_000_000 + k as u32,
        };
        let ids: Vec<u32> = if gc.md_domains && n_dom > 1 {
            let lith = r.lith.as_deref().map(|l| l.trim().to_lowercase());
            let found = doms.iter().find(|d| {
                lith.as_ref()
                    .is_some_and(|l| d.value_list().iter().any(|v| v.to_lowercase() == *l))
            });
            match found.and_then(|d| model.domain_names.iter().position(|n| *n == d.name)) {
                Some(i) => vec![i as u32 + 1],
                None => {
                    no_domain += 1;
                    continue;
                }
            }
        } else {
            (1..=n_dom as u32).collect()
        };
        points.push((at.centre, r.length, hole, ids, &r.c));
    }
    if points.is_empty() {
        return Err(crate::gti18n::g().m_est_none.into());
    }
    let keep: Vec<(String, Option<Vec<f32>>)> = ["density", "tonnage"]
        .iter()
        .map(|c| (c.to_string(), model.column(c).map(<[f32]>::to_vec)))
        .collect();
    let nb = Neighborhood {
        radii: gc.md_radii,
        orientation: Orientation {
            azimuth: gc.md_az,
            dip: 0.0,
            rake: 0.0,
        },
        min_samples: gc.md_min.round().max(1.0) as usize,
        max_samples: gc.md_max.round().max(1.0) as usize,
        max_per_octant: 0,
        max_per_hole: gc.md_hole.round().max(0.0) as usize,
    };
    let flat = VariogramModel {
        nugget: 1.0,
        structures: Vec::new(),
        orientation: Orientation::default(),
    };
    let attrs: [(&str, Attr); 3] = [("gt_rmr", |c| c.rmr), ("gt_gsi", |c| c.gsi), ("gt_logq", |c| c.log_q())];
    let mut columns = Vec::new();
    let mut estimated = vec![false; model.len()];
    // Confidence: how far and how many records informed each block (taken from the
    // first attribute that has records; every attribute uses the same search).
    for c in ["gt_dist", "gt_n"] {
        model.columns.remove(c);
    }
    let mut confidence = false;
    for (col, value) in attrs {
        let samples: Vec<Sample> = points
            .iter()
            .filter_map(|(xyz, len, hole, ids, c)| value(c).map(|v| (xyz, len, hole, ids, v)))
            .flat_map(|(xyz, len, hole, ids, v)| {
                ids.iter().map(move |&d| Sample {
                    xyz: *xyz,
                    value: v,
                    length: *len,
                    hole: *hole,
                    domain: d,
                })
            })
            .collect();
        model.columns.remove(col);
        if samples.is_empty() {
            continue;
        }
        let var = format!("{col}_w");
        let plan = Plan {
            variable: var.clone(),
            ordinary: false,
            simple: false,
            idw: (gc.md_method == 0).then_some(gc.md_power),
            nearest: true,
            discretization: [1, 1, 1],
            domains: (1..=n_dom as u8)
                .map(|id| DomainPlan {
                    id,
                    model: flat.clone(),
                    passes: vec![nb.clone()],
                    sk_mean: 0.0,
                    density: 0.0,
                })
                .collect(),
        };
        estimate_model(model, &samples, &plan, &|_| {});
        let src = if gc.md_method == 0 {
            format!("{var}_idw")
        } else {
            format!("{var}_nn")
        };
        let values = model.column(&src).map(<[f32]>::to_vec).unwrap_or_default();
        for (i, v) in values.iter().enumerate() {
            estimated[i] |= v.is_finite();
        }
        *model.column_mut(col) = values;
        if !confidence {
            for (target, suffix) in [("gt_dist", "dist"), ("gt_n", "n")] {
                if let Some(v) = model.column(&format!("{var}_{suffix}")).map(<[f32]>::to_vec) {
                    *model.column_mut(target) = v;
                }
            }
            confidence = true;
        }
        for c in blockmodel_engine::estimate::column_names(&var) {
            model.columns.remove(&c);
        }
        columns.push(col);
    }
    // Restore the resource columns the estimator rewrites.
    for (c, v) in keep {
        match v {
            Some(v) => {
                model.columns.insert(c, v);
            }
            None => {
                model.columns.remove(&c);
            }
        }
    }
    if let Some(rmr) = model.column("gt_rmr").map(<[f32]>::to_vec) {
        *model.column_mut("gt_class") = rmr
            .iter()
            .map(|v| {
                if v.is_finite() {
                    rmr_class(*v as f64) as f32
                } else {
                    f32::NAN
                }
            })
            .collect();
        columns.push("gt_class");
    }
    if confidence {
        columns.extend(["gt_dist", "gt_n"]);
    }
    let total = (0..model.len()).filter(|&i| model.domain[i] != 0).count();
    Ok(EstOutcome {
        estimated: estimated.iter().filter(|e| **e).count(),
        total,
        no_domain,
        columns,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cad::mock::MockSink;
    use crate::gti18n::ES;
    use drillhole_engine::model::{Collar, DataSet};

    fn write(dir: &std::path::Path, name: &str, text: &str) -> String {
        let p = dir.join(name);
        std::fs::write(&p, text).unwrap();
        p.display().to_string()
    }

    fn tmp(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("om_gt_{tag}_{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn holes() -> Vec<HoleModel> {
        let ds = DataSet {
            collars: vec![Collar {
                hole_id: "DH1".into(),
                x: 0.0,
                y: 0.0,
                z: 100.0,
                max_depth: 60.0,
                azimuth: Some(0.0),
                dip: Some(-90.0),
            }],
            ..Default::default()
        };
        drillhole_engine::pipeline::build_holes(&ds)
    }

    const LOG: &str = "SONDAJE;DESDE;HASTA;LITOLOGIA;RQD;ESPACIADO;R4;AGUA;Jn;Jr;Ja;Jw;SRF\n\
        DH1;0;10;Andesita;80;0,5;20;seco;9;1,5;1;1;1\n\
        DH1;10;20;Andesita;30;0,1;10;goteo;12;1;2;0,66;2,5\n\
        DH1;20;30;Brecha;;;;;;;;;\n\
        DH9;0;5;Andesita;80;0,5;20;seco;9;1,5;1;1;1\n";
    const LAB: &str = "Litologia,UCS,mi\nAndesita,120,25\n";

    fn loaded(tag: &str) -> (GeotechConfig, GtState) {
        let d = tmp(tag);
        let mut cfg = GeotechConfig {
            logging: write(&d, "log.csv", LOG),
            lab: write(&d, "lab.csv", LAB),
            ..Default::default()
        };
        let (st, lines) = load(&ES, &mut cfg, &mut |_, _, _| None).unwrap();
        assert!(lines.iter().all(|(e, _)| !e), "{lines:?}");
        std::fs::remove_dir_all(d).ok();
        (cfg, st)
    }

    #[test]
    fn load_classify_report_and_draw() {
        let (cfg, mut st) = loaded("a");
        assert_eq!(st.logging.as_ref().unwrap().records.len(), 4);
        assert_eq!(st.lab_rows, 1);
        st.results = classify_all(&st, &holes(), &[], &cfg);
        let r = &st.results;
        // Row 1: 12 (lab UCS 120) + 17 + 10 + 20 + 15 = 74 -> class II.
        assert_eq!(r[0].c.rmr, Some(74.0));
        assert_eq!(r[0].c.class, Some(2));
        assert!(r[1].c.rmr.unwrap() < r[0].c.rmr.unwrap());
        assert!(!r[2].c.is_classified());
        assert!(matches!(r[3].at, Err(LocateError::UnknownHole(_))));
        let lines = report(&ES, r);
        assert!(lines[0].contains("4 registros"), "{lines:?}");
        assert!(lines.iter().any(|l| l.contains("sin ubicar")));
        assert!(lines.iter().any(|l| l.contains("UCS de laboratorio")));
        let mut sink = MockSink::default();
        let n = draw(&mut sink, &ES, &cfg, r, LocalOrigin::default());
        assert_eq!(n, 3, "the unlocated record is not drawn");
        assert_eq!(sink.entities[0].color, Some(class_color(2)));
        assert_eq!(sink.entities[2].color, Some(GRAY), "unclassified in grey");
        assert_eq!(sink.entities[0].tag.get("rmr"), Some("74"));
        // Redraw replaces.
        draw(&mut sink, &ES, &cfg, r, LocalOrigin::default());
        assert_eq!(sink.entities.len(), 3);
        assert_eq!(clear(&mut sink, &ES), 3);
    }

    #[test]
    fn missing_columns_ask_once_and_are_remembered() {
        let d = tmp("b");
        let mut cfg = GeotechConfig {
            logging: write(&d, "log.csv", "ID,P1,P2,RQD\nDH1,0,5,70\n"),
            ..Default::default()
        };
        let mut asked = 0;
        let (st, _) = load(&ES, &mut cfg, &mut |_, h, f| {
            asked += 1;
            assert!(f.contains(&GtField::Hole) && h.contains(&"P1".to_string()));
            Some(
                [
                    ("hole".to_string(), "ID".to_string()),
                    ("from".into(), "P1".into()),
                    ("to".into(), "P2".into()),
                ]
                .into(),
            )
        })
        .unwrap();
        assert_eq!(asked, 1);
        assert_eq!(st.logging.unwrap().records[0].num(GtField::Rqd), Some(70.0));
        // Second load: no question.
        let (st, _) = load(&ES, &mut cfg, &mut |_, _, _| panic!("asked again")).unwrap();
        assert!(st.logging.is_some());
        // Cancelled: error, nothing loaded.
        let mut fresh = GeotechConfig {
            logging: cfg.logging.clone(),
            ..Default::default()
        };
        assert!(load(&ES, &mut fresh, &mut |_, _, _| None).is_err());
        assert!(load(&ES, &mut GeotechConfig::default(), &mut |_, _, _| None).is_err());
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn mapping_by_chainage_with_labels() {
        let d = tmp("c");
        let mut cfg = GeotechConfig {
            mapping: write(&d, "map.csv", "PK;RMR;Q\n10;55;2\n30;35;0,3\n"),
            ..Default::default()
        };
        let (mut st, _) = load(&ES, &mut cfg, &mut |_, _, _| None).unwrap();
        assert!(st.needs_axes());
        let axes = vec![("A".to_string(), vec![[0.0, 0.0, 50.0], [100.0, 0.0, 50.0]])];
        st.results = classify_all(&st, &[], &axes, &cfg);
        assert_eq!(st.results[0].at.as_ref().unwrap().centre, [10.0, 0.0, 50.0]);
        let prims = scene(&st.results, &cfg, LocalOrigin::default());
        assert_eq!(prims.len(), 4, "point + label per record");
        assert!(matches!(&prims[1].geom, Geom::Text { value, .. } if value.contains("RMR 55")));
        assert_eq!(prims[2].color, Some(class_color(4)));
        cfg.color_by = 1;
        assert_eq!(
            scene(&st.results, &cfg, LocalOrigin::default())[0].color,
            Some(30),
            "Q = 2: poor"
        );
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn hoek_brown_outcome_and_data() {
        let (mut cfg, mut st) = loaded("d");
        let out = hoek_brown(&ES, &cfg).unwrap();
        assert!(out.text.contains("mb 1.677"), "{}", out.text);
        assert!(out.svg.starts_with("<svg"));
        cfg.hb_gsi = 120.0;
        assert!(hoek_brown(&ES, &cfg).is_err());
        assert!(hb_from_data(&ES, &mut cfg, &st).is_err(), "nothing classified yet");
        st.results = classify_all(&st, &holes(), &[], &cfg);
        cfg.hb_lith = "andesita".into();
        let msg = hb_from_data(&ES, &mut cfg, &st).unwrap().message;
        assert_eq!((cfg.hb_sigci, cfg.hb_mi), (120.0, 25.0));
        assert!(
            cfg.hb_gsi > 0.0 && cfg.hb_gsi <= 100.0 && msg.contains("Laboratorio"),
            "{msg}"
        );
    }

    /// The geotechnical model drawn as cubes, levels and by class colours.
    #[test]
    fn model_views_as_levels_and_cubes() {
        use crate::resources::{view, ViewInput, GT_VIEW_ATTR};
        let (cfg, mut st) = loaded("v");
        st.results = classify_all(&st, &holes(), &[], &cfg);
        let mut pc = ProjectConfig {
            gt: cfg,
            ..Default::default()
        };
        pc.rs.bm.size = [10.0, 10.0, 10.0];
        let mut model = model_for(&st.results, &pc).unwrap();
        estimate(&mut model, &st.results, &pc).unwrap();
        let with_values = model.column("gt_rmr").unwrap().iter().filter(|v| v.is_finite()).count();
        assert!(with_values > 3);
        let r = crate::rsi18n::r();
        let input = ViewInput {
            axis: None,
            points: &[],
        };
        // Cubes: one mesh per colour, every block with a value is drawn.
        for attr in 0..4 {
            pc.rs.bm.view_attr = GT_VIEW_ATTR + attr;
            pc.rs.bm.view_mode = 3;
            let out = view(&model, &pc, &input, r, LocalOrigin::default()).unwrap();
            assert_eq!(out.cubes, Some((with_values, with_values)), "attribute {attr}");
            assert!(!out.limited && !out.prims.is_empty());
            assert!(out.prims.iter().all(|p| matches!(p.geom, Geom::Mesh { .. })));
            let drawn: usize = out
                .prims
                .iter()
                .map(|p| p.tag.get("blocks").and_then(|b| b.parse::<usize>().ok()).unwrap())
                .sum();
            assert_eq!(drawn, with_values);
        }
        // The class view uses the class colours.
        pc.rs.bm.view_attr = GT_VIEW_ATTR + 3;
        let out = view(&model, &pc, &input, r, LocalOrigin::default()).unwrap();
        assert!(out
            .prims
            .iter()
            .all(|p| (1..=5).any(|k| p.color == Some(class_color(k)))));
        // A limit below the block count keeps an even sample and says so.
        pc.rs.bm.view_limit = 2.0;
        let out = view(&model, &pc, &input, r, LocalOrigin::default()).unwrap();
        assert_eq!(out.cubes, Some((2, with_values)));
        assert!(out.limited);
        // Levels still draw polygons.
        pc.rs.bm.view_mode = 0;
        pc.rs.bm.view_limit = 20_000.0;
        let out = view(&model, &pc, &input, r, LocalOrigin::default()).unwrap();
        assert!(out.polygons > 0 && out.cubes.is_none());
    }

    /// Poor-rock filter, cutting plane and the confidence columns of the model view.
    #[test]
    fn view_filter_cut_and_confidence() {
        use crate::resources::{view, ViewInput, GT_VIEW_ATTR};
        let (cfg, mut st) = loaded("f");
        st.results = classify_all(&st, &holes(), &[], &cfg);
        let mut pc = ProjectConfig {
            gt: cfg,
            ..Default::default()
        };
        pc.rs.bm.size = [10.0, 10.0, 10.0];
        let mut model = model_for(&st.results, &pc).unwrap();
        let out = estimate(&mut model, &st.results, &pc).unwrap();
        assert_eq!(
            out.columns,
            vec!["gt_rmr", "gt_gsi", "gt_logq", "gt_class", "gt_dist", "gt_n"]
        );
        // Confidence: every estimated block says how many records and how far.
        let col = |m: &BlockModel, k: &str| m.column(k).unwrap().to_vec();
        let (rmr, dist, n) = (col(&model, "gt_rmr"), col(&model, "gt_dist"), col(&model, "gt_n"));
        for i in 0..model.len() {
            assert_eq!(rmr[i].is_finite(), n[i].is_finite() && dist[i].is_finite(), "block {i}");
            if rmr[i].is_finite() {
                assert!(n[i] >= 1.0 && dist[i] >= 0.0);
            }
        }
        // A new estimation replaces them (no stale columns).
        let again = estimate(&mut model, &st.results, &pc).unwrap();
        assert_eq!(again.columns.iter().filter(|c| **c == "gt_dist").count(), 1);
        let total = rmr.iter().filter(|v| v.is_finite()).count();
        let r = crate::rsi18n::r();
        let input = ViewInput {
            axis: None,
            points: &[],
        };
        let drawn = |pc: &ProjectConfig| {
            view(&model, pc, &input, r, LocalOrigin::default())
                .unwrap()
                .cubes
                .map_or(0, |c| c.0)
        };
        pc.rs.bm.view_mode = 3;
        pc.rs.bm.view_attr = GT_VIEW_ATTR;
        assert_eq!(drawn(&pc), total);
        // Only below an RMR: strictly fewer (the logging has both good and poor rock), never more.
        let (lo, hi) = rmr
            .iter()
            .filter(|v| v.is_finite())
            .fold((f32::MAX, f32::MIN), |a, v| (a.0.min(*v), a.1.max(*v)));
        assert!(hi - lo > 5.0, "the example must span several RMR values");
        let mid = ((lo + hi) / 2.0) as f64;
        pc.rs.bm.view_filter = 1;
        pc.rs.bm.view_threshold = mid;
        let below = drawn(&pc);
        pc.rs.bm.view_filter = 2;
        let above = drawn(&pc);
        assert!(below > 0 && above > 0 && below + above <= total && below < total && above < total);
        // The class filter: "above class II" shows only classes III to V.
        pc.rs.bm.view_filter = 2;
        pc.rs.bm.view_attr = GT_VIEW_ATTR + 3;
        pc.rs.bm.view_threshold = 2.0;
        let poor = drawn(&pc);
        let class = model.column("gt_class").unwrap();
        assert_eq!(poor, class.iter().filter(|c| c.is_finite() && **c > 2.0).count());
        // The filter never touches resource attributes.
        pc.rs.bm.view_attr = 10;
        assert!(
            view(&model, &pc, &input, r, LocalOrigin::default()).is_err()
                || drawn(&pc) == 0
                || pc.rs.bm.view_filter == 2
        );
        pc.rs.bm.view_attr = GT_VIEW_ATTR;
        pc.rs.bm.view_filter = 0;
        // Cutting plane: a horizontal plane keeps one side; both sides add up to every block.
        let g = model.geometry;
        let zmid = g.origin[2] + g.n[2] as f64 * g.size[2] / 2.0;
        pc.rs.bm.view_cut = true;
        pc.rs.bm.view_cut_point = [g.origin[0], g.origin[1], zmid];
        pc.rs.bm.view_cut_dip = 0.0;
        pc.rs.bm.view_cut_side = 0;
        let up = drawn(&pc);
        pc.rs.bm.view_cut_side = 1;
        let down = drawn(&pc);
        assert!(up > 0 && down > 0 && up + down == total, "{up} + {down} != {total}");
        // A vertical plane east of everything keeps nothing on its far side.
        pc.rs.bm.view_cut_dip = 90.0;
        pc.rs.bm.view_cut_dipdir = 90.0;
        pc.rs.bm.view_cut_point = [g.origin[0] + 1.0e6, g.origin[1], zmid];
        pc.rs.bm.view_cut_side = 0;
        assert_eq!(drawn(&pc), 0);
        pc.rs.bm.view_cut_side = 1;
        assert_eq!(drawn(&pc), total);
        // Confidence attributes are drawn too (far is red).
        pc.rs.bm.view_cut = false;
        for k in [4, 5] {
            pc.rs.bm.view_attr = GT_VIEW_ATTR + k;
            assert_eq!(drawn(&pc), total, "attribute {k}");
        }
    }

    #[test]
    fn block_model_columns() {
        let (cfg, mut st) = loaded("e");
        st.results = classify_all(&st, &holes(), &[], &cfg);
        let mut pc = ProjectConfig {
            gt: cfg,
            ..Default::default()
        };
        pc.rs.bm.size = [10.0, 10.0, 10.0];
        let mut model = model_for(&st.results, &pc).unwrap();
        model.columns.insert("tonnage".into(), vec![7.0; model.len()]);
        let out = estimate(&mut model, &st.results, &pc).unwrap();
        assert!(out.estimated > 0 && out.estimated <= out.total);
        assert_eq!(
            out.columns,
            vec!["gt_rmr", "gt_gsi", "gt_logq", "gt_class", "gt_dist", "gt_n"]
        );
        assert!(
            model.column("tonnage").unwrap().iter().all(|t| *t == 7.0),
            "resource columns untouched"
        );
        assert!(model.column("density").is_none());
        assert!(
            !model.columns.keys().any(|k| k.contains("_w_")),
            "{:?}",
            model.columns.keys()
        );
        let rmr = model.column("gt_rmr").unwrap();
        let class = model.column("gt_class").unwrap();
        for i in 0..model.len() {
            if rmr[i].is_finite() {
                assert!((30.0..=80.0).contains(&rmr[i]), "{}", rmr[i]);
                assert_eq!(class[i] as u8, rmr_class(rmr[i] as f64));
            }
        }
        // Honouring domains: no domain lists the lithology -> nothing to estimate.
        pc.gt.md_domains = true;
        model.domain_names = vec!["A".into(), "B".into()];
        pc.rs.domains = vec![
            crate::rsconfig::DomainCfg {
                name: "A".into(),
                values: "Riolita".into(),
                ..Default::default()
            },
            crate::rsconfig::DomainCfg {
                name: "B".into(),
                values: "Otra".into(),
                ..Default::default()
            },
        ];
        assert!(estimate(&mut model, &st.results, &pc).is_err());
    }
}
