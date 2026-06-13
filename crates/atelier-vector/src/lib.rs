//! Vector engine: path model, shapes, tessellation (spec 0012). Pure crate —
//! serde + lyon + kurbo only, no GPU/UI (D-14). The GPU renderer consumes
//! `tessellate::Mesh`.

pub mod path;
pub mod shape;
pub mod tessellate;

pub use path::{FillRule, Path, PathBuilder, Seg, SubPath};
pub use shape::{LineCap, LineJoin, Shape, Stroke, VectorContent};
pub use tessellate::{tessellate, Mesh, Vertex};
