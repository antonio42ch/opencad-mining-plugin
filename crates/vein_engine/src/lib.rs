//! Narrow-vein modelling (no CAD, no UI).
//!
//! The vein is described in its own frame (`frame`): along strike, up dip
//! and along the normal. Its hanging wall and footwall are surfaces
//! interpolated in that frame (`surfaces`) from mapped contacts and from the
//! vein intercepts of drillholes and channels (`contacts`), so steep and
//! vertical veins work like flat ones.

pub mod contacts;
pub mod frame;
pub mod surfaces;
