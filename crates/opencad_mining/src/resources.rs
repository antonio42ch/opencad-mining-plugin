//! Resource estimation commands: composites, variography, block model.
//! Logic is pure or goes through `CadSink`, so it is tested without a CAD.

use std::collections::{BTreeMap, HashMap};

use blockmodel_engine::estimate::{estimate_model, DomainPlan, Plan};
use blockmodel_engine::region::{assign_domains, And, Everything, Region, SolidsRegion, SurfaceSide};
use blockmodel_engine::report::{self, Axis, Filter};
use blockmodel_engine::{BlockGeometry, BlockModel};
use drillhole_engine::color::{grade_color, ColorScheme, GRAY};
use drillhole_engine::pipeline::{build_holes, HoleModel};
use drillhole_engine::DataSet;
use geostat_engine::bridge::{composites_from_holes, contact_points, Composite3d};
use geostat_engine::composite::cap_values;
use geostat_engine::crossval::{cross_validate, CvStats};
use geostat_engine::decluster::{decluster, weighted_mean};
use geostat_engine::fit::{fit, FitConfig};
use geostat_engine::kriging::Method;
use geostat_engine::model::{Kind, VariogramModel};
use geostat_engine::stats::{histogram, summarize, Summary};
use geostat_engine::surface::{krige_surface, GridSpec, GridSurface};
use geostat_engine::vario::{experimental, ExpVariogram, VarioParams};
use geostat_engine::Sample;
use mining_core::{LocalOrigin, Vec3};
use solids_engine::cut::MeshData;

use crate::cad::{CadSink, Geom, LayerDef, Prim};
use crate::charts::{render, Chart, Series, Style};
use crate::config::ProjectConfig;
use crate::i18n::fmt;
use crate::rsconfig::{DomainCfg, ModelCfg, KINDS};
use crate::rsi18n::RsStrings;
use crate::xdata::{Class, Tag};

pub const LAYER_COMP_PREFIX: &str = "MIN_RS_COMP_";
pub const LAYER_CONTACT: &str = "MIN_RS_CONTACTO";
pub const LAYER_VIEW: &str = "MIN_RS_VISTA";

/// Everything computed in the session for one drawing tab.
#[derive(Default, Clone)]
pub struct RsState {
    pub composites: Vec<Composite3d>,
    pub weights: Vec<f64>,
    pub exp: Vec<(String, ExpVariogram)>,
    /// Contact surfaces by name.
    pub surfaces: Vec<(String, GridSurface)>,
    pub model: Option<BlockModel>,
    /// Variable name used for the block model columns.
    pub variable: String,
    /// Domain names in force when the composites were made: composite
    /// domain ids are positions in this list.
    pub domain_names: Vec<String>,
    /// Domain index the cached experimental variograms belong to.
    pub exp_domain: Option<usize>,
}

impl RsState {
    /// Cached variograms, only when they belong to domain `idx`.
    pub fn exp_for(&self, idx: usize) -> Option<&Vec<(String, ExpVariogram)>> {
        (self.exp_domain == Some(idx) && !self.exp.is_empty()).then_some(&self.exp)
    }

    /// New composites invalidate everything derived from the old ones.
    pub fn set_composites(
        &mut self,
        comps: Vec<Composite3d>,
        weights: Vec<f64>,
        domain_names: Vec<String>,
        variable: String,
    ) {
        *self = RsState {
            composites: comps,
            weights,
            domain_names,
            variable,
            surfaces: std::mem::take(&mut self.surfaces),
            ..Default::default()
        };
    }
}

/// The composites' domain ids are positions in the domain list used when
/// compositing; refuse to use them after the table was edited.
pub fn check_domains(composited: &[String], cfg: &ProjectConfig, r: &RsStrings) -> Result<(), String> {
    let now: Vec<String> = effective_domains(cfg).into_iter().map(|d| d.name).collect();
    if composited == now.as_slice() {
        Ok(())
    } else {
        Err(fmt(r.m_domains_changed, &[&composited.join(", ")]))
    }
}

/// Domains in use: the configured ones, or one "Global" domain.
pub fn effective_domains(cfg: &ProjectConfig) -> Vec<DomainCfg> {
    if cfg.rs.domains.is_empty() {
        vec![DomainCfg {
            name: "Global".into(),
            ..Default::default()
        }]
    } else {
        cfg.rs.domains.clone()
    }
}

/// Column-name-safe variable name (`Cu %` -> `cu_`).
pub fn var_name(attr: &str) -> String {
    let v: String = attr
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    if v.is_empty() {
        "grade".into()
    } else {
        v
    }
}

pub fn num(v: f64) -> String {
    if !v.is_finite() {
        return "-".into();
    }
    let s = format!("{v:.3}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

pub fn samples_of(comps: &[Composite3d]) -> Vec<Sample> {
    comps.iter().map(|c| c.sample()).collect()
}

#[derive(Debug, Clone, PartialEq)]
pub struct DomainStats {
    pub name: String,
    pub summary: Summary,
    pub declustered_mean: f64,
    pub cell: f64,
    pub capped: usize,
}

pub struct CompOutcome {
    pub composites: Vec<Composite3d>,
    pub weights: Vec<f64>,
    pub stats: Vec<DomainStats>,
    pub holes: usize,
    pub length: f64,
    pub capped: usize,
}

/// Composites, caps and declusters per domain.
pub fn composite(data: &DataSet, cfg: &ProjectConfig, r: &RsStrings) -> Result<CompOutcome, String> {
    if cfg.dh.assay_attr.is_empty() {
        return Err(r.m_no_attr.to_string());
    }
    let holes = build_holes(data);
    let doms = effective_domains(cfg);
    let rules = if cfg.rs.domains.is_empty() {
        Vec::new()
    } else {
        cfg.rs.rules()
    };
    let geo_attr = if cfg.rs.domains.is_empty() {
        ""
    } else {
        cfg.dh.geo_attr.as_str()
    };
    let mut comps = composites_from_holes(&holes, &cfg.dh.assay_attr, geo_attr, &rules, &cfg.rs.composite_config());
    // Top-cut per domain.
    let mut capped = 0;
    let mut stats = Vec::new();
    let mut weights = vec![1.0; comps.len()];
    for (i, d) in doms.iter().enumerate() {
        let id = i as u32 + 1;
        let idx: Vec<usize> = (0..comps.len()).filter(|&k| comps[k].domain == id).collect();
        let mut vals: Vec<f64> = idx.iter().map(|&k| comps[k].value).collect();
        let n_cap = if d.cap > 0.0 { cap_values(&mut vals, d.cap) } else { 0 };
        capped += n_cap;
        for (&k, v) in idx.iter().zip(&vals) {
            comps[k].value = *v;
        }
        let samples: Vec<Sample> = idx.iter().map(|&k| comps[k].sample()).collect();
        let dc = decluster(&samples, &cfg.rs.decluster_sizes(), cfg.rs.dc_zratio, true);
        for (&k, w) in idx.iter().zip(&dc.weights) {
            weights[k] = *w;
        }
        let lens: Vec<f64> = samples.iter().map(|s| s.length * 1.0).collect();
        let wt: Vec<f64> = dc.weights.iter().zip(&lens).map(|(w, l)| w * l).collect();
        let summary = summarize(&vals, Some(&wt));
        let raw = summarize(&vals, None);
        stats.push(DomainStats {
            name: d.name.clone(),
            summary: Summary {
                mean: raw.mean,
                variance: raw.variance,
                std_dev: raw.std_dev,
                cv: raw.cv,
                ..summary
            },
            declustered_mean: if samples.is_empty() {
                f64::NAN
            } else {
                weighted_mean(&samples, &dc.weights)
            },
            cell: dc.best_size,
            capped: n_cap,
        });
    }
    let length = comps.iter().map(|c| c.length()).sum();
    Ok(CompOutcome {
        composites: comps,
        weights,
        stats,
        holes: holes.len(),
        length,
        capped,
    })
}

pub fn stats_lines(r: &RsStrings, stats: &[DomainStats]) -> Vec<String> {
    stats
        .iter()
        .map(|s| {
            fmt(
                r.m_stats,
                &[
                    &s.name,
                    &s.summary.n,
                    &num(s.summary.min),
                    &num(s.summary.max),
                    &num(s.summary.mean),
                    &num(s.summary.cv),
                    &num(s.declustered_mean),
                    &num(s.cell),
                ],
            )
        })
        .collect()
}

/// Histogram of the composites, one series per domain with shared bins.
pub fn histogram_chart(
    r: &RsStrings,
    comps: &[Composite3d],
    weights: &[f64],
    domains: &[DomainCfg],
    unit: &str,
) -> String {
    let vals: Vec<f64> = comps.iter().map(|c| c.value).collect();
    let (lo, hi) = vals
        .iter()
        .fold((f64::MAX, f64::MIN), |(a, b), v| (a.min(*v), b.max(*v)));
    let series: Vec<Series> = domains
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let idx: Vec<usize> = (0..comps.len()).filter(|&k| comps[k].domain == i as u32 + 1).collect();
            let v: Vec<f64> = idx.iter().map(|&k| vals[k]).collect();
            let w: Vec<f64> = idx.iter().map(|&k| weights[k] * comps[k].length()).collect();
            let h = histogram(&v, Some(&w), 24, lo.min(hi), hi.max(lo + 1e-9));
            Series::new(
                d.name.clone(),
                h.freq
                    .iter()
                    .enumerate()
                    .map(|(b, f)| (0.5 * (h.edges[b] + h.edges[b + 1]), *f))
                    .collect(),
                Style::Bars,
                i,
            )
        })
        .collect();
    render(&Chart {
        title: &fmt(r.ch_hist, &[&unit]),
        x_label: unit,
        y_label: "%",
        series: &series,
        width: 320.0,
        height: 220.0,
        y_from_zero: true,
    })
}

/// Composites as CSV (also the session cache: see [`parse_composites_csv`]).
pub fn composites_csv(comps: &[Composite3d], domains: &[DomainCfg]) -> String {
    let mut s = String::from("hole,from,to,x,y,z,value,domain,x_from,y_from,z_from,x_to,y_to,z_to\n");
    for c in comps {
        let dn = domains
            .get((c.domain as usize).wrapping_sub(1))
            .map_or("", |d| d.name.as_str());
        let (f, t) = (c.from_xyz, c.to_xyz);
        s.push_str(&format!(
            "{},{},{},{:.4},{:.4},{:.4},{},{},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4}\n",
            c.hole.replace(',', " "),
            c.from,
            c.to,
            c.xyz[0],
            c.xyz[1],
            c.xyz[2],
            c.value,
            dn.replace(',', " "),
            f[0],
            f[1],
            f[2],
            t[0],
            t[1],
            t[2]
        ));
    }
    s
}

/// Reads back [`composites_csv`]. Domain names are matched against the
/// current domain list (an unknown name means the table changed). Files of
/// v0.4 without the end coordinates use the centre for both ends.
pub fn parse_composites_csv(text: &str, domains: &[DomainCfg]) -> Result<Vec<Composite3d>, String> {
    let mut lines = text.lines();
    let head: Vec<&str> = lines.next().ok_or("empty file")?.split(',').map(str::trim).collect();
    let col = |n: &str| head.iter().position(|h| *h == n);
    let need = |n: &str| col(n).ok_or_else(|| format!("missing column {n}"));
    let (ih, ifr, ito, ix, iy, iz, iv, id) = (
        need("hole")?,
        need("from")?,
        need("to")?,
        need("x")?,
        need("y")?,
        need("z")?,
        need("value")?,
        need("domain")?,
    );
    let ends = ["x_from", "y_from", "z_from", "x_to", "y_to", "z_to"].map(col);
    let mut holes: HashMap<String, u32> = HashMap::new();
    let mut out = Vec::new();
    for (n, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split(',').collect();
        let get = |i: usize| {
            f.get(i)
                .map(|s| s.trim())
                .ok_or_else(|| format!("line {}: too few fields", n + 2))
        };
        let num = |i: usize| -> Result<f64, String> {
            get(i)?
                .parse::<f64>()
                .map_err(|_| format!("line {}: bad number", n + 2))
        };
        let dname = get(id)?;
        let domain = domains
            .iter()
            .position(|d| d.name.trim() == dname)
            .ok_or_else(|| format!("unknown domain {dname}"))? as u32
            + 1;
        let next = holes.len() as u32;
        let hole = get(ih)?.to_string();
        let hole_index = *holes.entry(hole.clone()).or_insert(next);
        let xyz = [num(ix)?, num(iy)?, num(iz)?];
        let end = |k: usize| -> Result<Option<f64>, String> { ends[k].map(num).transpose() };
        let from_xyz = match (end(0)?, end(1)?, end(2)?) {
            (Some(a), Some(b), Some(c)) => [a, b, c],
            _ => xyz,
        };
        let to_xyz = match (end(3)?, end(4)?, end(5)?) {
            (Some(a), Some(b), Some(c)) => [a, b, c],
            _ => xyz,
        };
        out.push(Composite3d {
            hole,
            hole_index,
            from: num(ifr)?,
            to: num(ito)?,
            from_xyz,
            to_xyz,
            xyz,
            value: num(iv)?,
            domain,
        });
    }
    Ok(out)
}

/// Declustering weights per domain (as computed when compositing).
pub fn decluster_weights(comps: &[Composite3d], cfg: &ProjectConfig) -> Vec<f64> {
    let mut weights = vec![1.0; comps.len()];
    for id in 1..=effective_domains(cfg).len() as u32 {
        let idx: Vec<usize> = (0..comps.len()).filter(|&k| comps[k].domain == id).collect();
        let samples: Vec<Sample> = idx.iter().map(|&k| comps[k].sample()).collect();
        let dc = decluster(&samples, &cfg.rs.decluster_sizes(), cfg.rs.dc_zratio, true);
        for (&k, w) in idx.iter().zip(&dc.weights) {
            weights[k] = *w;
        }
    }
    weights
}

/// What the cached composites were made with.
pub fn composite_stamp(cfg: &ProjectConfig) -> crate::rsconfig::CompositeStamp {
    crate::rsconfig::CompositeStamp {
        variable: var_name(&cfg.dh.assay_attr),
        domains: effective_domains(cfg).into_iter().map(|d| d.name).collect(),
    }
}

/// Draws the composites as coloured lines (replacing the previous drawing).
pub fn draw_composites(
    sink: &mut dyn CadSink,
    r: &RsStrings,
    cfg: &ProjectConfig,
    comps: &[Composite3d],
    origin: LocalOrigin,
) -> usize {
    if comps.is_empty() {
        sink.error(r.m_no_composites);
        return 0;
    }
    let d = &cfg.dh;
    let layer = format!("{LAYER_COMP_PREFIX}{}", var_name(&d.assay_attr).to_uppercase());
    let global = comps.iter().fold(None, |a: Option<(f64, f64)>, c| {
        Some(a.map_or((c.value, c.value), |(lo, hi)| (lo.min(c.value), hi.max(c.value))))
    });
    let per_hole: HashMap<u32, (f64, f64)> = comps.iter().fold(HashMap::new(), |mut m, c| {
        let e = m.entry(c.hole_index).or_insert((c.value, c.value));
        *e = (e.0.min(c.value), e.1.max(c.value));
        m
    });
    let custom = d.custom_ranges();
    let scheme = d.scheme();
    let prims: Vec<Prim> = comps
        .iter()
        .map(|c| {
            let (lo, hi) = if d.scope() == drillhole_engine::pipeline::RangeScope::Global {
                global
            } else {
                per_hole.get(&c.hole_index).copied()
            }
            .unwrap_or((0.0, 1.0));
            let color = if scheme == ColorScheme::Custom && custom.is_empty() {
                GRAY
            } else {
                grade_color(scheme, c.value, lo, hi, &custom)
            };
            Prim {
                geom: Geom::Line(origin.to_local(c.from_xyz), origin.to_local(c.to_xyz)),
                layer: layer.clone(),
                color: Some(color),
                tag: Tag::new(Class::Comp)
                    .with("hole", &c.hole)
                    .with("from", c.from)
                    .with("to", c.to)
                    .with("value", format!("{:.4}", c.value))
                    .with("domain", c.domain),
            }
        })
        .collect();
    sink.begin("MIN_RS_DRAWCOMP");
    sink.ensure_layers(&[LayerDef::new(layer, 7)]);
    sink.remove_where(&|t: &Tag| t.class == Class::Comp);
    let n = sink.add(prims);
    sink.end();
    sink.info(&fmt(r.m_comp_drawn, &[&n]));
    n
}

/// One contact surface from the first contact (above -> below values) of
/// each hole, kriged on a grid over the collars.
fn surface_between(
    holes: &[HoleModel],
    cfg: &ProjectConfig,
    above: &str,
    below: &str,
    r: &RsStrings,
) -> Result<(GridSurface, usize), String> {
    let c = &cfg.rs.contact;
    let split = |s: &str| -> Vec<String> {
        s.split(';')
            .map(|x| x.trim().to_string())
            .filter(|x| !x.is_empty())
            .collect()
    };
    let (above, below) = (split(above), split(below));
    if above.is_empty() || below.is_empty() || cfg.dh.geo_attr.is_empty() {
        return Err(r.m_contact_cfg.to_string());
    }
    let pts = contact_points(holes, &cfg.dh.geo_attr, &above, &below);
    if pts.len() < 3 {
        return Err(fmt(r.m_contact_few, &[&pts.len()]));
    }
    let (lo, hi) = holes.iter().fold(([f64::MAX; 2], [f64::MIN; 2]), |(lo, hi), h| {
        (
            [lo[0].min(h.collar.x), lo[1].min(h.collar.y)],
            [hi[0].max(h.collar.x), hi[1].max(h.collar.y)],
        )
    });
    let step = c.step.max(0.1);
    let (x0, y0) = ((lo[0] / step).floor() * step, (lo[1] / step).floor() * step);
    let (nx, ny) = (
        (((hi[0] - x0) / step).ceil() as usize + 1).clamp(2, 2000),
        (((hi[1] - y0) / step).ceil() as usize + 1).clamp(2, 2000),
    );
    let s = krige_surface(
        &pts,
        &GridSpec {
            x0,
            y0,
            dx: step,
            dy: step,
            nx,
            ny,
        },
        &c.params(),
    );
    Ok((s, pts.len()))
}

/// Every contact surface of the table (shared kriging parameters).
pub fn contact_surfaces(
    holes: &[HoleModel],
    cfg: &ProjectConfig,
    r: &RsStrings,
) -> Result<Vec<(String, GridSurface, usize)>, String> {
    let rows = cfg.rs.contact_rows();
    if rows.is_empty() {
        return Err(r.m_contact_cfg.to_string());
    }
    rows.iter()
        .map(|c| {
            surface_between(holes, cfg, &c.above, &c.below, r)
                .map(|(s, n)| (c.name.clone(), s, n))
                .map_err(|e| format!("{}: {e}", c.name))
        })
        .collect()
}

/// First contact surface (single-contact projects).
pub fn contact_surface(
    holes: &[HoleModel],
    cfg: &ProjectConfig,
    r: &RsStrings,
) -> Result<(GridSurface, usize), String> {
    let row = cfg
        .rs
        .contact_rows()
        .into_iter()
        .next()
        .ok_or_else(|| r.m_contact_cfg.to_string())?;
    surface_between(holes, cfg, &row.above, &row.below, r)
}

/// Mesh of the surface (quads between finite nodes).
pub fn surface_mesh(s: &GridSurface, origin: LocalOrigin) -> (Vec<Vec3>, Vec<Vec<usize>>) {
    let mut idx = vec![usize::MAX; s.nx * s.ny];
    let mut v = Vec::new();
    for j in 0..s.ny {
        for i in 0..s.nx {
            if s.z[j * s.nx + i].is_finite() {
                idx[j * s.nx + i] = v.len();
                v.push(origin.to_local(s.node(i, j)));
            }
        }
    }
    let mut f = Vec::new();
    for j in 0..s.ny - 1 {
        for i in 0..s.nx - 1 {
            let q = [
                idx[j * s.nx + i],
                idx[j * s.nx + i + 1],
                idx[(j + 1) * s.nx + i + 1],
                idx[(j + 1) * s.nx + i],
            ];
            if q.iter().all(|&k| k != usize::MAX) {
                f.push(q.to_vec());
            }
        }
    }
    (v, f)
}

/// Draws every surface (one mesh each), replacing the previous ones.
pub fn draw_surfaces(
    sink: &mut dyn CadSink,
    r: &RsStrings,
    surfaces: &[(String, GridSurface, usize)],
    origin: LocalOrigin,
) {
    sink.begin("MIN_RS_CONTACT");
    sink.ensure_layers(&[LayerDef::new(LAYER_CONTACT, 30)]);
    sink.remove_where(&|t: &Tag| t.class == Class::Surface);
    for (name, s, npoints) in surfaces {
        let (vertices, faces) = surface_mesh(s, origin);
        if !faces.is_empty() {
            sink.add(vec![Prim {
                geom: Geom::Mesh { vertices, faces },
                layer: LAYER_CONTACT.into(),
                color: None,
                tag: Tag::new(Class::Surface)
                    .with("name", name)
                    .with("points", npoints)
                    .with("nx", s.nx)
                    .with("ny", s.ny)
                    .with("step", s.dx),
            }]);
        }
        let estimated = s.z.iter().filter(|z| z.is_finite()).count();
        sink.info(&format!(
            "{name}: {}",
            fmt(r.m_contact_done, &[npoints, &s.nx, &s.ny, &estimated])
        ));
    }
    sink.end();
}

pub fn draw_surface(sink: &mut dyn CadSink, r: &RsStrings, s: &GridSurface, npoints: usize, origin: LocalOrigin) {
    draw_surfaces(sink, r, &[("C1".into(), s.clone(), npoints)], origin);
}

#[cfg(test)]
pub(crate) mod fixtures {
    use super::*;
    use drillhole_engine::{Assay, Collar, GeoLog, Survey};
    use std::collections::BTreeMap;

    /// A 7 x 7 grid of vertical holes, oxide above sulphide with a gently
    /// dipping contact; oxide ~1 g/t, sulphide ~5 g/t.
    pub fn dataset() -> DataSet {
        let mut ds = DataSet::default();
        for i in 0..7 {
            for j in 0..7 {
                let id = format!("H{i}{j}");
                let (x, y) = (500_000.0 + i as f64 * 40.0, 8_500_000.0 + j as f64 * 40.0);
                ds.collars.push(Collar {
                    hole_id: id.clone(),
                    x,
                    y,
                    z: 4_000.0,
                    max_depth: 60.0,
                    azimuth: None,
                    dip: None,
                });
                ds.surveys.push(Survey {
                    hole_id: id.clone(),
                    depth: 0.0,
                    azimuth: 0.0,
                    dip: -90.0,
                });
                let contact = 20.0 + i as f64; // metres below the collar
                let geo = |f, t, v: &str| GeoLog {
                    hole_id: id.clone(),
                    from: f,
                    to: t,
                    attributes: BTreeMap::from([("rock".to_string(), v.to_string())]),
                };
                ds.geo.push(geo(0.0, contact, "OXIDO"));
                ds.geo.push(geo(contact, 60.0, "SULFURO"));
                for k in 0..60 {
                    // Smooth spatial trends (continuity at tens of metres) around each domain's mean.
                    let wave = (x / 70.0).sin() * (y / 60.0).cos();
                    let v = if (k as f64) < contact {
                        1.2 + 0.5 * wave
                    } else {
                        5.5 + 0.8 * wave + 0.3 * (k as f64 / 9.0).sin()
                    };
                    ds.assays.push(Assay {
                        hole_id: id.clone(),
                        from: k as f64,
                        to: k as f64 + 1.0,
                        attributes: BTreeMap::from([("au".to_string(), Some(v))]),
                    });
                }
            }
        }
        ds
    }

    pub fn cfg() -> ProjectConfig {
        let mut c = ProjectConfig::default();
        c.dh.assay_attr = "au".into();
        c.dh.geo_attr = "rock".into();
        c.rs.domains = vec![
            DomainCfg {
                name: "Óxidos".into(),
                values: "OXIDO".into(),
                density: 2.4,
                cap: 0.0,
                // v0.4 form on purpose: the legacy single-surface side still works.
                side: 1,
                surface: String::new(),
            },
            DomainCfg {
                name: "Sulfuros".into(),
                values: "SULFURO".into(),
                density: 2.8,
                cap: 0.0,
                side: 2,
                surface: String::new(),
            },
        ];
        c.rs.contact.above = "OXIDO".into();
        c.rs.contact.below = "SULFURO".into();
        c.rs.contact.step = 20.0;
        c.rs.contact.range = 200.0;
        c.rs.contact.radius = 150.0;
        c.rs.contact.sill = 20.0;
        c.rs.dc_min = 20.0;
        c.rs.dc_max = 80.0;
        c.rs.dc_steps = 4.0;
        c.rs.comp_length = 4.0;
        c
    }
}

#[cfg(test)]
mod pipeline_tests {
    use super::fixtures::*;
    use super::*;
    use crate::cad::mock::MockSink;
    use crate::i18n::Lang;
    use crate::rsconfig::StructCfg;
    use crate::rsi18n::for_lang;

    fn model_cfg(sill: f64) -> ModelCfg {
        ModelCfg {
            nugget: 0.05 * sill,
            structures: [
                StructCfg {
                    kind: 0,
                    sill,
                    ranges: [150.0, 150.0, 40.0],
                },
                StructCfg::default(),
            ],
            azimuth: 0.0,
            dip: 0.0,
            rake: 0.0,
        }
    }

    fn setup() -> (ProjectConfig, CompOutcome, GridSurface) {
        let r = for_lang(Lang::Es);
        let mut c = cfg();
        c.rs.vario.lag = 40.0;
        c.rs.vario.nlags = 5.0;
        c.rs.vario.az_tol = 90.0;
        c.rs.vario.dip_tol = 90.0;
        c.rs.vario.models.insert("Óxidos".into(), model_cfg(0.5));
        c.rs.vario.models.insert("Sulfuros".into(), model_cfg(2.0));
        c.rs.bm.radii = [160.0, 160.0, 40.0];
        c.rs.bm.pass2 = 2.0;
        let out = composite(&dataset(), &c, r).unwrap();
        let (surf, _) = contact_surface(&build_holes(&dataset()), &c, r).unwrap();
        (c, out, surf)
    }

    #[test]
    fn variography_fit_and_crossvalidation_on_one_domain() {
        let r = for_lang(Lang::En);
        let (mut c, out, _) = setup();
        let sulf: Vec<Sample> = samples_of(&out.composites)
            .into_iter()
            .filter(|s| s.domain == 2)
            .collect();
        let exps = compute_variograms(&sulf, &c);
        assert_eq!(exps.len(), 4);
        assert!(
            exps.iter().all(|(_, e)| !e.points.is_empty()),
            "every direction has pairs"
        );
        assert_eq!(direction_angles([0.0, 1.0, 0.0]), (0.0, 0.0));
        let (az, dip) = direction_angles([0.0, 0.0, -1.0]);
        assert!((az - 0.0).abs() < 1e-9 && (dip - 90.0).abs() < 1e-9);
        let svg = variogram_chart(r, &exps, &c.rs.vario.work.model(), "Sulfuros");
        assert!(svg.starts_with("<svg") && !svg.contains("NaN"));
        // Auto-fit starts even without structures and returns a usable model.
        c.rs.vario.work = ModelCfg {
            structures: [
                StructCfg {
                    sill: 0.0,
                    ..Default::default()
                },
                StructCfg::default(),
            ],
            ..ModelCfg::default()
        };
        let f = fit_working_model(&exps, &sulf, &c);
        assert!(f.score.is_finite() && f.model.total_sill() > 0.0 && !f.model.structures.is_empty());
        c.rs.vario.work = ModelCfg::from_model(&f.model);
        let (stats, line) = run_crossval(&sulf, &c, "Sulfuros", r).unwrap();
        assert!(
            stats.n > 20 && stats.mse.is_finite() && stats.correlation.is_finite(),
            "{stats:?}"
        );
        assert!(line.starts_with("Cross-validation «Sulfuros»"));
        // Without a model the validation is refused with the localised reason.
        c.rs.vario.work = ModelCfg {
            nugget: 0.0,
            structures: [
                StructCfg {
                    sill: 0.0,
                    ..Default::default()
                },
                StructCfg::default(),
            ],
            ..ModelCfg::default()
        };
        assert!(run_crossval(&sulf, &c, "Sulfuros", r)
            .unwrap_err()
            .contains("no saved variogram model"));
        // A pure-nugget model is valid.
        c.rs.vario.work.nugget = 1.0;
        assert!(run_crossval(&sulf, &c, "Sulfuros", r).is_ok());
    }

    #[test]
    fn full_pipeline_estimates_each_domain_from_its_own_samples() {
        let r = for_lang(Lang::Es);
        let (c, out, surf) = setup();
        let doms = effective_domains(&c);
        let regions = build_regions(&doms, &DomainMeshes::new(), &[("C1".to_string(), surf.clone())], r).unwrap();
        let mut model = create_model(&out.composites, &c, &regions).unwrap();
        assert!(model.assigned() > 100);
        assert_eq!(model.domain_names, vec!["Óxidos", "Sulfuros"]);
        // The contact dips ~6 m across the field: oxide blocks sit above sulphide blocks.
        let zs = |d: u8| -> Vec<f64> {
            (0..model.len())
                .filter(|&i| model.domain[i] == d)
                .map(|i| model.geometry.center(i)[2])
                .collect()
        };
        let (ox, su) = (zs(1), zs(2));
        assert!(ox.iter().sum::<f64>() / ox.len() as f64 > su.iter().sum::<f64>() / su.len() as f64);
        let rep = run_estimation(&mut model, &out.composites, &out.weights, &c, r).unwrap();
        assert_eq!(rep.domains.len(), 2);
        let ok = model.column("au_ok").unwrap();
        for (i, (v, d)) in ok.iter().zip(&model.domain).enumerate() {
            match d {
                1 if v.is_finite() => assert!(*v < 2.0, "oxide block {i} polluted: {v}"),
                2 if v.is_finite() => assert!(*v > 4.0, "sulphide block {i} polluted: {v}"),
                _ => {}
            }
        }
        assert!(rep.domains.iter().all(|d| d.estimated > 0 && d.estimated <= d.blocks));
        assert!(estimation_lines(r, &model, &rep)[0].starts_with("Óxidos:"));
        // Tonnage uses each domain's density.
        let t = model.column("tonnage").unwrap();
        let i2 = (0..model.len())
            .find(|&i| model.domain[i] == 2 && model.fraction[i] == 1.0)
            .unwrap();
        assert!((t[i2] as f64 - 10.0 * 10.0 * 12.0 * 2.8).abs() < 1e-1);
        let lines = resource_lines(r, &model, "au", 0.0, "g/t");
        assert_eq!(lines.len(), 4);
        assert!(lines[3].starts_with("Total:"));
        // Validation text and charts for every chart type.
        for chart in 0..3 {
            let mut cc = c.clone();
            cc.rs.bm.chart = chart;
            let (vl, svg) = validate(r, &model, &out.composites, &out.weights, &cc, "demo").unwrap();
            assert_eq!(vl.len(), 3);
            assert!(svg.starts_with("<svg") && !svg.contains("NaN"), "chart {chart}");
        }
    }

    #[test]
    fn edited_domain_table_is_detected_and_new_composites_reset_derived_state() {
        let r = for_lang(Lang::Es);
        let (mut c, out, _) = setup();
        let names: Vec<String> = effective_domains(&c).into_iter().map(|d| d.name).collect();
        assert!(check_domains(&names, &c, r).is_ok());
        c.rs.domains.swap(0, 1);
        let err = check_domains(&names, &c, r).unwrap_err();
        assert!(err.contains("Óxidos, Sulfuros"), "{err}");
        // Cached variograms belong to one domain only.
        let mut st = RsState::default();
        st.set_composites(out.composites.clone(), out.weights.clone(), names.clone(), "au".into());
        st.exp = vec![(
            "λ1".into(),
            compute_variograms(&samples_of(&out.composites), &c).remove(0).1,
        )];
        st.exp_domain = Some(0);
        assert!(st.exp_for(0).is_some() && st.exp_for(1).is_none());
        st.set_composites(out.composites, out.weights, names, "au".into());
        assert!(st.exp_for(0).is_none() && st.model.is_none());
    }

    #[test]
    fn region_errors_are_explicit() {
        let r = for_lang(Lang::En);
        let (c, _, _) = setup();
        let mut doms = effective_domains(&c);
        doms[0].side = 0; // no solid, no surface
        let err = build_regions(&doms, &DomainMeshes::new(), &[], r).err().unwrap();
        assert!(err.contains("Óxidos"), "{err}");
        // Missing surface for a domain that needs it.
        doms[0].side = 1;
        assert_eq!(
            build_regions(&doms, &DomainMeshes::new(), &[], r).err().unwrap(),
            r.m_contact_cfg
        );
        // One domain with nothing covers everything.
        let one = vec![DomainCfg {
            name: "Global".into(),
            ..Default::default()
        }];
        assert!(build_regions(&one, &DomainMeshes::new(), &[], r).is_ok());
        // Estimation without a saved model names the domain.
        let (mut c2, out, surf) = setup();
        c2.rs.vario.models.remove("Sulfuros");
        let regions = build_regions(
            &effective_domains(&c2),
            &DomainMeshes::new(),
            &[("C1".to_string(), surf.clone())],
            r,
        )
        .unwrap();
        let mut model = create_model(&out.composites, &c2, &regions).unwrap();
        assert!(run_estimation(&mut model, &out.composites, &out.weights, &c2, r)
            .unwrap_err()
            .contains("Sulfuros"));
    }

    fn ok_col(m: &BlockModel) -> &[f32] {
        m.column("au_ok").unwrap()
    }

    #[test]
    fn views_levels_sections_cubes_and_replacement() {
        let r = for_lang(Lang::Es);
        let (mut c, out, surf) = setup();
        let regions = build_regions(
            &effective_domains(&c),
            &DomainMeshes::new(),
            &[("C1".to_string(), surf.clone())],
            r,
        )
        .unwrap();
        let mut model = create_model(&out.composites, &c, &regions).unwrap();
        run_estimation(&mut model, &out.composites, &out.weights, &c, r).unwrap();
        let none = ViewInput {
            axis: None,
            points: &[],
        };
        let o = LocalOrigin::default();
        // Levels: one face per block of every layer.
        let lv = view(&model, &c, &none, r, o).unwrap();
        assert!(lv.polygons > 50 && !lv.limited && lv.planes >= 3);
        assert!(lv
            .prims
            .iter()
            .all(|p| matches!(&p.geom, Geom::Face(f) if f.len() == 4) && p.color.is_some()));
        let mut sink = MockSink::default();
        let n1 = draw_view(&mut sink, r, lv);
        let again = view(&model, &c, &none, r, o).unwrap();
        draw_view(&mut sink, r, again);
        assert_eq!(sink.entities.len(), n1, "a new view replaces the previous one");
        // Cut-off hides low grades; the domain filter keeps one domain.
        c.rs.bm.view_cutoff = 4.0;
        let hi = view(&model, &c, &none, r, o).unwrap();
        assert!(hi.polygons < n1);
        c.rs.bm.view_cutoff = 0.0;
        c.rs.bm.view_domain = 1;
        let only_ox = view(&model, &c, &none, r, o).unwrap();
        assert!(only_ox.polygons < n1 && only_ox.polygons > 0);
        c.rs.bm.view_domain = 0;
        // Section by 2 points through the middle of the field.
        c.rs.bm.view_mode = 1;
        let pts = [[500_120.0, 8_499_900.0, 0.0], [500_120.0, 8_500_400.0, 0.0]];
        let sec = view(
            &model,
            &c,
            &ViewInput {
                axis: None,
                points: &pts,
            },
            r,
            o,
        )
        .unwrap();
        assert!(
            sec.polygons > 10
                && sec
                    .prims
                    .iter()
                    .all(|p| matches!(&p.geom, Geom::Face(f) if (3..=4).contains(&f.len())))
        );
        assert!(view(
            &model,
            &c,
            &ViewInput {
                axis: None,
                points: &pts[..1]
            },
            r,
            o
        )
        .is_err());
        // Cubes above the cut-off: one mesh per colour, with closed boxes (8 vertices, 6 faces each).
        c.rs.bm.view_mode = 3;
        c.rs.bm.view_cutoff = 4.0;
        let cubes = view(&model, &c, &none, r, o).unwrap();
        for p in &cubes.prims {
            let Geom::Mesh { vertices, faces } = &p.geom else {
                panic!()
            };
            assert_eq!(vertices.len() / 8 * 6, faces.len());
        }
        // Over the limit the highest-grade blocks are kept, not the deepest.
        let all: Vec<usize> = (0..model.len())
            .filter(|&i| model.domain[i] != 0 && ok_col(&model)[i].is_finite())
            .collect();
        let top = pick_cubes(all.clone(), Some(ok_col(&model)), 5);
        let lowest_kept = top.iter().map(|&i| ok_col(&model)[i]).fold(f32::MAX, f32::min);
        let best_dropped = all
            .iter()
            .filter(|i| !top.contains(i))
            .map(|&i| ok_col(&model)[i])
            .fold(f32::MIN, f32::max);
        assert!(
            top.len() == 5 && lowest_kept >= best_dropped,
            "{lowest_kept} < {best_dropped}"
        );
        // Without values an even stride covers the whole extent (first and late blocks).
        let spread = pick_cubes((0..100).collect(), None, 4);
        assert_eq!(spread, vec![0, 25, 50, 75]);
        c.rs.bm.view_limit = 3.0;
        let lim_cubes = view(&model, &c, &none, r, o).unwrap();
        assert!(lim_cubes.limited && lim_cubes.cubes.unwrap().0 == 3);
        let mut sink = MockSink::default();
        draw_view(&mut sink, r, lim_cubes);
        assert!(sink.infos.iter().any(|m| m.contains("mayor valor")), "{:?}", sink.infos);
        c.rs.bm.view_limit = 20_000.0;
        // Domain colouring and limit.
        c.rs.bm.view_mode = 0;
        c.rs.bm.view_attr = 3;
        c.rs.bm.view_limit = 5.0;
        let lim = view(&model, &c, &none, r, o).unwrap();
        assert!(lim.limited && lim.polygons <= 5);
        // A method that was not run leaves its column empty: nothing to show.
        c.rs.bm.view_attr = 9;
        assert_eq!(view(&model, &c, &none, r, o).unwrap().polygons, 0);
        // A model without the column at all (e.g. loaded from another run) names it.
        let bare = BlockModel::new(model.geometry);
        assert!(view(&bare, &c, &none, r, o).unwrap_err().contains("au_sk"));
        // Empty selection is an error line, not a crash.
        let mut sink = MockSink::default();
        assert_eq!(
            draw_view(
                &mut sink,
                r,
                ViewOutcome {
                    prims: vec![],
                    planes: 0,
                    polygons: 0,
                    limited: false,
                    cubes: None,
                }
            ),
            0
        );
        assert_eq!(sink.errors.len(), 1);
    }

    /// Three stacked domains: above C1, between C1 and C2, below C2.
    #[test]
    fn three_domains_between_two_named_surfaces() {
        let r = for_lang(Lang::Es);
        let flat = |z: f64| GridSurface {
            x0: -100.0,
            y0: -100.0,
            dx: 100.0,
            dy: 100.0,
            nx: 3,
            ny: 3,
            z: vec![z; 9],
        };
        let surfaces = vec![("C1".to_string(), flat(20.0)), ("C2".to_string(), flat(-20.0))];
        let doms = vec![
            DomainCfg {
                name: "Lixiviado".into(),
                surface: "Encima C1".into(),
                ..Default::default()
            },
            DomainCfg {
                name: "Óxidos".into(),
                surface: "Debajo C1; Encima C2".into(),
                ..Default::default()
            },
            DomainCfg {
                name: "Sulfuros".into(),
                surface: "debajo c2".into(),
                ..Default::default()
            },
        ];
        let regions = build_regions(&doms, &DomainMeshes::new(), &surfaces, r).unwrap();
        let mut m = BlockModel::new(BlockGeometry {
            origin: [-50.0, -50.0, -60.0],
            size: [10.0, 10.0, 10.0],
            n: [10, 10, 12],
            rotation: 0.0,
        });
        let refs: Vec<&dyn Region> = regions.iter().map(|x| x.as_ref()).collect();
        assign_domains(&mut m, &refs, [2, 2, 2]);
        for i in 0..m.len() {
            let z = m.geometry.center(i)[2];
            let want = if z > 20.0 {
                1
            } else if z > -20.0 {
                2
            } else {
                3
            };
            assert_eq!(m.domain[i], want, "block at z = {z}");
        }
        // A name that is not in the contacts table is explained.
        let bad = vec![
            DomainCfg {
                name: "X".into(),
                surface: "Encima C9".into(),
                ..Default::default()
            },
            doms[0].clone(),
        ];
        let err = build_regions(&bad, &DomainMeshes::new(), &surfaces, r).err().unwrap();
        assert!(err.contains("C9") && err.contains("X"), "{err}");
    }

    #[test]
    fn manual_extent_and_file_paths() {
        let (mut c, out, _) = setup();
        c.rs.bm.auto_extent = false;
        c.rs.bm.origin = [500_000.0, 8_500_000.0, 3_900.0];
        c.rs.bm.counts = [10.0, 10.0, 5.0];
        let g = geometry_for(&out.composites, &c).unwrap();
        assert_eq!((g.n, g.len()), ([10, 10, 5], 500));
        c.rs.bm.size = [0.0, 1.0, 1.0];
        assert!(geometry_for(&out.composites, &c).is_err());
        let p = model_file(Some(std::path::Path::new("/data/proj/mina.dwg")), "", ".ombm").unwrap();
        assert_eq!(p, std::path::PathBuf::from("/data/proj/mina.ombm"));
        let q = model_file(
            Some(std::path::Path::new("/data/proj/mina.dwg")),
            "/out",
            "_bloques.csv",
        )
        .unwrap();
        assert_eq!(q, std::path::PathBuf::from("/out/mina_bloques.csv"));
        assert!(model_file(None, "", ".ombm").is_none());
        assert_eq!(attr_column("au", 0).unwrap(), "au_ok");
        assert_eq!(attr_column("au", 3), None);
        assert_eq!(attr_column("au", 10).unwrap(), "tonnage");
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::*;
    use super::*;
    use crate::cad::mock::MockSink;
    use crate::i18n::Lang;
    use crate::rsi18n::for_lang;

    #[test]
    fn composites_respect_domains_and_caps() {
        let r = for_lang(Lang::Es);
        let mut c = cfg();
        c.rs.domains[1].cap = 5.2;
        let out = composite(&dataset(), &c, r).unwrap();
        assert_eq!(out.holes, 49);
        assert!(
            out.composites
                .iter()
                .all(|k| (k.domain == 1 && k.value < 2.0) || (k.domain == 2 && k.value > 4.0)),
            "domains must not mix grades"
        );
        assert!(out
            .composites
            .iter()
            .filter(|k| k.domain == 2)
            .all(|k| k.value <= 5.2 + 1e-12));
        assert!(out.capped > 0);
        assert_eq!(out.stats.len(), 2);
        assert!(out.stats[1].summary.mean > out.stats[0].summary.mean);
        assert!(out.stats.iter().all(|s| s.declustered_mean.is_finite() && s.cell > 0.0));
        assert!(
            (out.length - 49.0 * 60.0).abs() < 1e-6,
            "all drilled length is composited: {}",
            out.length
        );
        assert_eq!(out.weights.len(), out.composites.len());
        let lines = stats_lines(r, &out.stats);
        assert!(lines[0].starts_with("Óxidos: n="));
    }

    #[test]
    fn missing_attribute_and_default_domain() {
        let r = for_lang(Lang::En);
        let mut c = cfg();
        c.dh.assay_attr.clear();
        assert!(composite(&dataset(), &c, r).is_err());
        c.dh.assay_attr = "au".into();
        c.rs.domains.clear();
        let out = composite(&dataset(), &c, r).unwrap();
        assert!(out.composites.iter().all(|k| k.domain == 1));
        assert_eq!(effective_domains(&c)[0].name, "Global");
    }

    #[test]
    fn drawing_composites_is_idempotent_and_tagged() {
        let r = for_lang(Lang::Es);
        let c = cfg();
        let out = composite(&dataset(), &c, r).unwrap();
        let mut sink = MockSink::default();
        let n1 = draw_composites(&mut sink, r, &c, &out.composites, LocalOrigin::default());
        let n2 = draw_composites(&mut sink, r, &c, &out.composites, LocalOrigin::default());
        assert_eq!((n1, sink.entities.len()), (out.composites.len(), n2));
        assert!(sink
            .entities
            .iter()
            .all(|p| p.tag.class == Class::Comp && p.layer == "MIN_RS_COMP_AU"));
        assert_eq!(sink.undo_depth, 0);
        assert!(draw_composites(&mut sink, r, &c, &[], LocalOrigin::default()) == 0 && !sink.errors.is_empty());
    }

    #[test]
    fn contact_surface_follows_the_dipping_contact() {
        let r = for_lang(Lang::Es);
        let c = cfg();
        let holes = build_holes(&dataset());
        let (s, n) = contact_surface(&holes, &c, r).unwrap();
        assert_eq!(n, 49);
        // Contact elevation = 4000 - (20 + i), i along X (40 m spacing): dz/dx = -1/40.
        let z = |x: f64| 4_000.0 - 20.0 - (x - 500_000.0) / 40.0;
        for x in [500_040.0, 500_100.0, 500_200.0] {
            let got = s.value_at(x, 8_500_100.0);
            assert!((got - z(x)).abs() < 0.3, "{x}: {got} vs {}", z(x));
        }
        let mut sink = MockSink::default();
        draw_surface(&mut sink, r, &s, n, LocalOrigin::default());
        assert_eq!(sink.entities.len(), 1);
        let Geom::Mesh { vertices, faces } = &sink.entities[0].geom else {
            panic!()
        };
        assert!(!faces.is_empty() && faces.iter().flatten().all(|&i| i < vertices.len()));
        // Errors: missing configuration, too few points.
        let mut bad = cfg();
        bad.rs.contact.below.clear();
        bad.rs.contacts.clear();
        assert_eq!(contact_surface(&holes, &bad, r).unwrap_err(), r.m_contact_cfg);
        assert!(contact_surface(&holes[..2], &c, r).is_err());
    }

    #[test]
    fn composites_survive_a_csv_roundtrip_and_stale_files_are_refused() {
        let r = for_lang(Lang::Es);
        let c = cfg();
        let out = composite(&dataset(), &c, r).unwrap();
        let back = parse_composites_csv(&composites_csv(&out.composites, &c.rs.domains), &c.rs.domains).unwrap();
        assert_eq!(back.len(), out.composites.len());
        for (a, b) in out.composites.iter().zip(&back) {
            assert_eq!(
                (a.hole.as_str(), a.domain, a.from, a.to, a.value),
                (b.hole.as_str(), b.domain, b.from, b.to, b.value)
            );
            assert!((0..3).all(|i| (a.xyz[i] - b.xyz[i]).abs() < 1e-3 && (a.to_xyz[i] - b.to_xyz[i]).abs() < 1e-3));
        }
        // Same weights as at compositing time.
        let w = decluster_weights(&back, &c);
        assert!(w.iter().zip(&out.weights).all(|(a, b)| (a - b).abs() < 1e-9));
        // Hole indices are consistent per hole.
        assert!(back
            .windows(2)
            .all(|p| (p[0].hole == p[1].hole) == (p[0].hole_index == p[1].hole_index)));
        // A domain that is no longer in the table is refused.
        let renamed = vec![DomainCfg {
            name: "Otro".into(),
            ..Default::default()
        }];
        assert!(parse_composites_csv(&composites_csv(&out.composites, &c.rs.domains), &renamed).is_err());
        // v0.4 files (no end coordinates) still load.
        let old = "hole,from,to,x,y,z,value,domain\nH00,0,4,1,2,3,1.5,Óxidos\n";
        let o = parse_composites_csv(old, &c.rs.domains).unwrap();
        assert_eq!((o[0].from_xyz, o[0].domain), ([1.0, 2.0, 3.0], 1));
        assert!(parse_composites_csv("hole,from\n", &c.rs.domains).is_err());
        assert_eq!(composite_stamp(&c).domains, vec!["Óxidos", "Sulfuros"]);
    }

    #[test]
    fn csv_and_histogram() {
        let r = for_lang(Lang::En);
        let c = cfg();
        let out = composite(&dataset(), &c, r).unwrap();
        let csv = composites_csv(&out.composites, &c.rs.domains);
        assert!(csv.starts_with("hole,from,to,x,y,z,value,domain,"));
        assert_eq!(csv.lines().count(), out.composites.len() + 1);
        assert!(csv.contains("Óxidos") && csv.contains("Sulfuros"));
        let svg = histogram_chart(r, &out.composites, &out.weights, &c.rs.domains, "g/t");
        assert!(svg.starts_with("<svg") && svg.contains("Óxidos") && !svg.contains("NaN"));
        assert_eq!(var_name("Cu %"), "cu__");
        assert_eq!(num(1.23456), "1.235");
    }
}

// ───────────────────────────── Variography ─────────────────────────────

/// Direction parameters (azimuth, dip positive down) of a unit vector.
pub fn direction_angles(d: Vec3) -> (f64, f64) {
    (
        (d[0].atan2(d[1]).to_degrees() + 360.0) % 360.0,
        (-d[2]).clamp(-1.0, 1.0).asin().to_degrees(),
    )
}

/// Experimental variograms along the model's three principal axes (and
/// omnidirectional when enabled), using the panel's working orientation.
pub fn compute_variograms(samples: &[Sample], cfg: &ProjectConfig) -> Vec<(String, ExpVariogram)> {
    let v = &cfg.rs.vario;
    let axes = v.work.orientation().axes();
    let nlags = v.nlags.round().max(1.0) as usize;
    let mut out: Vec<(String, ExpVariogram)> = axes
        .iter()
        .zip(["λ1", "λ2", "λ3"])
        .map(|(ax, name)| {
            let (az, dip) = direction_angles(*ax);
            let p = VarioParams {
                azimuth: az,
                dip,
                az_tol: v.az_tol,
                dip_tol: v.dip_tol,
                bw_h: v.bw_h,
                bw_v: v.bw_v,
                lag: v.lag,
                lag_tol: v.lag / 2.0,
                nlags,
            };
            (name.to_string(), experimental(samples, &p))
        })
        .collect();
    if v.omni {
        out.push(("Omni".into(), experimental(samples, &VarioParams::omni(v.lag, nlags))));
    }
    out
}

pub fn variogram_chart(r: &RsStrings, exps: &[(String, ExpVariogram)], model: &VariogramModel, domain: &str) -> String {
    let max_h = exps
        .iter()
        .flat_map(|(_, e)| e.points.iter().map(|p| p.lag))
        .fold(0.0, f64::max);
    let mut series = Vec::new();
    for (i, (name, e)) in exps.iter().enumerate() {
        series.push(Series::new(
            name.clone(),
            e.points.iter().map(|p| (p.lag, p.gamma)).collect(),
            Style::Dots,
            i,
        ));
        if !model.structures.is_empty() && max_h > 0.0 {
            series.push(Series::new(
                "",
                (0..=40)
                    .map(|k| {
                        let h = max_h * k as f64 / 40.0;
                        (h, model.gamma_along(e.dir, h))
                    })
                    .collect(),
                Style::Line,
                i,
            ));
        }
    }
    if model.total_sill() > 0.0 && max_h > 0.0 {
        series.push(Series::new(
            format!("C = {}", num(model.total_sill())),
            vec![(0.0, model.total_sill()), (max_h, model.total_sill())],
            Style::Dashed,
            5,
        ));
    }
    render(&Chart {
        title: &fmt(r.ch_vario, &[&domain]),
        x_label: r.ch_h,
        y_label: r.ch_gamma,
        series: &series,
        width: 320.0,
        height: 230.0,
        y_from_zero: true,
    })
}

/// Automatic fit of the working model to the experimental variograms.
pub fn fit_working_model(
    exps: &[(String, ExpVariogram)],
    samples: &[Sample],
    cfg: &ProjectConfig,
) -> geostat_engine::fit::FitResult {
    let v = &cfg.rs.vario;
    let start = {
        let m = v.work.model();
        if m.structures.is_empty() {
            let vals: Vec<f64> = samples.iter().map(|s| s.value).collect();
            let var = summarize(&vals, None).variance.max(1e-6);
            let reach = (v.lag * v.nlags * 0.6).max(10.0);
            VariogramModel {
                nugget: 0.2 * var,
                structures: vec![geostat_engine::model::Structure {
                    kind: KINDS[v.fit_kind1.min(2)],
                    sill: 0.8 * var,
                    ranges: [reach; 3],
                }],
                orientation: v.work.orientation(),
            }
        } else {
            m
        }
    };
    let k1 = KINDS[v.fit_kind1.min(2)];
    let mut kinds = vec![k1];
    if v.fit_two {
        kinds.push(if k1 == Kind::Exponential {
            Kind::Gaussian
        } else {
            Kind::Exponential
        });
    }
    let list: Vec<ExpVariogram> = exps
        .iter()
        .filter(|(n, _)| n != "Omni")
        .map(|(_, e)| e.clone())
        .collect();
    fit(
        &list,
        &start,
        &FitConfig {
            kinds,
            orientation: v.work.orientation(),
            fixed_nugget: v.fix_nugget.then_some(v.work.nugget),
            max_iter: 3000,
        },
    )
}

pub fn run_crossval(
    samples: &[Sample],
    cfg: &ProjectConfig,
    domain: &str,
    r: &RsStrings,
) -> Result<(CvStats, String), String> {
    let work = &cfg.rs.vario.work;
    let model = work.model();
    // A pure-nugget model is valid (kriging then averages); only a zero sill is not.
    if model.total_sill() <= 0.0 {
        return Err(fmt(r.m_no_model, &[&domain]));
    }
    let nb = cfg.rs.bm.passes(work.orientation()).remove(0);
    let cv = cross_validate(
        samples,
        &model,
        &nb,
        Method::Ordinary,
        (samples.len() / 2000).max(1),
        true,
    );
    if cv.stats.n < 2 {
        return Err(fmt(r.m_cv_few, &[&domain]));
    }
    let s = &cv.stats;
    Ok((
        s.clone(),
        fmt(
            r.m_cv_done,
            &[
                &domain,
                &s.n,
                &num(s.mean_error),
                &num(s.mse),
                &num(s.correlation),
                &num(s.z_mean),
                &num(s.z_var),
            ],
        ),
    ))
}

// ───────────────────────────── Block model ─────────────────────────────

pub type DomainMeshes = HashMap<String, Vec<MeshData>>;

/// One region per domain: solids tagged with the domain, and/or the side of
/// the contact surface. A single domain without either covers everything.
pub fn build_regions(
    doms: &[DomainCfg],
    meshes: &DomainMeshes,
    surfaces: &[(String, GridSurface)],
    r: &RsStrings,
) -> Result<Vec<Box<dyn Region>>, String> {
    let mut out: Vec<Box<dyn Region>> = Vec::new();
    for d in doms {
        let mut parts: Vec<Box<dyn Region>> = Vec::new();
        if let Some(m) = meshes.get(&d.name).filter(|m| !m.is_empty()) {
            parts.push(Box::new(SolidsRegion::new(m)));
        }
        for (name, above) in d.conditions() {
            if surfaces.is_empty() {
                return Err(r.m_contact_cfg.to_string());
            }
            // An empty name means the first surface.
            let s = if name.is_empty() {
                Some(&surfaces[0].1)
            } else {
                surfaces
                    .iter()
                    .find(|(n, _)| n.trim().eq_ignore_ascii_case(name.trim()))
                    .map(|(_, s)| s)
            };
            let Some(s) = s else {
                return Err(fmt(r.m_unknown_surface, &[&name, &d.name]));
            };
            parts.push(Box::new(SurfaceSide {
                surface: s.clone(),
                above,
            }));
        }
        out.push(match parts.len() {
            0 if doms.len() == 1 => Box::new(Everything),
            0 => return Err(fmt(r.m_bm_no_region, &[&d.name])),
            1 => parts.pop().unwrap(),
            _ => Box::new(And(parts)),
        });
    }
    Ok(out)
}

pub fn geometry_for(comps: &[Composite3d], cfg: &ProjectConfig) -> Result<BlockGeometry, String> {
    let b = &cfg.rs.bm;
    if b.auto_extent {
        let (lo, hi) = comps.iter().fold(([f64::MAX; 3], [f64::MIN; 3]), |(lo, hi), c| {
            (
                std::array::from_fn(|i| lo[i].min(c.xyz[i])),
                std::array::from_fn(|i| hi[i].max(c.xyz[i])),
            )
        });
        if comps.is_empty() {
            return Err("no composites".into());
        }
        BlockGeometry::covering(lo, hi, b.size, b.rotation, b.pad).map_err(|e| e.to_string())
    } else {
        let n = b.counts.map(|v| v.round().max(1.0) as usize);
        let g = BlockGeometry {
            origin: b.origin,
            size: b.size,
            n,
            rotation: b.rotation,
        };
        if b.size.iter().any(|s| *s <= 0.0) || g.len() > blockmodel_engine::geometry::MAX_BLOCKS {
            return Err(blockmodel_engine::geometry::GeometryError::TooManyBlocks(g.len()).to_string());
        }
        Ok(g)
    }
}

pub fn create_model(
    comps: &[Composite3d],
    cfg: &ProjectConfig,
    regions: &[Box<dyn Region>],
) -> Result<BlockModel, String> {
    let mut m = BlockModel::new(geometry_for(comps, cfg)?);
    m.domain_names = effective_domains(cfg).into_iter().map(|d| d.name).collect();
    let refs: Vec<&dyn Region> = regions.iter().map(|r| r.as_ref()).collect();
    assign_domains(&mut m, &refs, cfg.rs.bm.discretization());
    let min = cfg.rs.bm.min_fraction.max(0.0) as f32;
    for i in 0..m.len() {
        if m.domain[i] != 0 && m.fraction[i] < min {
            (m.domain[i], m.fraction[i]) = (0, 0.0);
        }
    }
    Ok(m)
}

pub fn run_estimation(
    model: &mut BlockModel,
    comps: &[Composite3d],
    weights: &[f64],
    cfg: &ProjectConfig,
    r: &RsStrings,
) -> Result<blockmodel_engine::estimate::Report, String> {
    let doms = effective_domains(cfg);
    let samples = samples_of(comps);
    let mut plans = Vec::new();
    for (i, d) in doms.iter().enumerate() {
        let id = i as u8 + 1;
        if !model.domain.contains(&id) {
            continue;
        }
        let mc: &ModelCfg = cfg.rs.model_of(&d.name).ok_or_else(|| fmt(r.m_no_model, &[&d.name]))?;
        let vm = mc.model();
        if vm.total_sill() <= 0.0 {
            return Err(fmt(r.m_no_model, &[&d.name]));
        }
        let idx: Vec<usize> = (0..samples.len()).filter(|&k| samples[k].domain == id as u32).collect();
        let sub: Vec<Sample> = idx.iter().map(|&k| samples[k].clone()).collect();
        let w: Vec<f64> = idx.iter().map(|&k| weights[k]).collect();
        plans.push(DomainPlan {
            id,
            passes: cfg.rs.bm.passes(vm.orientation),
            model: vm,
            sk_mean: weighted_mean(&sub, &w),
            density: d.density,
        });
    }
    let b = &cfg.rs.bm;
    let var = var_name(&cfg.dh.assay_attr);
    let plan = Plan {
        variable: var,
        ordinary: b.ordinary,
        simple: b.simple,
        idw: b.idw.then_some(b.idw_power),
        nearest: b.nearest || !b.ordinary,
        discretization: b.discretization(),
        domains: plans,
    };
    Ok(estimate_model(model, &samples, &plan, &|_| {}))
}

pub fn estimation_lines(r: &RsStrings, model: &BlockModel, rep: &blockmodel_engine::estimate::Report) -> Vec<String> {
    rep.domains
        .iter()
        .map(|d| {
            fmt(
                r.m_bm_domain,
                &[
                    &model.domain_name(d.id),
                    &d.blocks,
                    &d.estimated,
                    &d.per_pass.iter().map(|n| n.to_string()).collect::<Vec<_>>().join("/"),
                    &d.samples,
                ],
            )
        })
        .collect()
}

/// Grade column used for resources: OK when available, else NN.
pub fn grade_column(model: &BlockModel, var: &str) -> Option<String> {
    ["ok", "nn", "idw", "sk"]
        .iter()
        .map(|s| format!("{var}_{s}"))
        .find(|c| model.column(c).is_some_and(|col| col.iter().any(|v| v.is_finite())))
}

pub fn resource_lines(r: &RsStrings, model: &BlockModel, var: &str, cutoff: f64, unit: &str) -> Vec<String> {
    let Some(col) = grade_column(model, var) else {
        return Vec::new();
    };
    let mut lines = vec![fmt(r.m_resources, &[&num(cutoff), &unit])];
    for row in report::summary_by_domain(model, &col, cutoff) {
        lines.push(fmt(
            r.m_resource_row,
            &[
                &row.domain,
                &row.blocks,
                &num(row.tonnes.round()),
                &num(row.grade),
                &unit,
                &num(row.metal.round()),
            ],
        ));
    }
    lines
}

fn domain_filter(cfg: &ProjectConfig) -> Filter {
    Filter {
        domain: cfg.rs.bm.view_domain.min(255) as u8,
        min_fraction: 0.0,
    }
}

/// Validation lines and chart (swath, grade-tonnage or histogram).
pub fn validate(
    r: &RsStrings,
    model: &BlockModel,
    comps: &[Composite3d],
    weights: &[f64],
    cfg: &ProjectConfig,
    name: &str,
) -> Result<(Vec<String>, String), String> {
    let var = var_name(&cfg.dh.assay_attr);
    let ok = grade_column(model, &var).ok_or_else(|| fmt(r.m_no_col, &[&format!("{var}_ok")]))?;
    let nn = format!("{var}_nn");
    let doms = effective_domains(cfg);
    let samples = samples_of(comps);
    let mut lines = vec![fmt(r.m_validate_head, &[&name])];
    for (i, d) in doms.iter().enumerate() {
        let id = i as u8 + 1;
        let f = Filter {
            domain: id,
            min_fraction: 0.0,
        };
        let bs = report::block_summary(model, &ok, &f);
        let idx: Vec<usize> = (0..samples.len()).filter(|&k| samples[k].domain == id as u32).collect();
        let sub: Vec<Sample> = idx.iter().map(|&k| samples[k].clone()).collect();
        let w: Vec<f64> = idx.iter().map(|&k| weights[k]).collect();
        let nn_mean = report::block_summary(model, &nn, &f).mean;
        lines.push(fmt(
            r.m_validate_row,
            &[
                &d.name,
                &num(bs.mean),
                &num(bs.cv),
                &num(weighted_mean(&sub, &w)),
                &num(nn_mean),
            ],
        ));
    }
    let f = domain_filter(cfg);
    let b = &cfg.rs.bm;
    let svg = match b.chart {
        0 => {
            let axis = [Axis::X, Axis::Y, Axis::Z][b.swath_axis.min(2)];
            let sw = report::swath(model, &[&ok, &nn], axis, b.swath_width, &f);
            let ss = report::sample_swath(&samples, weights, axis, b.swath_width, f.domain as u32);
            let series = vec![
                Series::new(
                    r.ch_model,
                    sw.iter().map(|s| (s.center, s.means[0])).collect(),
                    Style::Line,
                    0,
                ),
                Series::new(
                    r.ch_nn,
                    sw.iter().map(|s| (s.center, s.means[1])).collect(),
                    Style::Line,
                    1,
                ),
                Series::new(r.ch_comp, ss, Style::Dots, 2),
            ];
            render(&Chart {
                title: &fmt(r.ch_swath, &[&name]),
                x_label: r.w_axes[axis as usize],
                y_label: &cfg.rs.unit,
                series: &series,
                width: 320.0,
                height: 230.0,
                y_from_zero: false,
            })
        }
        1 => {
            let max = comps.iter().map(|c| c.value).fold(0.0, f64::max).max(1e-6);
            let cuts: Vec<f64> = (0..=24).map(|k| max * k as f64 / 24.0).collect();
            let m = report::model_grade_tonnage(model, &ok, &f, &cuts);
            let tot = m.first().map_or(0.0, |g| g.weight).max(1e-300);
            let sel: Vec<usize> = (0..samples.len())
                .filter(|&k| f.domain == 0 || samples[k].domain == f.domain as u32)
                .collect();
            let s: Vec<Sample> = sel.iter().map(|&k| samples[k].clone()).collect();
            let w: Vec<f64> = sel.iter().map(|&k| weights[k]).collect();
            let c = report::sample_grade_tonnage(&s, &w, &cuts);
            let series = vec![
                Series::new(
                    r.ch_blocks,
                    m.iter().map(|g| (g.cutoff, g.weight / tot)).collect(),
                    Style::Line,
                    0,
                ),
                Series::new(
                    r.ch_comp,
                    c.iter().map(|g| (g.cutoff, g.weight)).collect(),
                    Style::Line,
                    1,
                ),
            ];
            render(&Chart {
                title: &fmt(r.ch_gt, &[&name]),
                x_label: r.ch_cutoff,
                y_label: r.ch_fraction,
                series: &series,
                width: 320.0,
                height: 230.0,
                y_from_zero: true,
            })
        }
        _ => histogram_chart(r, comps, weights, &doms, &cfg.rs.unit),
    };
    Ok((lines, svg))
}

// ───────────────────────────── Views ─────────────────────────────

const DOMAIN_COLORS: [u8; 10] = [3, 1, 5, 2, 6, 4, 30, 62, 141, 210];

/// First view attribute of the geotechnical columns (RMR, GSI, log Q, class).
pub const GT_VIEW_ATTR: usize = 11;

/// Column shown for the view attribute index (`None` = the domain itself).
pub fn attr_column(var: &str, i: usize) -> Option<String> {
    if let Some(c) = i.checked_sub(GT_VIEW_ATTR).and_then(|k| crate::geotech::COLUMNS.get(k)) {
        return Some(c.to_string());
    }
    let suffix = [
        "ok", "ok_var", "ok_ke", "", "pass", "n", "dist", "nn", "idw", "sk", "tonnage",
    ]
    .get(i)?;
    match i {
        3 => None,
        10 => Some("tonnage".into()),
        _ => Some(format!("{var}_{suffix}")),
    }
}

pub struct ViewInput<'a> {
    pub axis: Option<&'a [Vec3]>,
    pub points: &'a [Vec3],
}

#[derive(Debug)]
pub struct ViewOutcome {
    pub prims: Vec<Prim>,
    pub planes: usize,
    pub polygons: usize,
    pub limited: bool,
    /// Cubes view: `(drawn, total)` blocks.
    pub cubes: Option<(usize, usize)>,
}

/// Blocks to draw as cubes when there are more than `limit`: the highest
/// values first (a cut by storage order would keep the deepest levels and
/// drop the top of the deposit). Without values (domain colouring) an even
/// stride keeps the whole extent represented.
pub fn pick_cubes(mut blocks: Vec<usize>, values: Option<&[f32]>, limit: usize) -> Vec<usize> {
    let limit = limit.max(1);
    if blocks.len() <= limit {
        return blocks;
    }
    match values {
        Some(v) => {
            blocks.sort_by(|a, b| v[*b].total_cmp(&v[*a]).then(a.cmp(b)));
            blocks.truncate(limit);
            blocks
        }
        None => {
            let n = blocks.len();
            (0..limit).map(|k| blocks[k * n / limit]).collect()
        }
    }
}

pub fn view(
    model: &BlockModel,
    cfg: &ProjectConfig,
    input: &ViewInput,
    r: &RsStrings,
    origin: LocalOrigin,
) -> Result<ViewOutcome, String> {
    use blockmodel_engine::views::{block_cubes, level_polygons, plane_polygons};
    use solids_engine::cut::{level_planes, planes_along_axis, vertical_planes};
    let b = &cfg.rs.bm;
    let var = var_name(&cfg.dh.assay_attr);
    let col = attr_column(&var, b.view_attr);
    let values: Option<&[f32]> = match &col {
        Some(c) => Some(model.column(c).ok_or_else(|| fmt(r.m_no_col, &[c]))?),
        None => None,
    };
    let is_grade = matches!(b.view_attr, 0 | 7 | 8 | 9);
    let dom = b.view_domain.min(255) as u8;
    // Grade attributes honour the cut-off; the other attributes show every block.
    let base = |i: usize| -> bool {
        if model.domain[i] == 0 || (dom != 0 && model.domain[i] != dom) || model.fraction[i] <= 0.0 {
            return false;
        }
        values.is_none_or(|v| v[i].is_finite() && (!is_grade || v[i] as f64 >= b.view_cutoff))
    };
    // Geotechnical views can show only the poor (or good) rock and keep one side of a
    // plane, to see inside the model. They never apply to the resource attributes.
    let gt_view = b.view_attr >= GT_VIEW_ATTR;
    let filter = gt_view
        .then_some((b.view_filter, b.view_threshold))
        .filter(|(m, _)| *m != 0);
    let cut = (gt_view && b.view_cut).then(|| {
        (
            b.view_cut_point,
            geotech_engine::stereonet::Plane::new(b.view_cut_dip, b.view_cut_dipdir).normal(),
            b.view_cut_side == 1,
        )
    });
    let accept = |i: usize| -> bool {
        if !base(i) {
            return false;
        }
        if let (Some((m, t)), Some(v)) = (filter, values) {
            let x = v[i] as f64;
            if (m == 1 && x >= t) || (m == 2 && x <= t) {
                return false;
            }
        }
        if let Some((p, n, back)) = &cut {
            let d = mining_core::dot(mining_core::sub(model.geometry.center(i), *p), *n);
            // Blocks centred exactly on the plane belong to the front side, so the two
            // sides never overlap and together hold every block.
            if (!*back && d < 0.0) || (*back && d >= 0.0) {
                return false;
            }
        }
        true
    };
    // Colour scale over the accepted blocks.
    let (lo, hi) = match values {
        Some(v) => (0..model.len())
            .filter(|&i| base(i))
            .fold((f64::MAX, f64::MIN), |(a, z), i| {
                (a.min(v[i] as f64), z.max(v[i] as f64))
            }),
        None => (0.0, 1.0),
    };
    let custom = cfg.dh.custom_ranges();
    let scheme = if is_grade && b.view_attr < GT_VIEW_ATTR {
        cfg.dh.scheme()
    } else {
        ColorScheme::BlueToRed
    };
    let color_of = |i: usize| -> u8 {
        match values {
            None => DOMAIN_COLORS[(model.domain[i] as usize - 1) % DOMAIN_COLORS.len()],
            Some(v) => {
                if b.view_attr == 4 {
                    return [3, 2, 1, 6][(v[i].round() as usize).clamp(1, 4) - 1];
                }
                // Geotechnics: the class has its own colours; RMR, GSI and log Q
                // are reversed so poor rock is red.
                if b.view_attr == GT_VIEW_ATTR + 3 {
                    return crate::geotech::class_color(v[i].round().clamp(1.0, 5.0) as u8);
                }
                if b.view_attr >= GT_VIEW_ATTR {
                    // RMR, GSI, log Q and the number of records: low is bad (red). The
                    // mean distance to the records: far is bad (red).
                    let far_is_bad = b.view_attr == GT_VIEW_ATTR + 4;
                    let x = if far_is_bad { v[i] as f64 } else { hi + lo - v[i] as f64 };
                    return grade_color(ColorScheme::BlueToRed, x, lo, hi, &[]);
                }
                if scheme == ColorScheme::Custom && custom.is_empty() {
                    GRAY
                } else {
                    grade_color(scheme, v[i] as f64, lo, hi, &custom)
                }
            }
        }
    };
    let set = crate::axes::new_id();
    let limit = b.view_limit.round().max(1.0) as usize;
    let mk_tag = |plane: &str, i: usize| {
        Tag::new(Class::View)
            .with("set", &set)
            .with("mode", b.view_mode)
            .with("plane", plane)
            .with("block", i)
    };
    let mut prims: Vec<Prim> = Vec::new();
    let (mut planes_n, mut limited) = (0usize, false);
    let mut cubes: Option<(usize, usize)> = None;
    let push_poly = |prims: &mut Vec<Prim>, pts: &[Vec3], i: usize, plane: &str| {
        // Fan polygons with more than 4 corners into triangles-as-quads.
        let loc: Vec<Vec3> = pts.iter().map(|p| origin.to_local(*p)).collect();
        let faces: Vec<Vec<Vec3>> = if loc.len() <= 4 {
            vec![loc]
        } else {
            (1..loc.len() - 1).map(|k| vec![loc[0], loc[k], loc[k + 1]]).collect()
        };
        for f in faces {
            prims.push(Prim {
                geom: Geom::Face(f),
                layer: LAYER_VIEW.into(),
                color: Some(color_of(i)),
                tag: mk_tag(plane, i),
            });
        }
    };
    let g = &model.geometry;
    match b.view_mode {
        0 => {
            let (lo_z, hi_z, step) = if b.view_z_to > b.view_z_from {
                (b.view_z_from, b.view_z_to, b.view_z_step.max(0.01))
            } else {
                (
                    g.origin[2] + g.size[2] / 2.0,
                    g.origin[2] + g.n[2] as f64 * g.size[2],
                    g.size[2],
                )
            };
            let planes = level_planes(lo_z, hi_z, step).map_err(|e| e.to_string())?;
            for p in &planes {
                planes_n += 1;
                let z = p.plane.origin[2];
                for poly in level_polygons(model, z, &accept) {
                    if prims.len() >= limit {
                        limited = true;
                        break;
                    }
                    push_poly(&mut prims, &poly.points, poly.block, &p.label);
                }
            }
        }
        1 | 2 => {
            let planes = if b.view_mode == 1 {
                let (p1, p2) = (
                    input.points.first().ok_or("point missing")?,
                    input.points.get(1).ok_or("point missing")?,
                );
                vertical_planes(*p1, *p2, b.view_spacing, b.view_count.round().max(1.0) as usize)
            } else {
                planes_along_axis(input.axis.ok_or("axis missing")?, b.view_interval, 0.0, 0.0)
            }
            .map_err(|e| e.to_string())?;
            for p in &planes {
                planes_n += 1;
                let remaining = limit.saturating_sub(prims.len());
                let (polys, hit) = plane_polygons(model, &p.plane, &accept, remaining.max(1));
                for poly in &polys {
                    push_poly(&mut prims, &poly.points, poly.block, &p.label);
                }
                if hit || prims.len() >= limit {
                    limited = true;
                    break;
                }
            }
        }
        _ => {
            let blocks: Vec<usize> = (0..model.len()).filter(|&i| accept(i)).collect();
            let total = blocks.len();
            // Geotechnical attributes keep an even sample (no "highest first").
            let chosen = pick_cubes(blocks, values.filter(|_| b.view_attr < GT_VIEW_ATTR), limit);
            limited = chosen.len() < total;
            cubes = Some((chosen.len(), total));
            let mut by_color: BTreeMap<u8, Vec<usize>> = BTreeMap::new();
            for &i in &chosen {
                by_color.entry(color_of(i)).or_default().push(i);
            }
            for (c, list) in by_color {
                let (v, f) = block_cubes(model, &list);
                prims.push(Prim {
                    geom: Geom::Mesh {
                        vertices: v.into_iter().map(|p| origin.to_local(p)).collect(),
                        faces: f,
                    },
                    layer: LAYER_VIEW.into(),
                    color: Some(c),
                    tag: Tag::new(Class::View)
                        .with("set", &set)
                        .with("mode", 3)
                        .with("blocks", list.len()),
                });
            }
            planes_n = 1;
        }
    }
    let polygons = prims.len();
    Ok(ViewOutcome {
        cubes,
        prims,
        planes: planes_n,
        polygons,
        limited,
    })
}

pub fn draw_view(sink: &mut dyn CadSink, r: &RsStrings, out: ViewOutcome) -> usize {
    if out.prims.is_empty() {
        sink.error(r.m_view_empty);
        return 0;
    }
    sink.begin("MIN_RS_BMVIEW");
    sink.ensure_layers(&[LayerDef::new(LAYER_VIEW, 7)]);
    sink.remove_where(&|t: &Tag| t.class == Class::View);
    let n = sink.add(out.prims);
    sink.end();
    match (out.limited, out.cubes) {
        (true, Some((drawn, total))) => sink.info(&fmt(r.m_view_cubes_limited, &[&drawn, &total])),
        (true, None) => sink.info(&fmt(r.m_view_limited, &[&n])),
        _ => {}
    }
    sink.info(&fmt(r.m_view_done, &[&n, &out.planes]));
    n
}

/// Path of the model file for a drawing at `dwg` (or in `folder`).
pub fn model_file(dwg: Option<&std::path::Path>, folder: &str, ext: &str) -> Option<std::path::PathBuf> {
    let stem = dwg?.file_stem()?.to_string_lossy().to_string();
    let dir = if folder.trim().is_empty() {
        dwg?.parent()?.to_path_buf()
    } else {
        std::path::PathBuf::from(folder.trim())
    };
    Some(dir.join(format!("{stem}{ext}")))
}
