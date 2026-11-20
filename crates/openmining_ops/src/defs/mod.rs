//! Operation definitions per module.

mod geotech;

use crate::{OpDef, PanelSpec};

pub fn all() -> Vec<&'static OpDef> {
    let mut v: Vec<&'static OpDef> = Vec::new();
    v.extend(geotech::OPS);
    v
}

pub fn panels() -> Vec<&'static PanelSpec> {
    let mut v: Vec<&'static PanelSpec> = Vec::new();
    v.extend(geotech::PANELS);
    v
}
