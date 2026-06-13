//! Document model, layer tree, commands/undo. No GPU/UI deps (CLAUDE.md invariant).
//!
//! All mutations go through [`Command`] objects executed against a [`Document`]
//! via [`History`]; UI code never mutates the model directly.

pub mod adjust;
pub mod blend;
pub mod command;
pub mod document;
pub mod history;
pub mod mask;
pub mod node;
pub mod tile;

pub use adjust::Adjustment;
pub use blend::BlendMode;
pub use command::Command;
pub use document::{Document, ProjectFocus};
pub use history::{Editor, History};
pub use mask::{CombineOp, Mask};
pub use node::{LayerProps, Node, NodeId, NodeKind, PlaceholderArt, RasterContent};
pub use tile::{Tile, TileCoord, TileMap, TILE_SIZE};
