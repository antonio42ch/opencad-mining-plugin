//! Typed operations of Open Mining.
//!
//! Every action of the plugin is an [`OpDef`]: its command, a description
//! written for AI clients, typed parameters and how they bind to the project
//! settings. From that single definition come:
//!
//! * the `key=value` command-line parser ([`parse::parse_command`]) and the
//!   JSON argument reader ([`parse::args_from_json`]) used by native tools;
//! * the JSON Schema of the arguments ([`schema::input_schema`]);
//! * the panels (built by the plugin from [`PanelSpec`]);
//! * the MCP guide ([`doc::markdown`]).
//!
//! Results and errors have one machine-readable shape ([`result`]).

pub mod defs;
pub mod doc;
pub mod help;
pub mod parse;
pub mod result;
pub mod schema;
pub mod settings;
mod types;

pub use types::*;

/// Every operation, in documentation order.
pub fn all() -> Vec<&'static OpDef> {
    defs::all()
}

/// Operation by command name or alias (case-insensitive), or by op name.
pub fn find(name: &str) -> Option<&'static OpDef> {
    let up = name.trim().to_ascii_uppercase();
    all()
        .into_iter()
        .find(|d| d.command == up || d.aliases.iter().any(|a| *a == up) || d.name.eq_ignore_ascii_case(name.trim()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn names_commands_and_controls_are_unique() {
        let mut names = HashSet::new();
        let mut commands = HashSet::new();
        for d in all() {
            assert!(names.insert(d.name), "duplicate op {}", d.name);
            assert!(
                d.name
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
                "{}",
                d.name
            );
            assert!(d.command.starts_with("MIN_"), "{}", d.command);
            for c in std::iter::once(d.command).chain(d.aliases.iter().copied()) {
                assert!(commands.insert(c), "duplicate command {c}");
            }
            let mut keys = HashSet::new();
            for p in d.params {
                assert!(keys.insert(p.key), "{}: duplicate param {}", d.name, p.key);
                assert!(!p.help.is_empty(), "{}.{} has no help", d.name, p.key);
                if let Some(ptr) = p.setting {
                    assert!(ptr.starts_with('/'), "{}.{}", d.name, p.key);
                }
                assert!(
                    !(p.required && p.setting.is_some()),
                    "{}.{}: a setting always has a value",
                    d.name,
                    p.key
                );
            }
            for k in d.positional {
                assert!(d.param(k).is_some(), "{}: positional {k} is not a param", d.name);
            }
            assert!(!d.summary.is_empty() && !d.result.is_empty(), "{}", d.name);
        }
    }

    #[test]
    fn panels_reference_existing_params_and_ops() {
        for p in defs::panels() {
            p.visit(&mut |item| match item {
                Item::Param(op, key) => {
                    let d = find(op).unwrap_or_else(|| panic!("panel {}: op {op}", p.id));
                    let prm = d.param(key).unwrap_or_else(|| panic!("panel {}: {op}.{key}", p.id));
                    assert!(prm.setting.is_some(), "panel {}: {op}.{key} is not a setting", p.id);
                }
                Item::Button(op) => assert!(find(op).is_some(), "panel {}: button {op}", p.id),
                _ => {}
            });
        }
    }

    #[test]
    fn lookup() {
        assert_eq!(find("min_gt_load").unwrap().name, "gt_load");
        assert_eq!(find("gt_load").unwrap().command, "MIN_GT_LOAD");
        assert!(find("MIN_NOPE").is_none());
    }
}
