//! Raster engine: CPU reference compositor and blend math (spec 0003),
//! composite op list shared with the GPU compositor (spec 0004).
//! The CPU path is the source of truth the GPU must match within 1 LSB
//! (CLAUDE.md invariant, D-9).

pub mod blend;
pub mod brush;
pub mod compositor;
pub mod ops;

pub use blend::{blend_rgb, dissolve_keeps};
pub use brush::{segment_tiles, stamp_segment, BrushParams};
pub use compositor::composite_rgba8;

/// The one true f32 -> u8 quantization both compositors use.
#[inline]
pub fn quantize_rgba8(c: f32) -> u8 {
    (c * 255.0 + 0.5).clamp(0.0, 255.0) as u8
}
