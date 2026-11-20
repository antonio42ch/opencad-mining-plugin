//! Machine-readable outcome of an operation.
//!
//! Besides its human messages, every command writes one line to the command
//! history: `OPENMINING_RESULT {json}`. Native tools return the same JSON.

use serde_json::{json, Value};

pub const PREFIX: &str = "OPENMINING_RESULT ";

/// Stable error codes (AI clients branch on them).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    UnknownCommand,
    UnknownArgument,
    InvalidArgument,
    OutOfRange,
    MissingArgument,
    /// Something the operation needs is not loaded or computed yet.
    NoData,
    /// A table lacks a required column.
    MissingColumn,
    /// A referenced entity, file, hole, domain... does not exist.
    NotFound,
    /// Waiting for a pick: give the input as an argument instead.
    NeedsInput,
    Cancelled,
    Failed,
}

impl ErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            ErrorCode::UnknownCommand => "unknown_command",
            ErrorCode::UnknownArgument => "unknown_argument",
            ErrorCode::InvalidArgument => "invalid_argument",
            ErrorCode::OutOfRange => "out_of_range",
            ErrorCode::MissingArgument => "missing_argument",
            ErrorCode::NoData => "no_data",
            ErrorCode::MissingColumn => "missing_column",
            ErrorCode::NotFound => "not_found",
            ErrorCode::NeedsInput => "needs_input",
            ErrorCode::Cancelled => "cancelled",
            ErrorCode::Failed => "failed",
        }
    }
    pub const ALL: [ErrorCode; 11] = [
        ErrorCode::UnknownCommand,
        ErrorCode::UnknownArgument,
        ErrorCode::InvalidArgument,
        ErrorCode::OutOfRange,
        ErrorCode::MissingArgument,
        ErrorCode::NoData,
        ErrorCode::MissingColumn,
        ErrorCode::NotFound,
        ErrorCode::NeedsInput,
        ErrorCode::Cancelled,
        ErrorCode::Failed,
    ];
    pub fn describe(self) -> &'static str {
        match self {
            ErrorCode::UnknownCommand => "the command is not an Open Mining operation",
            ErrorCode::UnknownArgument => "an argument name is not a parameter of the operation",
            ErrorCode::InvalidArgument => "a value has the wrong type or format",
            ErrorCode::OutOfRange => "a number is outside its limits",
            ErrorCode::MissingArgument => "a required argument was not given",
            ErrorCode::NoData => {
                "something the operation needs is not loaded or computed yet (run the step named in the message)"
            }
            ErrorCode::MissingColumn => "an input table lacks a required column",
            ErrorCode::NotFound => "a referenced entity, file, hole or domain does not exist",
            ErrorCode::NeedsInput => "the operation waits for a pick; pass the input as an argument instead",
            ErrorCode::Cancelled => "the user cancelled a dialog or pick",
            ErrorCode::Failed => "the operation could not complete (see the message)",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OpError {
    pub code: ErrorCode,
    /// Localized message (the same the user sees).
    pub message: String,
    /// Argument the error refers to.
    pub param: Option<String>,
}

impl OpError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        OpError {
            code,
            message: message.into(),
            param: None,
        }
    }
    pub fn at(mut self, param: &str) -> Self {
        self.param = Some(param.to_string());
        self
    }
}

impl std::fmt::Display for OpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Status {
    #[default]
    Done,
    /// An interactive pick was started; its follow-up writes the result.
    Waiting,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct OpResult {
    pub status: Status,
    pub data: Value,
    /// Handles (hex) of the entities created.
    pub created: Vec<String>,
    /// Non-fatal problems (localized).
    pub warnings: Vec<String>,
}

impl OpResult {
    pub fn data(data: Value) -> Self {
        OpResult {
            data,
            ..Default::default()
        }
    }
    pub fn waiting(prompt: &str) -> Self {
        OpResult {
            status: Status::Waiting,
            data: json!({ "prompt": prompt }),
            ..Default::default()
        }
    }
}

pub fn to_json(op: &str, r: &Result<OpResult, OpError>) -> Value {
    match r {
        Ok(o) => json!({
            "op": op,
            "ok": true,
            "status": if o.status == Status::Waiting { "waiting_input" } else { "done" },
            "data": o.data,
            "created": o.created,
            "warnings": o.warnings,
        }),
        Err(e) => json!({
            "op": op,
            "ok": false,
            "error": { "code": e.code.as_str(), "message": e.message, "param": e.param },
        }),
    }
}

/// The history line of an outcome.
pub fn line(op: &str, r: &Result<OpResult, OpError>) -> String {
    format!("{PREFIX}{}", to_json(op, r))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_parse_back() {
        let ok = Ok(OpResult {
            data: json!({"n": 3}),
            created: vec!["1A".into()],
            ..Default::default()
        });
        let l = line("gt_load", &ok);
        let v: Value = serde_json::from_str(l.strip_prefix(PREFIX).unwrap()).unwrap();
        assert_eq!(
            (v["ok"].as_bool(), v["data"]["n"].as_i64(), v["status"].as_str()),
            (Some(true), Some(3), Some("done"))
        );
        let err: Result<OpResult, OpError> = Err(OpError::new(ErrorCode::OutOfRange, "too big").at("gsi"));
        let v = to_json("gt_hb", &err);
        assert_eq!(
            (v["error"]["code"].as_str(), v["error"]["param"].as_str()),
            (Some("out_of_range"), Some("gsi"))
        );
        assert!(!line("x", &err).contains('\n'));
    }
}
