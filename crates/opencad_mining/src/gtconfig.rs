//! Settings of the geotechnical module (part of the project settings saved
//! in the drawing).

use std::collections::BTreeMap;

use geotech_engine::classify::ClassifyOptions;
use geotech_engine::gsi::Correlation;
use geotech_engine::hoek_brown::Application;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GeotechConfig {
    pub logging: String,
    pub mapping: String,
    pub lab: String,
    pub joints: String,
    /// Column choices remembered per table kind: kind -> field -> header.
    pub column_maps: BTreeMap<String, BTreeMap<String, String>>,
    /// 0 = RMR89 (RMR14 pending verified tables).
    pub rmr_version: usize,
    pub correlation: usize,
    pub cross_fill: bool,
    pub rqd_from_ff: bool,
    /// 0 RMR class, 1 Q class, 2 GSI.
    pub color_by: usize,
    pub labels: bool,
    pub label_height: f64,
    pub hb_sigci: f64,
    pub hb_gsi: f64,
    pub hb_mi: f64,
    pub hb_d: f64,
    pub hb_ei: f64,
    pub hb_app: usize,
    /// kN/m3.
    pub hb_gamma: f64,
    pub hb_depth: f64,
    pub hb_lith: String,
    /// 0 IDW, 1 nearest.
    pub md_method: usize,
    pub md_power: f64,
    pub md_radii: [f64; 3],
    pub md_az: f64,
    pub md_min: f64,
    pub md_max: f64,
    pub md_hole: f64,
    pub md_domains: bool,
    /// 0 RMR, 1 GSI, 2 log Q, 3 class.
    pub md_attr: usize,
}

impl Default for GeotechConfig {
    fn default() -> Self {
        GeotechConfig {
            logging: String::new(),
            mapping: String::new(),
            lab: String::new(),
            joints: String::new(),
            column_maps: BTreeMap::new(),
            rmr_version: 0,
            correlation: 0,
            cross_fill: false,
            rqd_from_ff: true,
            color_by: 0,
            labels: true,
            label_height: 1.0,
            hb_sigci: 100.0,
            hb_gsi: 50.0,
            hb_mi: 10.0,
            hb_d: 0.0,
            hb_ei: 0.0,
            hb_app: 0,
            hb_gamma: 27.0,
            hb_depth: 300.0,
            hb_lith: String::new(),
            md_method: 0,
            md_power: 2.0,
            md_radii: [50.0, 50.0, 20.0],
            md_az: 0.0,
            md_min: 1.0,
            md_max: 12.0,
            md_hole: 0.0,
            md_domains: false,
            md_attr: 0,
        }
    }
}

impl GeotechConfig {
    pub fn options(&self) -> ClassifyOptions {
        ClassifyOptions {
            correlation: Correlation::ALL[self.correlation.min(2)],
            cross_fill: self.cross_fill,
            rqd_from_ff: self.rqd_from_ff,
        }
    }

    pub fn application(&self) -> Application {
        if self.hb_app == 1 {
            Application::Slope
        } else {
            Application::Tunnel
        }
    }

    pub fn column_map(&self, kind: &str) -> BTreeMap<String, String> {
        self.column_maps.get(kind).cloned().unwrap_or_default()
    }
}
