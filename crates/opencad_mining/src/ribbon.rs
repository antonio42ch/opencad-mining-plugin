//! Ribbon tab: one group per domain. Labels follow the UI language.

use std::sync::OnceLock;

use ocs_plugin_api::ribbon::{CadModule, IconKind, ModuleEvent, RibbonGroup, RibbonItem, ToolDef};

use crate::i18n::s;

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
                    title: t.grp_geology,
                    tools: vec![
                        large("MIN_GM_PANEL", t.tool_panel, icon!("geomap")),
                        large("MIN_GM_TRACE", t.tool_trace, icon!("trace")),
                    ],
                },
            ]
        })
    }
}
