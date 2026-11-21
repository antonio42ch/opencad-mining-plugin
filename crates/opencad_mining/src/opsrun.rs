//! Runs operations: parse the command line against the definition, store
//! the setting arguments, call the module's handler and write the result
//! line (`OPENMINING_RESULT {json}`) to the command history.

use ocs_plugin_api::host::HostApi;
use openmining_ops::parse::{parse_command, Args};
use openmining_ops::result::{line, ErrorCode, OpError, OpResult};
use openmining_ops::{self as ops, OpDef};

use crate::config::ProjectConfig;

pub type Outcome = Result<OpResult, OpError>;

/// Writes the outcome: the error message (handlers print their own info
/// lines) and the result line.
pub fn emit(host: &mut dyn HostApi, op: &str, r: &Outcome) {
    if let Err(e) = r {
        host.push_error(&e.message);
    }
    host.push_info(&line(op, r));
}

/// Replaces names given to `Index` parameters (a domain by name) by their
/// index in the run-time list.
fn resolve_names(def: &OpDef, args: &mut Args, lists: &crate::opsbind::Lists) -> Result<(), OpError> {
    let unknown = |key: &str, name: &str, list: &[String]| {
        OpError::new(
            ErrorCode::NotFound,
            format!("{key}: '{name}' is not one of {}", list.join(", ")),
        )
        .at(key)
    };
    for p in def.params {
        let (source, all) = match p.kind {
            ops::Kind::Index { source, all } => (source, Some(all)),
            // A name is matched without case (attributes are stored lower-case);
            // unknown names are refused when the list is known.
            ops::Kind::Name { source } => (source, None),
            _ => continue,
        };
        let Some(name) = args.get(p.key).and_then(|v| v.as_str()).map(str::to_string) else {
            continue;
        };
        let list = lists.get(source);
        match all {
            None if list.is_empty() => {}
            None => {
                let found = list
                    .iter()
                    .find(|n| n.eq_ignore_ascii_case(name.trim()))
                    .ok_or_else(|| unknown(p.key, &name, list))?;
                args.insert(p.key.to_string(), serde_json::json!(found));
            }
            Some(all) => {
                let pos = list
                    .iter()
                    .position(|n| n.eq_ignore_ascii_case(name.trim()))
                    .ok_or_else(|| unknown(p.key, &name, list))?;
                args.insert(p.key.to_string(), serde_json::json!(pos + usize::from(all.is_some())));
            }
        }
    }
    Ok(())
}

/// Stores the setting arguments of `def` in `cfg` (table arguments through
/// their codec) and shows them in the panels.
pub fn apply_settings(
    host: &mut dyn HostApi,
    def: &OpDef,
    args: &mut Args,
    tab: u64,
    cfg: &mut ProjectConfig,
) -> Result<(), OpError> {
    if !def
        .params
        .iter()
        .any(|p| p.setting.is_some() && args.contains_key(p.key))
    {
        return Ok(());
    }
    resolve_names(def, args, &crate::opsbind::Lists::of(tab, cfg))?;
    let mut root = crate::opsbind::settings_json(cfg);
    for p in def.params {
        if let (ops::Kind::Table { codec, .. }, Some(ptr), Some(v)) = (p.kind, p.setting, args.get(p.key)) {
            let value = crate::opsbind::table_json(codec, v).ok_or_else(|| {
                OpError::new(ErrorCode::InvalidArgument, format!("{}: unreadable table", p.key)).at(p.key)
            })?;
            ops::settings::set(&mut root, ptr, value);
        }
    }
    let plain: Args = args
        .iter()
        .filter(|(k, _)| !matches!(def.param(k).map(|p| p.kind), Some(ops::Kind::Table { .. })))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    ops::settings::apply(def, &plain, &mut root);
    crate::opsbind::from_json(cfg, root);
    crate::opsbind::push_all(host, cfg);
    Ok(())
}

fn handler(host: &mut dyn HostApi, def: &OpDef, args: &Args, tab: u64, cfg: &mut ProjectConfig) -> Outcome {
    if def.kind == ops::OpKind::Query {
        return crate::queries::run(host, def.name, args, tab, cfg);
    }
    match def.group {
        ops::Group::Geotechnics if def.name.starts_with("st_") || def.name.starts_with("sm_") => {
            crate::rmcommands::run(host, def.name, args, tab, cfg)
        }
        ops::Group::Geotechnics => crate::gtcommands::run(host, def.name, args, tab, cfg),
        ops::Group::Resources => crate::rscommands::run(host, def.name, args, tab, cfg),
        _ => crate::commands::run(host, def.name, args, tab, cfg),
    }
}

/// Runs the operation `name` with JSON arguments (native tools).
#[cfg_attr(not(feature = "api25"), allow(dead_code))]
pub fn run_json(
    host: &mut dyn HostApi,
    name: &str,
    value: &serde_json::Value,
    tab: u64,
    cfg: &mut ProjectConfig,
) -> Outcome {
    let def =
        ops::find(name).ok_or_else(|| OpError::new(ErrorCode::UnknownCommand, format!("unknown tool '{name}'")))?;
    let mut args = ops::parse::args_from_json(def, value)?;
    apply_settings(host, def, &mut args, tab, cfg)?;
    let r = handler(host, def, &args, tab, cfg);
    if let Err(e) = &r {
        host.push_error(&e.message);
    }
    r
}

/// Runs `line` when its command is an operation; `false` otherwise.
pub fn try_dispatch(host: &mut dyn HostApi, line_text: &str, tab: u64, cfg: &mut ProjectConfig) -> bool {
    let name = line_text.split_whitespace().next().unwrap_or("");
    let Some(def) = ops::find(name)
        .filter(|d| d.command.eq_ignore_ascii_case(name) || d.aliases.iter().any(|a| a.eq_ignore_ascii_case(name)))
    else {
        return false;
    };
    let r = parse_command(def, line_text).and_then(|mut args| {
        apply_settings(host, def, &mut args, tab, cfg)?;
        handler(host, def, &args, tab, cfg)
    });
    emit(host, def.name, &r);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn domain_names_become_indices() {
        let lists = crate::opsbind::Lists {
            domains: vec!["Óxidos".into(), "Sulfuros".into()],
            ..Default::default()
        };
        let view = ops::find("rs_bmview").unwrap();
        let mut args = parse_command(view, "MIN_RS_BMVIEW domain=sulfuros").unwrap();
        resolve_names(view, &mut args, &lists).unwrap();
        assert_eq!(args["domain"], json!(2), "0 is 'all'");
        let vario = ops::find("rs_vario").unwrap();
        let mut args = parse_command(vario, "MIN_RS_VARIO domain=Sulfuros").unwrap();
        resolve_names(vario, &mut args, &lists).unwrap();
        assert_eq!(args["domain"], json!(1));
        let mut args = parse_command(vario, "MIN_RS_VARIO domain=Nada").unwrap();
        assert_eq!(
            resolve_names(vario, &mut args, &lists).unwrap_err().code,
            ErrorCode::NotFound
        );
        let draw = ops::find("dh_draw").unwrap();
        let lists = crate::opsbind::Lists {
            assay: vec!["au".into(), "cu%".into()],
            ..Default::default()
        };
        let mut args = parse_command(draw, "MIN_DH_DRAW assay_attr=CU%").unwrap();
        resolve_names(draw, &mut args, &lists).unwrap();
        assert_eq!(args["assay_attr"], json!("cu%"));
        let mut args = parse_command(draw, "MIN_DH_DRAW assay_attr=ZN").unwrap();
        assert_eq!(
            resolve_names(draw, &mut args, &lists).unwrap_err().code,
            ErrorCode::NotFound
        );
    }
}
