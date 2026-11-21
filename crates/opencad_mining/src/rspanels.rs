//! Panels of the resource-estimation modules (built from `openmining_ops`).

use ocs_plugin_api::dialog::PanelDef;

use crate::config::ProjectConfig;
use crate::opsbind;

pub const COMP: &str = "min_rs_comp";
pub const VARIO: &str = "min_rs_vario";
pub const BM: &str = "min_rs_bm";
pub const ALL: [&str; 3] = [COMP, VARIO, BM];

pub const COMP_STATUS: &str = "rs_comp_status";
pub const COMP_CHART: &str = "rs_comp_chart";
pub const VARIO_STATUS: &str = "rs_v_status";
pub const VARIO_CHART: &str = "rs_v_chart";
pub const BM_STATUS: &str = "rs_b_status";
pub const BM_CHART: &str = "rs_b_chart";

/// Chart control of a resource panel.
pub fn chart_of(panel: &str) -> &'static str {
    match panel {
        COMP => COMP_CHART,
        VARIO => VARIO_CHART,
        _ => BM_CHART,
    }
}

/// `svg` is the current chart (re-registering the panel with another one
/// refreshes the image).
pub fn build(panel: &str, c: &ProjectConfig, lists: &opsbind::Lists, svg: &str) -> PanelDef {
    opsbind::panel_with(panel, c, lists, &[(chart_of(panel), svg)])
}
