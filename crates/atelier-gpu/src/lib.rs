//! GPU rendering: device management, tile compositor, WGSL shaders.
//!
//! Invariant (CLAUDE.md): this is the only crate that imports wgpu.

pub mod checkerboard;
pub mod compositor;
pub mod viewport;

pub use checkerboard::{CheckerParams, CheckerboardRenderer};
pub use compositor::GpuCompositor;
pub use viewport::Viewport;
