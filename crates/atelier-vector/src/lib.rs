//! Vector engine: path model, shapes, tessellation (spec 0012), boolean ops
//! (spec 0031). Pure crate — no GPU/UI (D-14); deps: serde, lyon, kurbo,
//! i_overlay. The GPU renderer consumes `tessellate::Mesh`.

pub mod boolean;
pub mod path;
pub mod shape;
pub mod tessellate;

pub use boolean::{boolean, BoolOp};
pub use path::{FillRule, Path, PathBuilder, Seg, SubPath};
pub use shape::{LineCap, LineJoin, Shape, Stroke, VectorContent};
pub use tessellate::{tessellate, Mesh, Vertex};
