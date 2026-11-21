//! `help`: the operations described for an AI client that cannot see the
//! guide (until the host offers native tools): list, or the full detail and
//! JSON Schema of one.

use serde_json::{json, Value};

use crate::{schema, Group, Kind, OpDef, OpKind};

fn group_key(g: Group) -> &'static str {
    match g {
        Group::Project => "project",
        Group::Drillholes => "drillholes",
        Group::Workings => "workings",
        Group::Axes => "axes",
        Group::Sections => "sections",
        Group::Geology => "geology",
        Group::Resources => "resources",
        Group::Geotechnics => "geotechnics",
    }
}

fn param(p: &crate::Param) -> Value {
    json!({
        "key": p.key,
        "type": p.kind.type_name(),
        "setting": p.setting.is_some(),
        "required": p.required,
        "help": p.help,
        "options": match p.kind {
            Kind::Choice { options } => json!(options.iter().map(|o| o.key).collect::<Vec<_>>()),
            _ => Value::Null,
        },
    })
}

fn brief(d: &OpDef) -> Value {
    json!({
        "name": d.name,
        "command": d.command,
        "group": group_key(d.group),
        "kind": if d.kind == OpKind::Query { "query" } else { "action" },
        "summary": d.summary,
    })
}

fn detail(d: &OpDef) -> Value {
    let mut v = brief(d);
    v["aliases"] = json!(d.aliases);
    v["positional"] = json!(d.positional);
    v["parameters"] = json!(d.params.iter().map(param).collect::<Vec<_>>());
    v["input_schema"] = schema::input_schema(d);
    v["result"] = json!(d.result);
    v
}

/// Groups with at least one operation, for the listing.
pub fn groups() -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for d in crate::all() {
        let g = group_key(d.group);
        if !out.contains(&g) {
            out.push(g);
        }
    }
    out
}

/// One operation in full (by name or command), or the list (optionally of a
/// group). `None` when the name or the group does not exist.
pub fn help(op: Option<&str>, group: Option<&str>) -> Option<Value> {
    if let Some(name) = op {
        return crate::find(name).map(detail);
    }
    let list: Vec<Value> = crate::all()
        .into_iter()
        .filter(|d| group.is_none_or(|g| group_key(d.group).eq_ignore_ascii_case(g)))
        .map(brief)
        .collect();
    if list.is_empty() {
        return None;
    }
    Some(json!({"groups": groups(), "operations": list}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_detail_and_unknowns() {
        let all = help(None, None).unwrap();
        assert_eq!(all["operations"].as_array().unwrap().len(), crate::all().len());
        assert!(all["groups"].as_array().unwrap().contains(&json!("geotechnics")));
        let g = help(None, Some("Geotechnics")).unwrap();
        assert!(g["operations"]
            .as_array()
            .unwrap()
            .iter()
            .all(|o| o["group"] == "geotechnics"));
        let d = help(Some("min_gt_view"), None).unwrap();
        assert_eq!(d["name"], "gt_view");
        let params = d["parameters"].as_array().unwrap();
        let mode = params.iter().find(|p| p["key"] == "mode").unwrap();
        assert_eq!(mode["options"], json!(["levels", "two_points", "axis", "cubes"]));
        assert!(d["input_schema"]["properties"]["limit"].is_object());
        assert!(help(Some("nope"), None).is_none() && help(None, Some("nope")).is_none());
    }
}
