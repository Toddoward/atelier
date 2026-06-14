//! Raster engine: CPU reference compositor and blend math (spec 0003),
//! composite op list shared with the GPU compositor (spec 0004).
//! The CPU path is the source of truth the GPU must match within 1 LSB
//! (CLAUDE.md invariant, D-9).

pub mod adjust;
pub mod blend;
pub mod brush;
pub mod compositor;
pub mod fill;
pub mod ops;
pub mod raster_vector;
pub mod resample;
pub mod selection;

pub use adjust::{apply_tile, target_tiles, Adjustment};
pub use blend::{blend_rgb, dissolve_keeps};
pub use brush::{segment_tiles, stamp_segment, stamp_segment_clipped, BrushParams};
pub use compositor::composite_rgba8;
pub use fill::{fill_region, gradient_region};
pub use raster_vector::rasterize_vector;
pub use resample::{resample_layer, sample_bilinear, transform_layer};

/// The one true f32 -> u8 quantization both compositors use.
#[inline]
pub fn quantize_rgba8(c: f32) -> u8 {
    (c * 255.0 + 0.5).clamp(0.0, 255.0) as u8
}
