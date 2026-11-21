//! Ribbon tab: one group per domain. Labels follow the UI language.

use std::sync::OnceLock;

use ocs_plugin_api::ribbon::{CadModule, IconKind, ModuleEvent, RibbonGroup, RibbonItem, ToolDef};

use crate::gti18n::g;
use crate::i18n::s;
use crate::rmi18n::m;
use crate::rsi18n::r;

macro_rules! icon {
    ($name:literal) => {
        IconKind::Svg(include_bytes!(concat!("../../../assets/icons/", $name, ".svg")))
    };
}

pub struct MiningModule;

fn large(id: &'static str, label: &'static str, icon: IconKind) -> RibbonItem {
    RibbonItem::LargeTool(ToolDef {
        id,
        label,
        icon,
        event: ModuleEvent::Command(id.to_string()),
    })
}

fn small(id: &'static str, label: &'static str, icon: IconKind) -> RibbonItem {
    RibbonItem::LabeledTool(ToolDef {
        id,
        label,
        icon,
        event: ModuleEvent::Command(id.to_string()),
    })
}

impl CadModule for MiningModule {
    fn id(&self) -> &'static str {
        "opencad_mining"
    }
    fn title(&self) -> &'static str {
        s().tab
    }
    fn ribbon_groups(&self) -> &[RibbonGroup] {
        static GROUPS: OnceLock<Vec<RibbonGroup>> = OnceLock::new();
        GROUPS.get_or_init(|| {
            let t = s();
            let rs = r();
            let gt = g();
            let rm = m();
            vec![
                RibbonGroup {
                    title: t.grp_drillholes,
                    tools: vec![
                        large("MIN_DH_PANEL", t.tool_panel, icon!("panel")),
                        small("MIN_DH_LOAD", t.tool_load, icon!("load")),
                        small("MIN_DH_DRAW", t.tool_draw, icon!("draw")),
                        small("MIN_DH_CLEAR", t.tool_clear, icon!("clear")),
                    ],
                },
                RibbonGroup {
                    title: t.grp_workings,
                    tools: vec![
                        large("MIN_UG_PANEL", t.tool_panel, icon!("workings")),
                        small("MIN_UG_AXIS", t.tool_axis, icon!("axis")),
                        small("MIN_UG_HBOUND", t.tool_hbound, icon!("hbound")),
                        small("MIN_UG_VBOUND", t.tool_vbound, icon!("vbound")),
                    ],
                },
                RibbonGroup {
                    title: t.grp_axes,
                    tools: vec![
                        large("MIN_AX_PANEL", t.tool_panel, icon!("axes")),
                        small("MIN_AX_LINE", t.tool_line, icon!("straight")),
                        small("MIN_AX_CURVE", t.tool_curve, icon!("curve")),
                    ],
                },
                RibbonGroup {
                    title: t.grp_sections,
                    tools: vec![
                        large("MIN_SC_PANEL", t.tool_panel, icon!("sections")),
                        small("MIN_SC_RUN", t.tool_sections, icon!("sectionrun")),
                    ],
                },
                RibbonGroup {
                    title: rs.grp_resources,
                    tools: vec![
                        large("MIN_RS_COMP_PANEL", rs.tool_comp, icon!("composite")),
                        large("MIN_RS_VARIO_PANEL", rs.tool_vario, icon!("variogram")),
                        large("MIN_RS_BM_PANEL", rs.tool_bm, icon!("blocks")),
                        small("MIN_RS_COMPOSITE", rs.tool_composite, icon!("composite")),
                        small("MIN_RS_ESTIMATE", rs.tool_estimate, icon!("estimate")),
                        small("MIN_RS_BMVIEW", rs.tool_view, icon!("modelview")),
                    ],
                },
                RibbonGroup {
                    title: gt.grp_geotech,
                    tools: vec![
                        large("MIN_GT_DATA_PANEL", gt.tool_data, icon!("geotech")),
                        large("MIN_GT_HB_PANEL", gt.tool_hb, icon!("hoekbrown")),
                        large("MIN_GT_MODEL_PANEL", gt.tool_model, icon!("blocks")),
                        small("MIN_GT_LOAD", gt.tool_load, icon!("load")),
                        small("MIN_GT_CLASSIFY", gt.tool_classify, icon!("geotech")),
                        small("MIN_GT_ESTIMATE", gt.tool_estimate, icon!("estimate")),
                    ],
                },
                RibbonGroup {
                    title: rm.grp,
                    tools: vec![
                        large("MIN_RM_STRUCT_PANEL", rm.tool_struct, icon!("stereonet")),
                        large("MIN_RM_DESIGN_PANEL", rm.tool_design, icon!("hoekbrown")),
                        small("MIN_ST_STEREONET", rm.tool_stereo, icon!("stereonet")),
                        small("MIN_ST_WEDGES", rm.tool_wedges, icon!("geotech")),
                    ],
                },
                RibbonGroup {
                    title: t.grp_geology,
                    tools: vec![
                        large("MIN_GM_PANEL", t.tool_panel, icon!("geomap")),
                        large("MIN_GM_TRACE", t.tool_trace, icon!("trace")),
                        small("MIN_GM_TRACE_PICK", t.tool_trace_pick, icon!("trace")),
                        large("MIN_GM_PROJECT", t.tool_project, icon!("project")),
                    ],
                },
            ]
        })
    }
}
