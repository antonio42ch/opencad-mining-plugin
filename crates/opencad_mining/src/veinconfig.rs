//! Settings of the narrow-vein model (active vein).

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
