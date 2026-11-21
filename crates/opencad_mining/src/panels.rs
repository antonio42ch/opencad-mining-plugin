//! Dockable panels. Every panel is built from the operation definitions
//! (`openmining_ops`, see `opsbind`); buttons run the operation's command.

use ocs_plugin_api::dialog::PanelDef;

use crate::config::ProjectConfig;
use crate::opsbind;

pub const DH: &str = "min_drillholes";
pub const UG: &str = "min_workings";
pub const GM: &str = "min_geomap";
pub const AX: &str = "min_axes";
pub const SC: &str = "min_sections";
pub const ALL: [&str; 13] = [
    DH,
    UG,
    GM,
    AX,
    SC,
    crate::rspanels::COMP,
    crate::rspanels::VARIO,
    crate::rspanels::BM,
    crate::gtpanels::DATA,
    crate::gtpanels::HB,
    crate::gtpanels::MODEL,
    crate::rmcommands::STRUCT,
    crate::rmcommands::DESIGN,
];
pub const DH_STATUS: &str = "dh_status";
pub const AX_STATUS: &str = "ax_status";

/// The panel `id` built for `c` (charts empty).
pub fn build(id: &str, c: &ProjectConfig) -> PanelDef {
    opsbind::panel(id, c, &[])
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_defined_panel_is_registered_and_back() {
        let mut ids = crate::opsbind::panel_ids();
        ids.sort();
        let mut ours = super::ALL.to_vec();
        ours.sort();
        assert_eq!(ids, ours);
    }
}
