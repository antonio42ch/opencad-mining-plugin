//! Block models: geometry, domains, estimation, persistence, validation
//! and views. No CAD, no UI.

pub mod estimate;
pub mod file;
pub mod geometry;
pub mod model;
pub mod region;
pub mod report;
pub mod views;

pub use geometry::BlockGeometry;
pub use model::BlockModel;
