//! Arguments: `key=value` command lines and JSON objects, both validated
//! against the operation's parameters into the same normalized form.
//!
//! Normalized values: number -> JSON number, integer -> JSON integer,
//! boolean, text/path -> string, choice -> option key, point -> `[x, y, z]`,
//! handle -> upper-case hex string, handles -> array of them, table ->
//! array of arrays of strings.

use std::collections::BTreeMap;

use serde_json::{json, Value};

use crate::result::{ErrorCode, OpError};
use crate::{Kind, OpDef, Param};

pub type Args = BTreeMap<String, Value>;

/// Words of a command line; `"..."` keeps spaces (Windows paths).
pub fn tokenize(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let (mut quoted, mut any) = (false, false);
    for ch in line.chars() {
        match ch {
            // A quote opens a value only at the start of a word or right after
            // `key=`; elsewhere it is literal (JSON tables: rows=[["a",1]]).
            '"' if quoted || cur.is_empty() || cur.ends_with('=') => {
                quoted = !quoted;
                any = true;
            }
            c if c.is_whitespace() && !quoted => {
                if any {
                    out.push(std::mem::take(&mut cur));
                    any = false;
                }
            }
            c => {
                cur.push(c);
                any = true;
            }
        }
    }
    if any {
        out.push(cur);
    }
    out
}

fn bad(p: &Param, msg: String) -> OpError {
    OpError::new(ErrorCode::InvalidArgument, format!("{}: {msg}", p.key)).at(p.key)
}

fn number(p: &Param, s: &str) -> Result<f64, OpError> {
    s.trim()
        .replace(',', ".")
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .ok_or_else(|| bad(p, format!("'{s}' is not a number")))
}

fn check_range(p: &Param, v: f64, min: Option<f64>, max: Option<f64>) -> Result<(), OpError> {
    let below = min.is_some_and(|m| v < m);
    let above = max.is_some_and(|m| v > m);
    if below || above {
        let lim = match (min, max) {
            (Some(a), Some(b)) => format!("{a}..{b}"),
            (Some(a), None) => format!(">= {a}"),
            (None, Some(b)) => format!("<= {b}"),
            _ => String::new(),
        };
        return Err(OpError::new(ErrorCode::OutOfRange, format!("{} = {v} is outside {lim}", p.key)).at(p.key));
    }
    Ok(())
}

fn hex(p: &Param, s: &str) -> Result<String, OpError> {
    let t = s.trim().trim_start_matches("0x").trim_start_matches("0X");
    if t.is_empty() || !t.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(bad(p, format!("'{s}' is not an entity handle (hex)")));
    }
    Ok(t.to_ascii_uppercase())
}

/// Converts one textual value.
pub fn convert(p: &Param, s: &str) -> Result<Value, OpError> {
    Ok(match p.kind {
        Kind::Number { min, max, .. } => {
            let v = number(p, s)?;
            check_range(p, v, min, max)?;
            json!(v)
        }
        Kind::Integer { min, max } => {
            let v = s
                .trim()
                .parse::<i64>()
                .map_err(|_| bad(p, format!("'{s}' is not an integer")))?;
            check_range(p, v as f64, min.map(|m| m as f64), max.map(|m| m as f64))?;
            json!(v)
        }
        Kind::Bool => match s.trim().to_lowercase().as_str() {
            "1" | "true" | "yes" | "si" | "sí" | "sim" | "on" => json!(true),
            "0" | "false" | "no" | "não" | "nao" | "off" => json!(false),
            _ => return Err(bad(p, format!("'{s}' is not true/false"))),
        },
        Kind::Text | Kind::Path { .. } | Kind::Name { .. } => json!(s),
        // An index or a name; the plugin resolves names against its list.
        Kind::Index { .. } => match s.trim().parse::<u64>() {
            Ok(i) => json!(i),
            Err(_) => json!(s.trim()),
        },
        Kind::Choice { options } => {
            let t = s.trim();
            let by_key = options.iter().find(|o| o.key.eq_ignore_ascii_case(t));
            let by_label = || {
                options.iter().find(|o| {
                    [o.label.es, o.label.en, o.label.pt]
                        .iter()
                        .any(|l| l.eq_ignore_ascii_case(t))
                })
            };
            let by_index = || t.parse::<usize>().ok().and_then(|i| options.get(i));
            match by_key.or_else(by_label).or_else(by_index) {
                Some(o) => json!(o.key),
                None => {
                    let keys: Vec<&str> = options.iter().map(|o| o.key).collect();
                    return Err(bad(p, format!("'{s}' is not one of {}", keys.join(", "))));
                }
            }
        }
        Kind::Point => {
            let parts: Vec<&str> = s.split(',').collect();
            if !(2..=3).contains(&parts.len()) {
                return Err(bad(p, format!("'{s}' is not a point x,y[,z]")));
            }
            let mut v = [0.0; 3];
            for (i, part) in parts.iter().enumerate() {
                v[i] = part
                    .trim()
                    .parse::<f64>()
                    .ok()
                    .filter(|x| x.is_finite())
                    .ok_or_else(|| bad(p, format!("'{s}' is not a point x,y[,z]")))?;
            }
            json!(v)
        }
        Kind::Handle => json!(hex(p, s)?),
        Kind::Handles => {
            let list: Result<Vec<String>, OpError> = s
                .split([',', ';'])
                .filter(|x| !x.trim().is_empty())
                .map(|x| hex(p, x))
                .collect();
            let list = list?;
            if list.is_empty() {
                return Err(bad(p, "no handles".into()));
            }
            json!(list)
        }
        Kind::Table { .. } => {
            let t = s.trim();
            if t.starts_with('[') {
                let v: Value = serde_json::from_str(t).map_err(|e| bad(p, e.to_string()))?;
                return from_json(p, &v);
            }
            let rows: Vec<Vec<String>> = t
                .split(';')
                .filter(|r| !r.trim().is_empty())
                .map(|r| r.split(',').map(|c| c.trim().to_string()).collect())
                .collect();
            json!(rows)
        }
    })
}

/// Converts one JSON value (native tool arguments).
pub fn from_json(p: &Param, v: &Value) -> Result<Value, OpError> {
    match (&p.kind, v) {
        (_, Value::String(s)) => convert(p, s),
        (Kind::Number { .. } | Kind::Integer { .. }, Value::Number(n)) => convert(p, &n.to_string()),
        (Kind::Bool, Value::Bool(b)) => Ok(json!(b)),
        (Kind::Index { .. }, Value::Number(n)) if n.is_u64() => Ok(v.clone()),
        (Kind::Choice { options }, Value::Number(n)) => n
            .as_u64()
            .and_then(|i| options.get(i as usize))
            .map(|o| json!(o.key))
            .ok_or_else(|| bad(p, format!("{n} is not an option index"))),
        (Kind::Point, Value::Array(a)) => {
            let nums: Option<Vec<f64>> = a.iter().map(Value::as_f64).collect();
            match nums {
                Some(n) if (2..=3).contains(&n.len()) => Ok(json!([n[0], n[1], n.get(2).copied().unwrap_or(0.0)])),
                _ => Err(bad(p, "a point is [x, y] or [x, y, z]".into())),
            }
        }
        (Kind::Handles, Value::Array(a)) => {
            let list: Result<Vec<String>, OpError> = a
                .iter()
                .map(|x| {
                    x.as_str()
                        .ok_or_else(|| bad(p, "handles are strings".into()))
                        .and_then(|s| hex(p, s))
                })
                .collect();
            Ok(json!(list?))
        }
        (Kind::Table { .. }, Value::Array(rows)) => {
            let mut out = Vec::new();
            for r in rows {
                let cells = r
                    .as_array()
                    .ok_or_else(|| bad(p, "a table is an array of rows".into()))?;
                out.push(
                    cells
                        .iter()
                        .map(|c| match c {
                            Value::String(s) => s.clone(),
                            other => other.to_string(),
                        })
                        .collect::<Vec<_>>(),
                );
            }
            Ok(json!(out))
        }
        _ => Err(bad(p, format!("expected a {}", p.kind.type_name()))),
    }
}

fn is_key(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn finish(def: &OpDef, args: Args) -> Result<Args, OpError> {
    for p in def.params.iter().filter(|p| p.required) {
        if !args.contains_key(p.key) {
            return Err(OpError::new(ErrorCode::MissingArgument, format!("{} needs {}", def.command, p.key)).at(p.key));
        }
    }
    Ok(args)
}

/// Parses a full command line (`MIN_X key=value ...` or positional values).
pub fn parse_command(def: &OpDef, line: &str) -> Result<Args, OpError> {
    let mut args = Args::new();
    let mut slots = def.positional.iter();
    for tok in tokenize(line).into_iter().skip(1) {
        if let Some((k, v)) = tok.split_once('=').filter(|(k, _)| is_key(k)) {
            let key = k.to_ascii_lowercase();
            let p = def.param(&key).ok_or_else(|| {
                let keys: Vec<&str> = def.params.iter().map(|p| p.key).collect();
                OpError::new(
                    ErrorCode::UnknownArgument,
                    format!(
                        "{} has no argument '{key}' (arguments: {})",
                        def.command,
                        keys.join(", ")
                    ),
                )
                .at(&key)
            })?;
            args.insert(p.key.to_string(), convert(p, v)?);
            continue;
        }
        let key = slots.next().ok_or_else(|| {
            OpError::new(
                ErrorCode::UnknownArgument,
                format!("{}: unexpected value '{tok}'", def.command),
            )
        })?;
        if tok == "-" {
            continue;
        }
        let p = def.param(key).expect("positional names a param (checked by tests)");
        args.insert(p.key.to_string(), convert(p, &tok)?);
    }
    finish(def, args)
}

/// Reads the JSON object of a native tool call.
pub fn args_from_json(def: &OpDef, obj: &Value) -> Result<Args, OpError> {
    let mut args = Args::new();
    let map = match obj {
        Value::Object(m) => m.clone(),
        Value::Null => Default::default(),
        _ => {
            return Err(OpError::new(
                ErrorCode::InvalidArgument,
                "arguments must be a JSON object",
            ))
        }
    };
    for (k, v) in &map {
        let p = def.param(k).ok_or_else(|| {
            OpError::new(
                ErrorCode::UnknownArgument,
                format!("{} has no argument '{k}'", def.name),
            )
            .at(k)
        })?;
        if !v.is_null() {
            args.insert(p.key.to_string(), from_json(p, v)?);
        }
    }
    finish(def, args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{input, opt, setting, Group, Kind, OpKind, L};

    const OPTS: &[crate::Opt] = &[
        opt("idw", L::new("Inverso", "Inverse distance", "Inverso")),
        opt("nn", L::same("Nearest")),
    ];
    const P: &[Param] = &[
        input(
            "dom",
            Kind::Index {
                source: "domains",
                all: None,
            },
            false,
            L::same("d"),
            "domain",
        ),
        setting(
            "path",
            Kind::Path { extensions: &["csv"] },
            "/x/path",
            L::same("p"),
            "a file",
        ),
        setting(
            "power",
            Kind::num(Some(0.1), Some(5.0), 0.5),
            "/x/power",
            L::same("p"),
            "power",
        ),
        setting(
            "method",
            Kind::Choice { options: OPTS },
            "/x/method",
            L::same("m"),
            "method",
        ),
        setting("labels", Kind::Bool, "/x/labels", L::same("l"), "labels"),
        input("at", Kind::Point, false, L::same("a"), "point"),
        input("axes", Kind::Handles, false, L::same("a"), "axes"),
        input(
            "count",
            Kind::Integer {
                min: Some(1),
                max: None,
            },
            false,
            L::same("c"),
            "count",
        ),
        input(
            "rows",
            Kind::Table {
                columns: &[],
                codec: "t",
            },
            false,
            L::same("r"),
            "rows",
        ),
        input("must", Kind::Text, true, L::same("m"), "required"),
    ];
    const D: OpDef = OpDef {
        name: "t",
        command: "MIN_T",
        aliases: &[],
        label: L::same("t"),
        group: Group::Project,
        kind: OpKind::Action,
        summary: "s",
        params: P,
        positional: &["path", "power"],
        result: "r",
    };

    #[test]
    fn key_value_and_positional() {
        let a = parse_command(
            &D,
            r#"MIN_T "C:\Mis datos\a.csv" - method=NN labels=sí at=1.5,2,3 axes=1a,2B must=x"#,
        )
        .unwrap();
        assert_eq!(a["path"], json!(r"C:\Mis datos\a.csv"));
        assert!(!a.contains_key("power"));
        assert_eq!(a["method"], json!("nn"));
        assert_eq!(a["labels"], json!(true));
        assert_eq!(a["at"], json!([1.5, 2.0, 3.0]));
        assert_eq!(a["axes"], json!(["1A", "2B"]));
        let a = parse_command(&D, "MIN_T dom=2 must=x").unwrap();
        assert_eq!(a["dom"], json!(2));
        assert_eq!(
            parse_command(&D, r#"MIN_T dom="Óxidos A" must=x"#).unwrap()["dom"],
            json!("Óxidos A")
        );
        let a = parse_command(&D, "MIN_T power=2,5 method=Inverso at=1,2 rows=a,b;c,d must=y count=3").unwrap();
        assert_eq!(
            (a["power"].clone(), a["method"].clone(), a["at"].clone()),
            (json!(2.5), json!("idw"), json!([1.0, 2.0, 0.0]))
        );
        assert_eq!(a["rows"], json!([["a", "b"], ["c", "d"]]));
        assert_eq!(
            parse_command(&D, r#"MIN_T rows=[["x",1]] must=z"#).unwrap()["rows"],
            json!([["x", "1"]])
        );
    }

    #[test]
    fn errors_have_codes() {
        let code = |l: &str| parse_command(&D, l).unwrap_err().code;
        assert_eq!(code("MIN_T must=x power=9"), ErrorCode::OutOfRange);
        assert_eq!(code("MIN_T must=x power=abc"), ErrorCode::InvalidArgument);
        assert_eq!(code("MIN_T must=x nope=1"), ErrorCode::UnknownArgument);
        assert_eq!(code("MIN_T must=x method=kriging"), ErrorCode::InvalidArgument);
        assert_eq!(code("MIN_T must=x axes=zz"), ErrorCode::InvalidArgument);
        assert_eq!(code("MIN_T must=x count=0"), ErrorCode::OutOfRange);
        assert_eq!(code("MIN_T"), ErrorCode::MissingArgument);
        assert_eq!(code("MIN_T a - c must=x"), ErrorCode::UnknownArgument);
        assert_eq!(code("MIN_T a b must=x"), ErrorCode::InvalidArgument);
        let e = parse_command(&D, "MIN_T must=x nope=1").unwrap_err();
        assert!(
            e.message.contains("power") && e.param.as_deref() == Some("nope"),
            "{}",
            e.message
        );
    }

    #[test]
    fn json_arguments_match_the_command_line() {
        let a = args_from_json(
            &D,
            &json!({"power": 2, "method": 1, "at": [1, 2], "axes": ["1a"], "must": "x", "labels": true}),
        )
        .unwrap();
        let b = parse_command(&D, "MIN_T power=2 method=nn at=1,2 axes=1A must=x labels=1").unwrap();
        assert_eq!(a, b);
        assert_eq!(
            args_from_json(&D, &json!({"must": "x", "at": [1]})).unwrap_err().code,
            ErrorCode::InvalidArgument
        );
        assert_eq!(
            args_from_json(&D, &json!([1])).unwrap_err().code,
            ErrorCode::InvalidArgument
        );
    }

    #[test]
    fn tokenizer_keeps_quoted_spaces() {
        assert_eq!(tokenize(r#"A "x y" b="c d" "" e"#), vec!["A", "x y", "b=c d", "", "e"]);
    }
}
