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

/// Stores the setting arguments of `def` in `cfg` and shows them in the panels.
pub fn apply_settings(host: &mut dyn HostApi, def: &OpDef, args: &Args, cfg: &mut ProjectConfig) {
    if !def
        .params
        .iter()
        .any(|p| p.setting.is_some() && args.contains_key(p.key))
    {
        return;
    }
    let mut root = crate::opsbind::settings_json(cfg);
    ops::settings::apply(def, args, &mut root);
    crate::opsbind::from_json(cfg, root);
    crate::opsbind::push_all(host, cfg);
}

fn handler(host: &mut dyn HostApi, def: &OpDef, args: &Args, tab: u64, cfg: &mut ProjectConfig) -> Outcome {
    match def.group {
        ops::Group::Geotechnics => crate::gtcommands::run(host, def.name, args, tab, cfg),
        _ => Err(OpError::new(
            ErrorCode::UnknownCommand,
            format!("{} has no handler", def.name),
        )),
    }
}

/// Runs `line` when its command is an operation; `false` otherwise.
pub fn try_dispatch(host: &mut dyn HostApi, line_text: &str, tab: u64, cfg: &mut ProjectConfig) -> bool {
    let name = line_text.split_whitespace().next().unwrap_or("");
    let Some(def) = ops::find(name)
        .filter(|d| d.command.eq_ignore_ascii_case(name) || d.aliases.iter().any(|a| a.eq_ignore_ascii_case(name)))
    else {
        return false;
    };
    let r = parse_command(def, line_text).and_then(|args| {
        apply_settings(host, def, &args, cfg);
        handler(host, def, &args, tab, cfg)
    });
    emit(host, def.name, &r);
    true
}
