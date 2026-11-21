//! Channel-sample operations (`ch_convert`, `ch_orient`): the sampling sheet
//! becomes drillhole data appended to the loaded drillholes.

use drillhole_engine::channels::{self, Anchor, ChError, Converted, Options};
use drillhole_engine::error::TableKind;
use drillhole_engine::model::{hole_key, DataSet};
use drillhole_engine::table::RawTable;
use ocs_plugin_api::host::HostApi;
use openmining_ops::parse::Args;
use openmining_ops::result::{ErrorCode, OpError, OpResult};
use serde_json::{json, Value};

use crate::cad::CadSink;
use crate::config::ProjectConfig;
use crate::host_sink::HostSink;
use crate::i18n::{fmt, lang, Lang};
use crate::interactive::PickTwoPoints;
use crate::opsrun::Outcome;
use crate::session;
use crate::state::{self, Pending};

pub const PANEL: &str = "min_channels";
const STATUS: &str = "ch_status";

pub struct ChStrings {
    pub tool: &'static str,
    pub m_no_files: &'static str,
    pub m_done: &'static str,
    pub m_issue: &'static str,
    pub m_files: &'static str,
    pub m_none: &'static str,
    pub m_orient: &'static str,
    pub m_pick: [&'static str; 2],
    pub m_need_channel: &'static str,
    pub m_auto: &'static str,
}

static ES: ChStrings = ChStrings {
    tool: "Canales",
    m_no_files: "Canales: indique la tabla de puntos topográficos y la hoja de muestreo.",
    m_done: "Canales: {0} convertidos ({1} muestras); sondajes cargados: {2}.",
    m_issue: "Canal {0} (fila {1}): {2}",
    m_files: "Tablas tipo sondaje guardadas: {0}",
    m_none: "Ningún canal pudo convertirse.",
    m_orient: "Canal {0}: azimut {1}°, inclinación {2}°.",
    m_pick: [
        "Primer punto de la dirección del canal",
        "Segundo punto de la dirección del canal",
    ],
    m_need_channel: "Indique el canal a orientar.",
    m_auto: "Canales agregados de nuevo a los sondajes ({0}).",
};

static EN: ChStrings = ChStrings {
    tool: "Channels",
    m_no_files: "Channels: choose the topographic points table and the sampling sheet.",
    m_done: "Channels: {0} converted ({1} samples); drillholes loaded: {2}.",
    m_issue: "Channel {0} (row {1}): {2}",
    m_files: "Drillhole-format tables saved: {0}",
    m_none: "No channel could be converted.",
    m_orient: "Channel {0}: azimuth {1}°, inclination {2}°.",
    m_pick: [
        "First point of the channel direction",
        "Second point of the channel direction",
    ],
    m_need_channel: "Give the channel to orient.",
    m_auto: "Channels appended again to the drillholes ({0}).",
};

static PT: ChStrings = ChStrings {
    tool: "Canais",
    m_no_files: "Canais: indique a tabela de pontos topográficos e a planilha de amostragem.",
    m_done: "Canais: {0} convertidos ({1} amostras); furos carregados: {2}.",
    m_issue: "Canal {0} (linha {1}): {2}",
    m_files: "Tabelas tipo furo salvas: {0}",
    m_none: "Nenhum canal pôde ser convertido.",
    m_orient: "Canal {0}: azimute {1}°, inclinação {2}°.",
    m_pick: [
        "Primeiro ponto da direção do canal",
        "Segundo ponto da direção do canal",
    ],
    m_need_channel: "Indique o canal a orientar.",
    m_auto: "Canais adicionados de novo aos furos ({0}).",
};

pub fn c() -> &'static ChStrings {
    match lang() {
        Lang::Es => &ES,
        Lang::En => &EN,
        Lang::Pt => &PT,
    }
}

fn failed(code: ErrorCode, msg: impl Into<String>) -> OpError {
    OpError::new(code, msg)
}

fn options(cfg: &ProjectConfig) -> Options {
    let x = &cfg.ch;
    Options {
        prefix: x.prefix.clone(),
        anchor: if x.anchor == 1 { Anchor::Start } else { Anchor::Centre },
        height: x.height,
        azimuth_offset: x.azimuth_offset,
        inclination: x.inclination,
        overrides: x
            .orient
            .iter()
            .map(|o| (o.channel.trim().to_lowercase(), (o.azimuth, o.inclination)))
            .collect(),
    }
}

/// Reads and converts the configured files (pure apart from reading them).
pub fn convert_files(cfg: &ProjectConfig) -> Result<(channels::Sheet, Converted), OpError> {
    let s = c();
    let x = &cfg.ch;
    if x.points.trim().is_empty() || x.sheet.trim().is_empty() {
        return Err(
            failed(ErrorCode::MissingArgument, s.m_no_files).at(if x.points.trim().is_empty() {
                "points"
            } else {
                "sheet"
            }),
        );
    }
    let read = |p: &str| {
        RawTable::from_path(std::path::Path::new(p.trim()), TableKind::Assay)
            .map_err(|e| failed(ErrorCode::NotFound, crate::i18n::s().import_error(&e)))
    };
    let points = channels::parse_points(&read(&x.points)?)
        .map_err(|e| failed(ErrorCode::MissingColumn, e.to_string()).at("points"))?;
    let sheet = channels::parse_sheet(&read(&x.sheet)?, &x.column_map).map_err(|e| {
        let code = if matches!(e, ChError::MissingColumns { .. }) {
            ErrorCode::MissingColumn
        } else {
            ErrorCode::InvalidArgument
        };
        failed(code, e.to_string()).at("sheet")
    })?;
    let conv = channels::convert(&sheet, &points, &options(cfg));
    Ok((sheet, conv))
}

/// Replaces the channels of a data set (ids with the prefix) by `conv`.
pub fn merge(data: Option<DataSet>, conv: &Converted, prefix: &str) -> DataSet {
    let mut d = data.unwrap_or_default();
    let p = prefix.trim().to_lowercase();
    let is_channel = |id: &str| !p.is_empty() && hole_key(id).starts_with(&p);
    d.collars.retain(|x| !is_channel(&x.hole_id));
    d.surveys.retain(|x| !is_channel(&x.hole_id));
    d.assays.retain(|x| !is_channel(&x.hole_id));
    d.geo.retain(|x| !is_channel(&x.hole_id));
    d.collars.extend(conv.collars.iter().cloned());
    d.surveys.extend(conv.surveys.iter().cloned());
    d.assays.extend(conv.assays.iter().cloned());
    d.geo.extend(conv.geo.iter().cloned());
    d
}

fn write_files(host: &dyn HostApi, conv: &Converted) -> Result<Option<Vec<String>>, String> {
    let Some(dwg) = host.document_path(host.tab_id()) else {
        return Ok(None);
    };
    let (collar, survey, assays, geo) = channels::to_csv(conv);
    let mut out = Vec::new();
    for (suffix, text) in [
        ("_canales_collar.csv", collar),
        ("_canales_survey.csv", survey),
        ("_canales_leyes.csv", assays),
        ("_canales_geo.csv", geo),
    ] {
        let Some(path) = crate::resources::model_file(Some(&dwg), "", suffix) else {
            continue;
        };
        std::fs::write(&path, text).map_err(|e| e.to_string())?;
        out.push(path.display().to_string());
    }
    Ok(Some(out))
}

fn convert(host: &mut dyn HostApi, tab: u64, cfg: &ProjectConfig) -> Outcome {
    let s = c();
    let (sheet, conv) = convert_files(cfg)?;
    let mut lines = Vec::new();
    for i in &conv.issues {
        let l = fmt(s.m_issue, &[&i.channel, &i.row, &i.message]);
        host.push_error(&l);
        lines.push(l);
    }
    if conv.collars.is_empty() {
        return Err(failed(ErrorCode::NoData, s.m_none));
    }
    let data = merge(state::with_tab(tab, |t| t.data.clone()), &conv, &cfg.ch.prefix);
    let holes = data.collars.len();
    let mut warnings = lines.clone();
    let files = if cfg.ch.write_files {
        match write_files(host, &conv) {
            Ok(f) => f,
            Err(e) => {
                host.push_error(&e);
                warnings.push(e);
                None
            }
        }
    } else {
        None
    };
    let done = fmt(s.m_done, &[&conv.collars.len(), &conv.samples, &holes]);
    host.push_info(&done);
    if let Some(f) = &files {
        host.push_info(&fmt(s.m_files, &[&f.join(", ")]));
    }
    state::with_tab(tab, |t| t.data = Some(data));
    let mut sink = HostSink::new(host);
    sink.begin("MIN_CH_CONVERT");
    session::commit(&mut sink, crate::i18n::s(), tab, cfg);
    sink.end();
    session::push_panels(host, cfg);
    let mut status = vec![done];
    status.extend(lines);
    crate::opsbind::push_one(host, cfg, PANEL, Some((STATUS, status.join("\n"))));
    let issues: Vec<Value> = conv
        .issues
        .iter()
        .map(|i| json!({"channel": i.channel, "row": i.row, "message": i.message}))
        .collect();
    Ok(OpResult {
        warnings,
        ..OpResult::data(json!({
            "channels": conv.collars.len(), "samples": conv.samples, "issues": issues,
            "grade_columns": sheet.grade_columns, "text_columns": sheet.text_columns, "holes": holes, "files": files,
        }))
    })
}

/// After `dh_load`: converts and appends the channels again when configured.
pub fn reappend(host: &mut dyn HostApi, tab: u64, cfg: &ProjectConfig) -> Option<String> {
    if !cfg.ch.auto || cfg.ch.sheet.trim().is_empty() || cfg.ch.points.trim().is_empty() {
        return None;
    }
    let (_, conv) = convert_files(cfg).ok()?;
    if conv.collars.is_empty() {
        return None;
    }
    let data = merge(state::with_tab(tab, |t| t.data.clone()), &conv, &cfg.ch.prefix);
    state::with_tab(tab, |t| t.data = Some(data));
    let line = fmt(c().m_auto, &[&conv.collars.len()]);
    host.push_info(&line);
    Some(line)
}

fn orient_with(
    host: &mut dyn HostApi,
    tab: u64,
    cfg: &mut ProjectConfig,
    a: mining_core::Vec3,
    b: mining_core::Vec3,
) -> Outcome {
    let s = c();
    let ch = cfg.ch.target.trim().to_string();
    if ch.is_empty() {
        return Err(failed(ErrorCode::MissingArgument, s.m_need_channel).at("channel"));
    }
    let (dx, dy, dz) = (b[0] - a[0], b[1] - a[1], b[2] - a[2]);
    let h = dx.hypot(dy);
    if h < 1e-9 && dz.abs() < 1e-9 {
        return Err(failed(ErrorCode::InvalidArgument, "p1 and p2 are the same point").at("p2"));
    }
    let az = if h < 1e-9 {
        0.0
    } else {
        dx.atan2(dy).to_degrees().rem_euclid(360.0)
    };
    let inc = dz.atan2(h).to_degrees();
    cfg.ch.orient.retain(|o| !o.channel.trim().eq_ignore_ascii_case(&ch));
    cfg.ch.orient.push(crate::chconfig::OrientCfg {
        channel: ch.clone(),
        azimuth: (az * 100.0).round() / 100.0,
        inclination: (inc * 100.0).round() / 100.0,
    });
    host.push_info(&fmt(s.m_orient, &[&ch, &format!("{az:.1}"), &format!("{inc:.1}")]));
    let converted = convert(host, tab, cfg)?;
    Ok(OpResult {
        data: json!({"channel": ch, "azimuth": az, "inclination": inc, "converted": converted.data}),
        ..converted
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

pub fn run(host: &mut dyn HostApi, name: &str, args: &Args, tab: u64, cfg: &mut ProjectConfig) -> Outcome {
    let s = c();
    match name {
        "ch_convert" => convert(host, tab, cfg),
        "ch_orient" => match (point(args, "p1"), point(args, "p2")) {
            (Some(a), Some(b)) => orient_with(host, tab, cfg, a, b),
            _ => {
                if cfg.ch.target.trim().is_empty() {
                    return Err(failed(ErrorCode::MissingArgument, s.m_need_channel).at("channel"));
                }
                state::with_tab(tab, |t| t.config = cfg.clone());
                host.start_interactive(Box::new(PickTwoPoints::new(
                    s.m_pick.map(str::to_string),
                    "MIN_CH_ORIENT_EXEC",
                )));
                Ok(OpResult::waiting(s.m_pick[0]))
            }
        },
        other => Err(OpError::new(
            ErrorCode::UnknownCommand,
            format!("unknown channel operation {other}"),
        )),
    }
}

/// Follow-up of the two-point pick of `ch_orient`.
pub fn follow_up(
    host: &mut dyn HostApi,
    command: &str,
    tab: u64,
    cfg: &mut ProjectConfig,
) -> Option<(&'static str, Outcome)> {
    if command != "MIN_CH_ORIENT_EXEC" {
        return None;
    }
    let Some(Pending::TwoPoints(a, b)) = state::take_pending() else {
        return None;
    };
    Some(("ch_orient", orient_with(host, tab, cfg, a, b)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use drillhole_engine::model::Collar;

    #[test]
    fn merging_replaces_earlier_channels_and_keeps_drillholes() {
        let hole = |id: &str| Collar {
            hole_id: id.into(),
            x: 0.0,
            y: 0.0,
            z: 0.0,
            max_depth: 10.0,
            azimuth: None,
            dip: None,
        };
        let data = DataSet {
            collars: vec![hole("DH-1"), hole("CH-OLD")],
            ..Default::default()
        };
        let conv = Converted {
            collars: vec![hole("CH-1"), hole("CH-2")],
            ..Default::default()
        };
        let d = merge(Some(data), &conv, "CH-");
        let ids: Vec<&str> = d.collars.iter().map(|c| c.hole_id.as_str()).collect();
        assert_eq!(ids, vec!["DH-1", "CH-1", "CH-2"]);
        // Without a prefix nothing is removed.
        assert_eq!(merge(Some(d.clone()), &conv, "").collars.len(), 5);
    }

    #[test]
    fn files_are_read_and_converted() {
        let dir = std::env::temp_dir().join(format!("om_ch_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("p.csv");
        let s = dir.join("s.csv");
        std::fs::write(&p, "PUNTO;ESTE;NORTE;COTA\nPT-1;1000;2000;3000\nPT-2;1000;2100;3000\n").unwrap();
        std::fs::write(
            &s,
            "CANAL;PUNTO;HACIA;DISTANCIA;ANCHO;POTENCIA;AU\nC1;PT-1;PT-2;10;0,5;0,5;4\nC2;PT-1;PT-2;20;0,7;0,7;6\n",
        )
        .unwrap();
        let mut cfg = ProjectConfig::default();
        assert_eq!(convert_files(&cfg).unwrap_err().code, ErrorCode::MissingArgument);
        cfg.ch.points = p.display().to_string();
        cfg.ch.sheet = s.display().to_string();
        let (sheet, conv) = convert_files(&cfg).unwrap();
        assert_eq!((conv.collars.len(), conv.samples, sheet.grade_columns.len()), (2, 2, 1));
        // An orientation set on screen replaces the default.
        cfg.ch.orient = vec![crate::chconfig::OrientCfg {
            channel: "c2".into(),
            azimuth: 30.0,
            inclination: 10.0,
        }];
        let (_, conv) = convert_files(&cfg).unwrap();
        assert_eq!((conv.collars[1].azimuth, conv.collars[1].dip), (Some(30.0), Some(10.0)));
        std::fs::remove_dir_all(dir).ok();
    }
}
