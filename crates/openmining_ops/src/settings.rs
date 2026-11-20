//! Binding of setting parameters to the project settings (a JSON value,
//! addressed by JSON pointers).

use serde_json::{json, Value};

use crate::parse::Args;
use crate::{Kind, OpDef, Param};

/// Writes `v` at `ptr` (the path must exist, as every settings field is
/// serialized). Returns false when it does not.
pub fn set(root: &mut Value, ptr: &str, v: Value) -> bool {
    match root.pointer_mut(ptr) {
        Some(slot) => {
            *slot = v;
            true
        }
        None => false,
    }
}

/// Settings form of a normalized argument (a choice is stored by index).
pub fn to_setting(p: &Param, v: &Value) -> Value {
    match (&p.kind, v) {
        (Kind::Choice { options }, Value::String(k)) => json!(options.iter().position(|o| o.key == k).unwrap_or(0)),
        _ => v.clone(),
    }
}

/// Argument form of a stored setting (a choice index becomes its key).
pub fn from_setting(p: &Param, v: &Value) -> Value {
    match (&p.kind, v) {
        (Kind::Choice { options }, Value::Number(n)) => {
            let i = n.as_u64().unwrap_or(0) as usize;
            json!(options.get(i).or(options.first()).map_or("", |o| o.key))
        }
        _ => v.clone(),
    }
}

/// Stores every given setting argument into `root`.
pub fn apply(def: &OpDef, args: &Args, root: &mut Value) {
    for p in def.params {
        if let (Some(ptr), Some(v)) = (p.setting, args.get(p.key)) {
            set(root, ptr, to_setting(p, v));
        }
    }
}

/// Current values of the operation's settings (argument form).
pub fn current(def: &OpDef, root: &Value) -> Args {
    def.params
        .iter()
        .filter_map(|p| Some((p.key.to_string(), from_setting(p, root.pointer(p.setting?)?))))
        .collect()
}

/// Asserts that every setting of every operation points at a field of
/// `root` (the plugin's default settings) with a compatible JSON type.
pub fn check_against(root: &Value) {
    for d in crate::all() {
        for p in d.params {
            let Some(ptr) = p.setting else { continue };
            let v = root
                .pointer(ptr)
                .unwrap_or_else(|| panic!("{}.{}: {ptr} not in the settings", d.name, p.key));
            let ok = match p.kind {
                Kind::Number { .. } => v.is_number(),
                Kind::Integer { .. } | Kind::Choice { .. } => v.is_u64() || v.is_i64(),
                Kind::Bool => v.is_boolean(),
                Kind::Text | Kind::Path { .. } => v.is_string(),
                Kind::Table { .. } => v.is_array(),
                Kind::Point | Kind::Handle | Kind::Handles => false,
            };
            assert!(
                ok,
                "{}.{}: {ptr} holds {v}, not a {}",
                d.name,
                p.key,
                p.kind.type_name()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_choice() {
        let d = crate::find("gt_estimate").unwrap();
        let p = d.param("method").unwrap();
        assert_eq!(to_setting(p, &json!("nn")), json!(1));
        assert_eq!(from_setting(p, &json!(1)), json!("nn"));
        let mut root = json!({"gt": {"md_method": 0, "md_power": 2.0}});
        let mut args = Args::new();
        args.insert("method".into(), json!("nn"));
        apply(d, &args, &mut root);
        assert_eq!(root["gt"]["md_method"], json!(1));
    }
}
