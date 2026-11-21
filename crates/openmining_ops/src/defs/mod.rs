//! Operation definitions per module. All but `geotech` are generated from
//! the panel strings of the plugin (labels in es/en/pt) plus the help for
//! AI clients; edit them like any source file.

mod axes;
mod channels;
mod drillholes;
mod geology;
mod geotech;
mod queries;
mod resources;
mod rockmech;
mod sections;
mod vein;
mod workings;

use crate::{OpDef, PanelSpec};

pub fn all() -> Vec<&'static OpDef> {
    let mut v: Vec<&'static OpDef> = Vec::new();
    for m in [
        queries::OPS,
        drillholes::OPS,
        channels::OPS,
        workings::OPS,
        axes::OPS,
        sections::OPS,
        geology::OPS,
        resources::OPS,
        geotech::OPS,
        rockmech::OPS,
        vein::OPS,
        vein::MODEL_OPS,
    ] {
        v.extend(m);
    }
    v
}

pub fn panels() -> Vec<&'static PanelSpec> {
    let mut v: Vec<&'static PanelSpec> = Vec::new();
    for m in [
        drillholes::PANELS,
        channels::PANELS,
        workings::PANELS,
        axes::PANELS,
        sections::PANELS,
        geology::PANELS,
        resources::PANELS,
        geotech::PANELS,
        rockmech::PANELS,
        vein::PANELS,
        vein::MODEL_PANELS,
    ] {
        v.extend(m);
    }
    v
}
