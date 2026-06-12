//! Raster engine: CPU reference compositor and blend math (spec 0003).
//! Brush engine arrives in spec 0005. The CPU path is the source of truth the
//! GPU compositor must match within 1 LSB (CLAUDE.md invariant, D-9).

pub mod blend;
pub mod compositor;

pub use blend::{blend_rgb, dissolve_keeps};
pub use compositor::composite_rgba8;
