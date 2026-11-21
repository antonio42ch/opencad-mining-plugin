//! Panels of the geotechnical module (built from `openmining_ops` defs).

use ocs_plugin_api::dialog::PanelDef;

use crate::config::ProjectConfig;
use crate::opsbind;

pub const DATA: &str = "min_gt_data";
pub const HB: &str = "min_gt_hb";
pub const MODEL: &str = "min_gt_model";
pub const ALL: [&str; 3] = [DATA, HB, MODEL];

pub const DATA_STATUS: &str = "gt_status";
pub const HB_OUT: &str = "gt_hb_out";
pub const HB_CHART: &str = "gt_hb_chart";
pub const MODEL_STATUS: &str = "gt_md_status";

pub fn data(c: &ProjectConfig) -> PanelDef {
    opsbind::panel(DATA, c, &[])
}

/// `svg` is the current envelope (re-registering refreshes it).
pub fn hb(c: &ProjectConfig, svg: &str) -> PanelDef {
    opsbind::panel(HB, c, &[(HB_CHART, svg)])
}

pub fn model(c: &ProjectConfig) -> PanelDef {
    opsbind::panel(MODEL, c, &[])
}
