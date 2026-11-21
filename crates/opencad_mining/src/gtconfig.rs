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
    /// View: 0 levels, 1 sections by 2 points, 2 sections along an axis, 3 cubes.
    pub view_mode: usize,
    pub view_z_from: f64,
    pub view_z_to: f64,
    pub view_z_step: f64,
    pub view_spacing: f64,
    pub view_count: f64,
    pub view_interval: f64,
    pub view_limit: f64,
    /// 0 show all, 1 only values below the threshold, 2 only above.
    pub view_filter: usize,
    pub view_threshold: f64,
    pub view_cut: bool,
    pub view_cut_e: f64,
    pub view_cut_n: f64,
    pub view_cut_z: f64,
    pub view_cut_dip: f64,
    pub view_cut_dipdir: f64,
    /// 0 keep the side the normal points to, 1 the other.
    pub view_cut_side: usize,
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
            view_mode: 0,
            view_z_from: 0.0,
            view_z_to: 0.0,
            view_z_step: 12.0,
            view_spacing: 20.0,
            view_count: 1.0,
            view_interval: 20.0,
            view_limit: 20_000.0,
            view_filter: 0,
            view_threshold: 3.0,
            view_cut: false,
            view_cut_e: 0.0,
            view_cut_n: 0.0,
            view_cut_z: 0.0,
            view_cut_dip: 90.0,
            view_cut_dipdir: 0.0,
            view_cut_side: 0,
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
