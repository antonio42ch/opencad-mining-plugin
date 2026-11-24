//! Underground excavation solids (galleries, ramps, raises).
//!
//! Every method produces a list of *rings*: planar sections perpendicular to
//! the axis, all with the same vertex count and order. [`mesh::ClosedMesh`]
//! joins them into a watertight, outward-oriented mesh that any CAD can
//! store (no loft kernel needed).
//!
//! [`alignment`] designs axes (straights + curves with grade) and [`cut`]
//! intersects meshes with planes to produce cross-sections.

pub mod alignment;
pub mod axis;
pub mod boundary;
pub mod clip;
pub mod cut;
pub mod error;
pub mod frame;
pub mod inside;
pub mod mesh;
pub mod polyline;
pub mod section;
pub mod sheet;

pub use error::SolidError;

pub type Ring = Vec<mining_core::Vec3>;
