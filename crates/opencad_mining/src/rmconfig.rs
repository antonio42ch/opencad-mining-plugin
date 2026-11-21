//! Settings of the geotechnical phases 3 and 4 (structures, stresses and
//! design); part of the project settings saved in the drawing.

use serde::{Deserialize, Serialize};

/// One discontinuity set of the sets table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct JointSetCfg {
    pub name: String,
    pub dip: f64,
    pub dipdir: f64,
    /// Friction angle (degrees) and cohesion (kPa) for wedges.
    pub friction: f64,
    pub cohesion: f64,
}

impl Default for JointSetCfg {
    fn default() -> Self {
        JointSetCfg {
            name: String::new(),
            dip: 0.0,
            dipdir: 0.0,
            friction: 30.0,
            cohesion: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RockMechConfig {
    pub src_joints: bool,
    pub src_traces: bool,
    pub contours: bool,
    pub n_sets: usize,
    pub min_angle: f64,
    pub max_angle: f64,
    pub sets: Vec<JointSetCfg>,
    pub face_dip: f64,
    pub face_dipdir: f64,
    pub friction: f64,
    pub lateral: f64,
    pub axis_trend: f64,
    pub axis_plunge: f64,
    pub width: f64,
    pub height: f64,
    pub max_length: f64,
    pub set_a: usize,
    pub set_b: usize,
    pub set_c: usize,
    pub gamma: f64,
    pub support: f64,
    pub chainage: f64,
    pub draw_radius: f64,
    pub depth: f64,
    pub k_h: f64,
    pub k_hmin: f64,
    pub az_h: f64,
    /// 0 circle, 1 equivalent radius of width x height.
    pub opening: usize,
    pub radius: f64,
    pub e_mass: f64,
    pub nu: f64,
    pub install: f64,
    pub use_shotcrete: bool,
    pub sc_t: f64,
    pub sc_sigma: f64,
    pub sc_e: f64,
    pub use_bolts: bool,
    pub b_length: f64,
    pub b_diam: f64,
    pub b_sc: f64,
    pub b_sl: f64,
    pub b_tbf: f64,
    pub b_q: f64,
    /// 0 square, 1 rectangular, 2 rib.
    pub pl_shape: usize,
    pub pl_width: f64,
    pub pl_length: f64,
    pub pl_height: f64,
    pub pl_room: f64,
    pub pl_ucs: f64,
    /// 0 Lunder-Pakalnis, 1 Obert-Duvall, 2 Bieniawski, 3 Hedley-Grant.
    pub pl_formula: usize,
    pub sp_span: f64,
    pub sp_length: f64,
    pub sp_thickness: f64,
    pub sp_gravity: f64,
    pub sp_dip: f64,
    pub sp_critical: f64,
    /// 0 Kirsch, 1 convergence-confinement.
    pub draw_chart: usize,
    pub draw_size: f64,
}

impl Default for RockMechConfig {
    fn default() -> Self {
        RockMechConfig {
            src_joints: true,
            src_traces: true,
            contours: true,
            n_sets: 0,
            min_angle: 20.0,
            max_angle: 30.0,
            sets: Vec::new(),
            face_dip: 60.0,
            face_dipdir: 90.0,
            friction: 30.0,
            lateral: 20.0,
            axis_trend: 0.0,
            axis_plunge: 0.0,
            width: 4.0,
            height: 4.0,
            max_length: 0.0,
            set_a: 1,
            set_b: 2,
            set_c: 3,
            gamma: 27.0,
            support: 0.0,
            chainage: 0.0,
            draw_radius: 50.0,
            depth: 300.0,
            k_h: 1.5,
            k_hmin: 1.0,
            az_h: 0.0,
            opening: 0,
            radius: 2.5,
            e_mass: 0.0,
            nu: 0.25,
            install: 2.0,
            use_shotcrete: false,
            sc_t: 0.1,
            sc_sigma: 30.0,
            sc_e: 30_000.0,
            use_bolts: false,
            b_length: 3.0,
            b_diam: 0.025,
            b_sc: 1.5,
            b_sl: 1.5,
            b_tbf: 0.15,
            b_q: 0.05,
            pl_shape: 0,
            pl_width: 8.0,
            pl_length: 8.0,
            pl_height: 5.0,
            pl_room: 6.0,
            pl_ucs: 0.0,
            pl_formula: 0,
            sp_span: 10.0,
            sp_length: 100.0,
            sp_thickness: 20.0,
            sp_gravity: 2.7,
            sp_dip: 70.0,
            sp_critical: 0.0,
            draw_chart: 0,
            draw_size: 100.0,
        }
    }
}
