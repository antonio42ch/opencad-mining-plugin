use std::collections::BTreeMap;

use mining_core::Vec3;

#[derive(Debug, Clone, PartialEq)]
pub struct Collar {
    pub hole_id: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub max_depth: f64,
    pub azimuth: Option<f64>,
    pub dip: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Survey {
    pub hole_id: String,
    pub depth: f64,
    pub azimuth: f64,
    pub dip: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Assay {
    pub hole_id: String,
    pub from: f64,
    pub to: f64,
    pub attributes: BTreeMap<String, Option<f64>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GeoLog {
    pub hole_id: String,
    pub from: f64,
    pub to: f64,
    pub attributes: BTreeMap<String, String>,
}

impl GeoLog {
    /// Non-empty attribute values joined with `" | "`.
    pub fn lithology(&self) -> String {
        self.attributes
            .values()
            .filter(|v| !v.is_empty())
            .cloned()
            .collect::<Vec<_>>()
            .join(" | ")
    }
}

/// A desurveyed station.
#[derive(Debug, Clone, PartialEq)]
pub struct SurveyNode {
    pub md: f64,
    pub xyz: Vec3,
    pub azimuth: f64,
    pub dip: f64,
    pub is_synthetic: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceType {
    Assay,
    Geo,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MappedInterval {
    pub hole_id: String,
    pub from: f64,
    pub to: f64,
    pub from_xyz: Vec3,
    pub to_xyz: Vec3,
    /// Point on the trace at `(from + to) / 2` (not the chord midpoint).
    pub centroid_xyz: Vec3,
    pub attributes: BTreeMap<String, Option<f64>>,
    pub geo_attributes: BTreeMap<String, String>,
    pub lithology: String,
    pub is_synthetic: bool,
    pub source: SourceType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Info,
    Warning,
    Error,
    Fatal,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidationMessage {
    pub severity: Severity,
    pub hole_id: String,
    pub rule: &'static str,
    pub message: String,
    pub row: Option<usize>,
}

/// Everything imported from the CSV files, before/after validation.
#[derive(Debug, Clone, Default)]
pub struct DataSet {
    pub collars: Vec<Collar>,
    pub surveys: Vec<Survey>,
    pub assays: Vec<Assay>,
    pub geo: Vec<GeoLog>,
}

/// Case-insensitive hole key used for every join.
pub fn hole_key(id: &str) -> String {
    id.trim().to_lowercase()
}
