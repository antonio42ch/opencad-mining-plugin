//! Drillholes: load (with column mapping), draw (idempotent), clear.
//! Scene building is pure (`scene`), so it is unit-tested without a CAD.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::Path;

use drillhole_engine::color::{category_color, grade_color, ColorScheme, GRAY};
use drillhole_engine::error::TableKind;
use drillhole_engine::import::{self, ColumnMap, Field};
use drillhole_engine::pipeline::{attribute_range, build_holes, HoleModel, RangeScope};
use drillhole_engine::table::RawTable;
use drillhole_engine::validate::validate;
use drillhole_engine::{hole_key, DataSet, ImportError, Severity};
use mining_core::LocalOrigin;

use crate::cad::{CadSink, Geom, LayerDef, Prim};
use crate::config::{DrillholeConfig, ProjectConfig};
use crate::i18n::{fmt, Strings};
use crate::xdata::{Class, Tag};

pub const LAYER_TRACE: &str = "MIN_DH_TRAZA";
pub const LAYER_COLLAR: &str = "MIN_DH_COLLAR";
pub const LAYER_GEO_PREFIX: &str = "MIN_DH_GEO_";
pub const LAYER_ASSAY_PREFIX: &str = "MIN_DH_LEY_";

/// Asks the user to map columns: `(table, headers, required, optional)` ->
/// field key -> header, or `None` when cancelled.
pub type MappingPrompt<'a> =
    dyn FnMut(TableKind, &[String], &[Field], &[Field]) -> Option<BTreeMap<String, String>> + 'a;

pub struct Loaded {
    pub data: DataSet,
    pub messages: usize,
    /// Validation report lines, `true` = error.
    pub report: Vec<(bool, String)>,
    pub summary: String,
    pub assay_attrs: Vec<String>,
    pub geo_attrs: Vec<String>,
}

fn read_table(path: &str, kind: TableKind) -> Result<Option<RawTable>, ImportError> {
    if path.trim().is_empty() {
        return Ok(None);
    }
    RawTable::from_path(Path::new(path.trim()), kind).map(Some)
}

/// Parses with the stored mapping; on a mapping error asks once, stores the
/// answer in the config and retries.
fn parse_with_mapping<T>(
    raw: &RawTable,
    kind: TableKind,
    cfg: &mut DrillholeConfig,
    ask: &mut MappingPrompt,
    parse: fn(&RawTable, &ColumnMap) -> Result<Vec<T>, ImportError>,
) -> Result<Vec<T>, ImportError> {
    match parse(raw, &cfg.column_map(kind.key())) {
        Err(e) if e.headers().is_some() || matches!(e, ImportError::UnknownMappedColumn { .. }) => {
            let headers: Vec<String> = raw
                .rows
                .first()
                .map(|h| h.iter().map(|s| s.trim().to_string()).collect())
                .unwrap_or_default();
            let (req, opt) = Field::for_table(kind);
            let Some(map) = ask(kind, &headers, req, opt) else {
                return Err(e);
            };
            cfg.column_maps.insert(kind.key().to_string(), map);
            parse(raw, &cfg.column_map(kind.key()))
        }
        other => other,
    }
}

/// Reads, maps and validates the files named in the settings. Talks to the
/// user only through `ask` (column mapping); everything else is returned.
pub fn load(s: &Strings, cfg: &mut ProjectConfig, ask: &mut MappingPrompt) -> Result<Loaded, String> {
    let d = &mut cfg.dh;
    if d.collar.trim().is_empty() {
        return Err(s.m_no_collar_file.to_string());
    }
    let (collar, survey, assays, geo) = (d.collar.clone(), d.survey.clone(), d.assays.clone(), d.geo.clone());
    let raw = (|| -> Result<DataSet, ImportError> {
        let mut ds = DataSet::default();
        let raw = read_table(&collar, TableKind::Collar)?.expect("checked above");
        ds.collars = parse_with_mapping(&raw, TableKind::Collar, d, ask, import::parse_collars)?;
        if let Some(raw) = read_table(&survey, TableKind::Survey)? {
            ds.surveys = parse_with_mapping(&raw, TableKind::Survey, d, ask, import::parse_surveys)?;
        }
        if let Some(raw) = read_table(&assays, TableKind::Assay)? {
            ds.assays = parse_with_mapping(&raw, TableKind::Assay, d, ask, import::parse_assays)?;
        }
        if let Some(raw) = read_table(&geo, TableKind::Geo)? {
            ds.geo = parse_with_mapping(&raw, TableKind::Geo, d, ask, import::parse_geo)?;
        }
        Ok(ds)
    })()
    .map_err(|e| s.import_error(&e))?;
    let (clean, msgs) = validate(&raw);
    let report = msgs
        .iter()
        .map(|m| {
            (
                m.severity >= Severity::Error,
                format!("[{}] {:?} {}: {}", m.rule, m.severity, m.hole_id, m.message),
            )
        })
        .collect();
    let assay_attrs = import::assay_attributes(&clean.assays);
    let geo_attrs = import::geo_attributes(&clean.geo);
    if !assay_attrs.contains(&d.assay_attr) {
        d.assay_attr = assay_attrs.first().cloned().unwrap_or_default();
    }
    if !geo_attrs.contains(&d.geo_attr) {
        d.geo_attr = geo_attrs.first().cloned().unwrap_or_default();
    }
    let summary = fmt(
        s.m_loaded,
        &[
            &clean.collars.len(),
            &clean.surveys.len(),
            &clean.assays.len(),
            &clean.geo.len(),
            &msgs.len(),
        ],
    );
    Ok(Loaded {
        data: clean,
        messages: msgs.len(),
        report,
        summary,
        assay_attrs,
        geo_attrs,
    })
}

/// Fills `cfg.origin` automatically (first collar rounded down to 1000 m)
/// when enabled and left at 0,0,0. Returns `true` when it changed.
pub fn auto_origin(cfg: &mut ProjectConfig, data: &DataSet) -> bool {
    let o = &mut cfg.origin;
    if !o.enabled || o.x != 0.0 || o.y != 0.0 || o.z != 0.0 {
        return false;
    }
    let Some(c) = data.collars.first() else {
        return false;
    };
    let r = |v: f64| (v / 1000.0).floor() * 1000.0;
    (o.x, o.y, o.z) = (r(c.x), r(c.y), 0.0);
    true
}

pub fn origin(cfg: &ProjectConfig) -> LocalOrigin {
    let o = &cfg.origin;
    if o.enabled {
        LocalOrigin { x: o.x, y: o.y, z: o.z }
    } else {
        LocalOrigin::default()
    }
}

fn layer_suffix(attr: &str) -> String {
    attr.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect()
}

/// Layers the scene uses.
pub fn layers(cfg: &DrillholeConfig) -> Vec<LayerDef> {
    let mut v = vec![LayerDef::new(LAYER_TRACE, 8), LayerDef::new(LAYER_COLLAR, 3)];
    if cfg.draw_assays && !cfg.assay_attr.is_empty() {
        v.push(LayerDef::new(
            format!("{LAYER_ASSAY_PREFIX}{}", layer_suffix(&cfg.assay_attr)),
            7,
        ));
    }
    if cfg.draw_geo && !cfg.geo_attr.is_empty() {
        v.push(LayerDef::new(
            format!("{LAYER_GEO_PREFIX}{}", layer_suffix(&cfg.geo_attr)),
            7,
        ));
    }
    v
}

/// Pure scene: every entity to create for `holes` with the given settings.
pub fn scene(holes: &[HoleModel], cfg: &DrillholeConfig, origin: LocalOrigin) -> Vec<Prim> {
    let mut out = Vec::new();
    let h = cfg.label_height;
    let scheme = cfg.scheme();
    let custom = cfg.custom_ranges();
    let global = if cfg.scope() == RangeScope::Global {
        attribute_range(holes.iter().flat_map(|x| &x.assays), &cfg.assay_attr)
    } else {
        None
    };
    let assay_layer = format!("{LAYER_ASSAY_PREFIX}{}", layer_suffix(&cfg.assay_attr));
    let geo_layer = format!("{LAYER_GEO_PREFIX}{}", layer_suffix(&cfg.geo_attr));
    let loc = |p| origin.to_local(p);

    for hole in holes {
        let id = &hole.collar.hole_id;
        let c = &hole.collar;
        if cfg.draw_trace && hole.trace.len() >= 2 {
            out.push(Prim {
                geom: Geom::Polyline3d(hole.trace.iter().map(|n| loc(n.xyz)).collect()),
                layer: LAYER_TRACE.into(),
                color: None,
                tag: Tag::new(Class::Trace).with("hole", id).with("length", c.max_depth),
            });
        }
        if cfg.draw_collar {
            out.push(Prim {
                geom: Geom::Text {
                    at: loc([c.x, c.y, c.z + 2.0 * h]),
                    height: h,
                    value: id.clone(),
                },
                layer: LAYER_COLLAR.into(),
                color: None,
                tag: Tag::new(Class::Collar)
                    .with("hole", id)
                    .with("x", c.x)
                    .with("y", c.y)
                    .with("z", c.z)
                    .with("max_depth", c.max_depth),
            });
        }
        if cfg.draw_assays && !cfg.assay_attr.is_empty() {
            let range = global.or_else(|| attribute_range(&hole.assays, &cfg.assay_attr));
            for m in &hole.assays {
                let value = m.attributes.get(&cfg.assay_attr).copied().flatten();
                let color = match (m.is_synthetic, value, range) {
                    (false, Some(v), Some((lo, hi))) => grade_color(scheme, v, lo, hi, &custom),
                    (false, Some(v), None) if scheme == ColorScheme::Custom => {
                        grade_color(scheme, v, 0.0, 0.0, &custom)
                    }
                    _ => GRAY,
                };
                let mut tag = Tag::new(Class::Assay)
                    .with("hole", id)
                    .with("from", m.from)
                    .with("to", m.to)
                    .with("attr", &cfg.assay_attr);
                if let Some(v) = value {
                    tag = tag.with("value", v);
                }
                if m.is_synthetic {
                    tag = tag.with("synthetic", 1);
                }
                out.push(Prim {
                    geom: Geom::Line(loc(m.from_xyz), loc(m.to_xyz)),
                    layer: assay_layer.clone(),
                    color: Some(color),
                    tag,
                });
                if cfg.draw_labels && !m.is_synthetic {
                    if let Some(v) = value {
                        out.push(Prim {
                            geom: Geom::Text {
                                at: loc(m.centroid_xyz),
                                height: h,
                                value: format!("{}: {v:.3}", cfg.assay_attr),
                            },
                            layer: assay_layer.clone(),
                            color: Some(color),
                            tag: Tag::new(Class::Label)
                                .with("hole", id)
                                .with("from", m.from)
                                .with("to", m.to)
                                .with("attr", &cfg.assay_attr),
                        });
                    }
                }
            }
        }
        if cfg.draw_geo && !cfg.geo_attr.is_empty() {
            for m in hole.geo.iter().filter(|m| !m.is_synthetic) {
                let Some(value) = m.geo_attributes.get(&cfg.geo_attr).filter(|v| !v.is_empty()) else {
                    continue;
                };
                let color = category_color(&cfg.geo_colors, value);
                out.push(Prim {
                    geom: Geom::Line(loc(m.from_xyz), loc(m.to_xyz)),
                    layer: geo_layer.clone(),
                    color: Some(color),
                    tag: Tag::new(Class::Geo)
                        .with("hole", id)
                        .with("from", m.from)
                        .with("to", m.to)
                        .with("attr", &cfg.geo_attr)
                        .with("value", value)
                        .with("lithology", &m.lithology),
                });
                if cfg.draw_labels {
                    out.push(Prim {
                        geom: Geom::Text {
                            at: loc(m.centroid_xyz),
                            height: h,
                            value: value.clone(),
                        },
                        layer: geo_layer.clone(),
                        color: Some(color),
                        tag: Tag::new(Class::Label)
                            .with("hole", id)
                            .with("from", m.from)
                            .with("to", m.to)
                            .with("attr", &cfg.geo_attr),
                    });
                }
            }
        }
    }
    out
}

/// Replaces the drawing of every loaded hole (no duplicates on redraw).
pub fn draw(sink: &mut dyn CadSink, s: &Strings, cfg: &ProjectConfig, data: &DataSet) -> usize {
    if cfg.dh.scheme() == ColorScheme::Custom && cfg.dh.draw_assays && !cfg.dh.custom_ranges_valid() {
        sink.error(s.m_custom_ranges_invalid);
        return 0;
    }
    let holes = build_holes(data);
    let ids: HashSet<String> = holes.iter().map(|h| hole_key(&h.collar.hole_id)).collect();
    let prims = scene(&holes, &cfg.dh, origin(cfg));
    sink.begin("MIN_DH_DRAW");
    sink.ensure_layers(&layers(&cfg.dh));
    let removed =
        sink.remove_where(&|t: &Tag| t.class.is_drillhole() && t.hole().is_some_and(|h| ids.contains(&hole_key(h))));
    let created = sink.add(prims);
    sink.end();
    if removed > 0 {
        sink.info(&fmt(s.m_removed, &[&removed]));
    }
    sink.info(&fmt(s.m_drawn, &[&holes.len(), &created]));
    created
}

/// Removes every drillhole entity of the plugin.
pub fn clear(sink: &mut dyn CadSink, s: &Strings) -> usize {
    sink.begin("MIN_DH_CLEAR");
    let n = sink.remove_where(&|t: &Tag| t.class.is_drillhole());
    sink.end();
    sink.info(&fmt(s.m_removed, &[&n]));
    n
}

/// Distinct values of the selected geology attribute with default colours
/// (existing assignments kept).
pub fn geo_color_table(data: &DataSet, cfg: &DrillholeConfig) -> BTreeMap<String, String> {
    const PALETTE: [&str; 11] = [
        "Rojo",
        "Amarillo",
        "Verde",
        "Cian",
        "Azul",
        "Magenta",
        "Naranja",
        "Verde lima",
        "Azul cielo",
        "Rosa",
        "Blanco",
    ];
    let values: BTreeSet<&String> = data
        .geo
        .iter()
        .filter_map(|g| g.attributes.get(&cfg.geo_attr))
        .collect();
    let existing: HashMap<String, &String> = cfg.geo_colors.iter().map(|(k, v)| (k.to_lowercase(), v)).collect();
    values
        .into_iter()
        .enumerate()
        .map(|(i, v)| {
            (
                v.clone(),
                existing
                    .get(&v.to_lowercase())
                    .map_or_else(|| PALETTE[i % PALETTE.len()].to_string(), |c| (*c).clone()),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cad::mock::MockSink;
    use crate::i18n::{for_lang, Lang};
    use drillhole_engine::{Assay, Collar, GeoLog, Survey};

    fn data() -> DataSet {
        DataSet {
            collars: vec![
                Collar {
                    hole_id: "DH1".into(),
                    x: 500_100.0,
                    y: 8_500_200.0,
                    z: 4_000.0,
                    max_depth: 30.0,
                    azimuth: None,
                    dip: None,
                },
                Collar {
                    hole_id: "DH2".into(),
                    x: 500_200.0,
                    y: 8_500_200.0,
                    z: 4_000.0,
                    max_depth: 30.0,
                    azimuth: None,
                    dip: None,
                },
            ],
            surveys: vec![Survey {
                hole_id: "DH1".into(),
                depth: 0.0,
                azimuth: 0.0,
                dip: -90.0,
            }],
            assays: vec![
                Assay {
                    hole_id: "DH1".into(),
                    from: 0.0,
                    to: 10.0,
                    attributes: BTreeMap::from([("cu".into(), Some(0.1))]),
                },
                Assay {
                    hole_id: "DH1".into(),
                    from: 20.0,
                    to: 30.0,
                    attributes: BTreeMap::from([("cu".into(), Some(2.0))]),
                },
                Assay {
                    hole_id: "DH2".into(),
                    from: 0.0,
                    to: 10.0,
                    attributes: BTreeMap::from([("cu".into(), Some(1.0))]),
                },
            ],
            geo: vec![GeoLog {
                hole_id: "DH1".into(),
                from: 0.0,
                to: 30.0,
                attributes: BTreeMap::from([("rock".into(), "AND".into())]),
            }],
        }
    }

    fn cfg() -> ProjectConfig {
        let mut c = ProjectConfig::default();
        c.dh.assay_attr = "cu".into();
        c.dh.geo_attr = "rock".into();
        c.dh.draw_geo = true;
        c
    }

    #[test]
    fn redraw_replaces_instead_of_duplicating() {
        let (s, c, d) = (for_lang(Lang::Es), cfg(), data());
        let mut sink = MockSink::default();
        let first = draw(&mut sink, s, &c, &d);
        let after_first = sink.entities.len();
        draw(&mut sink, s, &c, &d);
        assert_eq!(sink.entities.len(), after_first);
        assert_eq!(first, after_first);
        assert_eq!(sink.undo_depth, 0, "undo groups balanced");
        assert_eq!(sink.undo_steps, 2, "one undo step per draw");
        assert_eq!(clear(&mut sink, s), after_first);
        assert!(sink.entities.is_empty());
    }

    #[test]
    fn scene_content_tags_and_colours() {
        let c = cfg();
        let holes = build_holes(&data());
        let prims = scene(&holes, &c.dh, LocalOrigin::default());
        let count = |cl: Class| prims.iter().filter(|p| p.tag.class == cl).count();
        assert_eq!(count(Class::Trace), 2);
        assert_eq!(count(Class::Collar), 2);
        assert_eq!(count(Class::Assay), 4, "3 assays + 1 gap on DH1");
        assert_eq!(count(Class::Geo), 1);
        // Global scale: DH1 0.1 is lowest (blue), 2.0 highest (red); the gap is gray.
        let assay = |hole: &str, from: &str| {
            prims
                .iter()
                .find(|p| p.tag.class == Class::Assay && p.tag.hole() == Some(hole) && p.tag.get("from") == Some(from))
                .unwrap()
        };
        assert_eq!(assay("DH1", "0").color, Some(5));
        assert_eq!(assay("DH1", "20").color, Some(1));
        assert_eq!(assay("DH1", "10").color, Some(GRAY));
        assert_eq!(assay("DH1", "10").tag.get("synthetic"), Some("1"));
    }

    #[test]
    fn local_origin_shifts_geometry() {
        let mut c = cfg();
        c.origin.enabled = true;
        assert!(auto_origin(&mut c, &data()));
        assert_eq!((c.origin.x, c.origin.y), (500_000.0, 8_500_000.0));
        let prims = scene(&build_holes(&data()), &c.dh, origin(&c));
        let Geom::Polyline3d(pts) = &prims[0].geom else {
            panic!()
        };
        assert_eq!(pts[0], [100.0, 200.0, 4000.0]);
        // XDATA keeps world coordinates.
        let collar = prims.iter().find(|p| p.tag.class == Class::Collar).unwrap();
        assert_eq!(collar.tag.get("x"), Some("500100"));
    }

    #[test]
    fn invalid_custom_ranges_block_drawing() {
        let mut c = cfg();
        c.dh.scheme = 3;
        let mut sink = MockSink::default();
        assert_eq!(draw(&mut sink, for_lang(Lang::En), &c, &data()), 0);
        assert_eq!(sink.errors.len(), 1);
    }

    #[test]
    fn geo_colors_keep_existing_choices() {
        let mut c = cfg().dh;
        c.geo_colors.insert("and".into(), "Azul".into());
        let t = geo_color_table(&data(), &c);
        assert_eq!(t.get("AND").map(String::as_str), Some("Azul"));
    }

    #[test]
    fn load_asks_for_mapping_and_remembers_it() {
        let dir = std::env::temp_dir().join(format!("om_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let collar = dir.join("collar.csv");
        std::fs::write(&collar, "POZO;E;N;COTA;PROF\nA;1;2;3;10\n").unwrap();
        let mut c = ProjectConfig::default();
        c.dh.collar = collar.to_string_lossy().into();
        let mut asked = 0;
        let mut ask = |_k: TableKind, headers: &[String], req: &[Field], _o: &[Field]| {
            asked += 1;
            Some(
                req.iter()
                    .zip(headers)
                    .map(|(f, h)| (f.key().to_string(), h.clone()))
                    .collect(),
            )
        };
        let loaded = load(for_lang(Lang::Es), &mut c, &mut ask).unwrap();
        assert_eq!(loaded.data.collars.len(), 1);
        assert_eq!(asked, 1);
        assert_eq!(c.dh.column_maps["collar"]["x"], "E");
        // Second load uses the stored mapping without asking.
        let mut never = |_: TableKind, _: &[String], _: &[Field], _: &[Field]| -> Option<BTreeMap<String, String>> {
            panic!("asked again")
        };
        assert!(load(for_lang(Lang::Es), &mut c, &mut never).is_ok());
        std::fs::remove_dir_all(dir).ok();
    }
}
