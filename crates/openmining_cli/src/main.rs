//! `openmining` — the drillhole engine without the CAD.
//!
//! ```text
//! openmining validate  --collar C [--survey S] [--assays A] [--geo G]
//! openmining desurvey  --collar C [--survey S] [--out trace.csv]
//! openmining intervals --collar C [--survey S] --assays A [--out intervals.csv]
//! ```

use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use drillhole_engine::error::TableKind;
use drillhole_engine::import::{self, ColumnMap};
use drillhole_engine::pipeline::build_holes;
use drillhole_engine::table::RawTable;
use drillhole_engine::validate::validate;
use drillhole_engine::{DataSet, Severity};

const USAGE: &str = "usage: openmining <validate|desurvey|intervals> --collar FILE [--survey FILE] [--assays FILE] [--geo FILE] [--out FILE]";

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(2)
        }
    }
}

fn run(args: Vec<String>) -> Result<ExitCode, String> {
    let (cmd, rest) = args.split_first().ok_or(USAGE)?;
    let mut opts: HashMap<String, String> = HashMap::new();
    let mut it = rest.iter();
    while let Some(k) = it.next() {
        let key = k
            .strip_prefix("--")
            .ok_or_else(|| format!("unexpected argument '{k}'\n{USAGE}"))?;
        let v = it.next().ok_or_else(|| format!("missing value for --{key}"))?;
        opts.insert(key.to_string(), v.clone());
    }
    let ds = load(&opts)?;
    let (clean, msgs) = validate(&ds);
    let fatal = msgs.iter().any(|m| m.severity >= Severity::Error);
    for m in &msgs {
        eprintln!("[{}] {:?} {}: {}", m.rule, m.severity, m.hole_id, m.message);
    }
    let mut out: Box<dyn Write> = match opts.get("out") {
        Some(p) => Box::new(std::fs::File::create(p).map_err(|e| format!("{p}: {e}"))?),
        None => Box::new(std::io::stdout()),
    };
    let w = |out: &mut Box<dyn Write>, s: String| writeln!(out, "{s}").map_err(|e| e.to_string());
    match cmd.as_str() {
        "validate" => {
            eprintln!(
                "{} collars, {} surveys, {} assays, {} geo, {} messages",
                clean.collars.len(),
                clean.surveys.len(),
                clean.assays.len(),
                clean.geo.len(),
                msgs.len()
            );
            return Ok(if fatal { ExitCode::from(1) } else { ExitCode::SUCCESS });
        }
        "desurvey" => {
            w(&mut out, "hole_id,md,x,y,z,azimuth,dip,synthetic".into())?;
            for h in build_holes(&clean) {
                for n in &h.trace {
                    w(
                        &mut out,
                        format!(
                            "{},{},{},{},{},{},{},{}",
                            h.collar.hole_id, n.md, n.xyz[0], n.xyz[1], n.xyz[2], n.azimuth, n.dip, n.is_synthetic
                        ),
                    )?;
                }
            }
        }
        "intervals" => {
            let attrs = import::assay_attributes(&clean.assays);
            w(
                &mut out,
                format!(
                    "hole_id,from,to,x_from,y_from,z_from,x_to,y_to,z_to,synthetic,{}",
                    attrs.join(",")
                ),
            )?;
            for h in build_holes(&clean) {
                for m in &h.assays {
                    let vals: Vec<String> = attrs
                        .iter()
                        .map(|a| {
                            m.attributes
                                .get(a)
                                .copied()
                                .flatten()
                                .map_or(String::new(), |v| v.to_string())
                        })
                        .collect();
                    let (f, t) = (m.from_xyz, m.to_xyz);
                    w(
                        &mut out,
                        format!(
                            "{},{},{},{},{},{},{},{},{},{},{}",
                            m.hole_id,
                            m.from,
                            m.to,
                            f[0],
                            f[1],
                            f[2],
                            t[0],
                            t[1],
                            t[2],
                            m.is_synthetic,
                            vals.join(",")
                        ),
                    )?;
                }
            }
        }
        other => return Err(format!("unknown command '{other}'\n{USAGE}")),
    }
    Ok(ExitCode::SUCCESS)
}

fn load(opts: &HashMap<String, String>) -> Result<DataSet, String> {
    let read = |k: &str, kind: TableKind| -> Result<Option<RawTable>, String> {
        opts.get(k)
            .map(|p| RawTable::from_path(&PathBuf::from(p), kind).map_err(|e| e.to_string()))
            .transpose()
    };
    let m = ColumnMap::new();
    let collar = read("collar", TableKind::Collar)?.ok_or(USAGE)?;
    let mut ds = DataSet {
        collars: import::parse_collars(&collar, &m).map_err(|e| e.to_string())?,
        ..Default::default()
    };
    if let Some(t) = read("survey", TableKind::Survey)? {
        ds.surveys = import::parse_surveys(&t, &m).map_err(|e| e.to_string())?;
    }
    if let Some(t) = read("assays", TableKind::Assay)? {
        ds.assays = import::parse_assays(&t, &m).map_err(|e| e.to_string())?;
    }
    if let Some(t) = read("geo", TableKind::Geo)? {
        ds.geo = import::parse_geo(&t, &m).map_err(|e| e.to_string())?;
    }
    Ok(ds)
}
