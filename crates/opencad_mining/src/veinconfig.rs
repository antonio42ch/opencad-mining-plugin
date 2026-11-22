//! Settings of the narrow-vein model (active vein).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct VeinConfig {
    pub name: String,
    pub hw_layer: String,
    pub fw_layer: String,
    pub use_intercepts: bool,
    pub vein_attr: String,
    pub vein_values: String,
    /// 0 fitted to the contacts, 1 typed dip / dip direction.
    pub orient_mode: usize,
    pub dip: f64,
    pub dipdir: f64,
    pub step: f64,
    pub margin: f64,
    pub radius: f64,
    pub min_samples: usize,
    pub max_samples: usize,
    /// 0 kriging, 1 IDW.
    pub method: usize,
    pub power: f64,
    pub range: f64,
    pub densify: f64,
    pub min_thickness: f64,
    // Stage B2: prism model.
    pub grade: String,
    pub thick_col: String,
    pub lag: f64,
    pub nlags: usize,
    /// 0 spherical, 1 exponential, 2 gaussian.
    pub model_kind: usize,
    pub nugget: f64,
    pub sill: f64,
    pub range_strike: f64,
    pub range_dip: f64,
    pub block_u: f64,
    pub block_v: f64,
    pub est_radius: f64,
    pub est_min: usize,
    pub est_max: usize,
    pub max_dist: f64,
    pub disc: usize,
    pub density: f64,
    /// 0 grade, 1 thickness, 2 accumulation, 3 distance, 4 kriging variance, 5 category.
    pub color_by: usize,
    pub draw_prisms: bool,
    pub long_labels: bool,
    /// 0 exclude, 1 grade, 2 thickness, 3 boundary, 4 undo, 5 clear, 6 list.
    pub edit_action: usize,
    pub edit_value: f64,
    /// Placement of the last long section, per vein.
    pub long: BTreeMap<String, LongPlacement>,
    /// Boundary and edits on the long section, per vein.
    pub edits: BTreeMap<String, VeinEdits>,
    // Stage B3: tonnage.
    pub mmw: f64,
    pub ob_hw: f64,
    pub ob_fw: f64,
    pub grade_hw: f64,
    pub grade_fw: f64,
    pub density_waste: f64,
    /// Percent.
    pub recovery: f64,
    pub level_layer: String,
    pub raise_layer: String,
    pub level_pillar: f64,
    pub raise_pillar: f64,
    pub side_tol: f64,
    pub side_min: usize,
    pub rules: Vec<RuleCfg>,
    pub build_fit: bool,
    pub build_cubicate: bool,
}

/// One category rule; `None` = not checked.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RuleCfg {
    pub category: String,
    pub min_sides: Option<usize>,
    pub max_dist: Option<f64>,
    pub min_samples: Option<usize>,
    pub min_grade: Option<f64>,
}

impl RuleCfg {
    pub fn rule(&self) -> vein_engine::reserves::Rule {
        vein_engine::reserves::Rule {
            category: self.category.clone(),
            min_sides: self.min_sides,
            max_dist: self.max_dist,
            min_samples: self.min_samples,
            min_grade: self.min_grade,
        }
    }
}

/// Example rules (replace them with the company's norms).
pub fn example_rules() -> Vec<RuleCfg> {
    vec![
        RuleCfg {
            category: "Medido".into(),
            min_sides: Some(3),
            max_dist: Some(15.0),
            ..Default::default()
        },
        RuleCfg {
            category: "Indicado".into(),
            min_sides: Some(2),
            max_dist: Some(30.0),
            ..Default::default()
        },
        RuleCfg {
            category: "Inferido".into(),
            min_sides: Some(1),
            ..Default::default()
        },
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LongPlacement {
    /// Drawing point of `(u0, v0)`.
    pub at: [f64; 3],
    pub u0: f64,
    pub v0: f64,
}

impl LongPlacement {
    pub fn to_plane(&self, p: [f64; 3]) -> [f64; 2] {
        [p[0] - self.at[0] + self.u0, p[1] - self.at[1] + self.v0]
    }
    pub fn to_drawing(&self, u: f64, v: f64) -> [f64; 3] {
        [self.at[0] + u - self.u0, self.at[1] + v - self.v0, self.at[2]]
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct EditCfg {
    /// exclude, grade, thickness
    pub action: String,
    pub value: f64,
    pub polygon: Vec<[f64; 2]>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct VeinEdits {
    pub boundary: Vec<[f64; 2]>,
    pub list: Vec<EditCfg>,
}

impl Default for VeinConfig {
    fn default() -> Self {
        VeinConfig {
            name: "V1".into(),
            hw_layer: "VETA_TECHO".into(),
            fw_layer: "VETA_PISO".into(),
            use_intercepts: true,
            vein_attr: "tipo".into(),
            vein_values: "VETA".into(),
            orient_mode: 0,
            dip: 70.0,
            dipdir: 90.0,
            step: 2.0,
            margin: 5.0,
            radius: 50.0,
            min_samples: 2,
            max_samples: 12,
            method: 0,
            power: 2.0,
            range: 0.0,
            densify: 1.0,
            min_thickness: 0.1,
            grade: "au".into(),
            thick_col: "potencia".into(),
            lag: 10.0,
            nlags: 10,
            model_kind: 0,
            nugget: 0.0,
            sill: 1.0,
            range_strike: 40.0,
            range_dip: 25.0,
            block_u: 5.0,
            block_v: 5.0,
            est_radius: 60.0,
            est_min: 2,
            est_max: 16,
            max_dist: 40.0,
            disc: 3,
            density: 2.7,
            color_by: 0,
            draw_prisms: true,
            long_labels: true,
            edit_action: 0,
            edit_value: 0.0,
            long: BTreeMap::new(),
            edits: BTreeMap::new(),
            mmw: 0.8,
            ob_hw: 0.1,
            ob_fw: 0.05,
            grade_hw: 0.0,
            grade_fw: 0.0,
            density_waste: 2.6,
            recovery: 90.0,
            level_layer: "NIVELES".into(),
            raise_layer: "CHIMENEAS".into(),
            level_pillar: 0.0,
            raise_pillar: 0.0,
            side_tol: 3.0,
            side_min: 1,
            rules: example_rules(),
            build_fit: true,
            build_cubicate: true,
        }
    }
}

impl VeinConfig {
    pub fn values(&self) -> Vec<String> {
        self.vein_values
            .split([';', ','])
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }
}
