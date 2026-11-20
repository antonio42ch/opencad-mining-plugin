//! JSON Schema of an operation's arguments (native AI tools).

use serde_json::{json, Map, Value};

use crate::{Kind, OpDef, Param};

fn property(p: &Param) -> Value {
    let mut desc = p.help.to_string();
    if p.setting.is_some() {
        desc.push_str(" Project setting: omitted = current value; a given value is kept for later runs.");
    }
    let mut v = match p.kind {
        Kind::Number { min, max, .. } => {
            let mut m = json!({"type": "number"});
            if let Some(a) = min {
                m["minimum"] = json!(a);
            }
            if let Some(b) = max {
                m["maximum"] = json!(b);
            }
            m
        }
        Kind::Integer { min, max } => {
            let mut m = json!({"type": "integer"});
            if let Some(a) = min {
                m["minimum"] = json!(a);
            }
            if let Some(b) = max {
                m["maximum"] = json!(b);
            }
            m
        }
        Kind::Bool => json!({"type": "boolean"}),
        Kind::Text => json!({"type": "string"}),
        Kind::Path { .. } => json!({"type": "string", "description": "file path"}),
        Kind::Choice { options } => {
            json!({"type": "string", "enum": options.iter().map(|o| o.key).collect::<Vec<_>>()})
        }
        Kind::Point => json!({"type": "array", "items": {"type": "number"}, "minItems": 2, "maxItems": 3}),
        Kind::Handle => json!({"type": "string", "pattern": "^[0-9A-Fa-f]+$"}),
        Kind::Handles => {
            json!({"type": "array", "items": {"type": "string", "pattern": "^[0-9A-Fa-f]+$"}, "minItems": 1})
        }
        Kind::Table { columns } => json!({
            "type": "array",
            "items": {"type": "array", "items": {"type": "string"}},
            "description": format!("rows of [{}]", columns.iter().map(|c| c.en).collect::<Vec<_>>().join(", ")),
        }),
    };
    let extra = v
        .get("description")
        .and_then(Value::as_str)
        .map(|d| format!(" ({d})"))
        .unwrap_or_default();
    v["description"] = json!(format!("{desc}{extra}"));
    v
}

pub fn input_schema(def: &OpDef) -> Value {
    let props: Map<String, Value> = def.params.iter().map(|p| (p.key.to_string(), property(p))).collect();
    let required: Vec<&str> = def.params.iter().filter(|p| p.required).map(|p| p.key).collect();
    json!({
        "type": "object",
        "properties": props,
        "required": required,
        "additionalProperties": false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_schema_is_an_object_with_its_params() {
        for d in crate::all() {
            let s = input_schema(d);
            assert_eq!(s["type"], "object");
            assert_eq!(s["properties"].as_object().unwrap().len(), d.params.len(), "{}", d.name);
            for p in d.params {
                assert!(s["properties"][p.key]["description"]
                    .as_str()
                    .is_some_and(|t| !t.is_empty()));
            }
        }
    }
}
