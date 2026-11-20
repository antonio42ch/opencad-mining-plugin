//! The MCP guide (`docs/MCP.md`), generated from the definitions.

use std::fmt::Write;

use crate::result::{ErrorCode, PREFIX};
use crate::{Group, Kind, OpDef, OpKind};

fn kind_detail(k: &Kind) -> String {
    match k {
        Kind::Number { min, max, .. } => match (min, max) {
            (Some(a), Some(b)) => format!("number {a}..{b}"),
            (Some(a), None) => format!("number ≥ {a}"),
            (None, Some(b)) => format!("number ≤ {b}"),
            _ => "number".into(),
        },
        Kind::Integer { min, max } => match (min, max) {
            (Some(a), Some(b)) => format!("integer {a}..{b}"),
            (Some(a), None) => format!("integer ≥ {a}"),
            _ => "integer".into(),
        },
        Kind::Choice { options } => format!(
            "one of `{}`",
            options.iter().map(|o| o.key).collect::<Vec<_>>().join("`, `")
        ),
        Kind::Point => "point `x,y[,z]`".into(),
        Kind::Handle => "entity handle (hex)".into(),
        Kind::Handles => "handles `1A,2B`".into(),
        Kind::Path { .. } => "file path".into(),
        Kind::Name { source } => format!("name among the {source}"),
        Kind::Index { source, .. } => format!("name or index among the {source}"),
        Kind::Table { columns, .. } => format!(
            "table `{}`",
            columns.iter().map(|c| c.en).collect::<Vec<_>>().join(" | ")
        ),
        other => other.type_name().into(),
    }
}

fn op(out: &mut String, d: &OpDef) {
    let _ = writeln!(out, "### `{}` — `{}`\n", d.name, d.command);
    let _ = writeln!(
        out,
        "{}{}\n",
        if d.kind == OpKind::Query { "*Read-only.* " } else { "" },
        d.summary
    );
    if !d.aliases.is_empty() {
        let _ = writeln!(out, "Old names: `{}`.\n", d.aliases.join("`, `"));
    }
    if d.params.is_empty() {
        let _ = writeln!(out, "No arguments.\n");
    } else {
        let _ = writeln!(out, "| Argument | Type | Setting | Description |\n|---|---|---|---|");
        for p in d.params {
            let set = if p.setting.is_some() {
                "yes"
            } else if p.required {
                "**required**"
            } else {
                ""
            };
            let _ = writeln!(
                out,
                "| `{}` | {} | {} | {} |",
                p.key,
                kind_detail(&p.kind),
                set,
                p.help.replace('|', "\\|")
            );
        }
        out.push('\n');
    }
    if !d.positional.is_empty() {
        let _ = writeln!(
            out,
            "Positional form: `{} {}` (`-` skips one).\n",
            d.command,
            d.positional.join(" ")
        );
    }
    let _ = writeln!(out, "Result `data`: {}\n", d.result);
}

pub fn markdown() -> String {
    let mut out = String::new();
    out.push_str("<!-- Generated from crates/openmining_ops (cargo run -p openmining_cli -- mcp-doc). Do not edit by hand. -->\n");
    out.push_str("# Open Mining — operations for AI clients (MCP)\n\n");
    out.push_str(
        "Open CAD Studio exposes the drawing to AI clients with `OpenCADStudio --mcp`. Every Open Mining \
action below is a command run with the `run` tool (`ocs_run`), with arguments as `key=value` \
(quote values with spaces: `path=\"C:\\Mis datos\\log.csv\"`). Arguments marked *setting* are project \
settings: omitted, the current value is used (the one in the panel); given, it is stored for later \
runs. Entity inputs that a user would click (axes, solids, points) are passed as handles or coordinates.\n\n",
    );
    let _ = writeln!(
        out,
        "Each command writes its usual messages and **one result line** to the command history (read it with \
the history tool):\n\n```\n{PREFIX}{{\"op\":\"gt_load\",\"ok\":true,\"status\":\"done\",\"data\":{{…}},\"created\":[\"1A3F\"],\"warnings\":[]}}\n{PREFIX}{{\"op\":\"gt_hb\",\"ok\":false,\"error\":{{\"code\":\"out_of_range\",\"message\":\"…\",\"param\":\"gsi\"}}}}\n```\n"
    );
    out.push_str("`status` is `waiting_input` when a pick was started because an input was missing; pass it as an argument instead.\n\n");
    out.push_str("## Error codes\n\n| Code | Meaning |\n|---|---|\n");
    for c in ErrorCode::ALL {
        let _ = writeln!(out, "| `{}` | {} |", c.as_str(), c.describe());
    }
    out.push('\n');
    let groups = [
        Group::Project,
        Group::Drillholes,
        Group::Workings,
        Group::Axes,
        Group::Sections,
        Group::Geology,
        Group::Resources,
        Group::Geotechnics,
    ];
    for g in groups {
        let ops: Vec<&OpDef> = crate::all().into_iter().filter(|d| d.group == g).collect();
        if ops.is_empty() {
            continue;
        }
        let _ = writeln!(out, "## {}\n", g.title());
        for d in ops {
            op(&mut out, d);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn guide_is_up_to_date() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/MCP.md");
        let now = super::markdown();
        match std::fs::read_to_string(path) {
            // Windows checkouts may turn LF into CRLF.
            Ok(saved) if saved.replace("\r\n", "\n") == now => {}
            _ if std::env::var_os("OPENMINING_BLESS").is_some() => std::fs::write(path, now).unwrap(),
            _ => panic!("docs/MCP.md is stale: run `OPENMINING_BLESS=1 cargo test -p openmining_ops`"),
        }
    }
}
