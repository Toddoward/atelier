//! Native .atl format and common codec glue.
//!
//! Format spec: docs/FORMAT-ATL.md. Phase 1 ships schema v0 (manifest only);
//! Phase 2 adds binary tile parts.

pub mod atl;

pub use atl::{load_atl, save_atl, AtlError, SCHEMA_VERSION};
