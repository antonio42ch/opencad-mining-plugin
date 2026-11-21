//! Headless drillhole engine (no CAD, no UI).
//!
//! Pipeline: [`table`] (CSV/XLSX -> rows) -> [`import`] (rows -> records,
//! with automatic or manual column mapping) -> [`validate`] (R01..R06) ->
//! [`pipeline`] (desurvey + interval mapping per hole, R07) -> [`color`].

pub mod channels;
pub mod color;
pub mod desurvey;
pub mod error;
pub mod import;
pub mod mapper;
pub mod model;
pub mod pipeline;
pub mod table;
pub mod validate;

pub use error::ImportError;
pub use model::*;
