//! Layer-tree nodes (DOC-1..4 model surface).

use crate::blend::BlendMode;
use serde::{Deserialize, Serialize};

/// Stable node identity. Monotonic per document, never reused, survives
/// undo/redo (a removed node reinserted by undo keeps its id — commands
/// reference nodes by id and must stay valid across revert/apply cycles).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NodeId(pub u64);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayerProps {
    pub name: String,
    pub visible: bool,
    pub locked: bool,
    /// 0.0..=1.0
    pub opacity: f32,
    pub blend: BlendMode,
    /// Clipping mask: clips to the layer below.
    pub clip: bool,
}

impl LayerProps {
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            visible: true,
            locked: false,
            opacity: 1.0,
            blend: BlendMode::Normal,
            clip: false,
        }
    }
}

/// Phase-1 stand-in for real layer content: a colored rect in document space,
/// drawn by the canvas until the raster/vector engines land (Phases 2/4).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PlaceholderArt {
    /// x, y, w, h in document pixels.
    pub bounds: [f32; 4],
    pub color: [f32; 4],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NodeKind {
    Group { expanded: bool },
    Raster(PlaceholderArt),
    Vector(PlaceholderArt),
    /// Stubs until their phases (3, 11, 10, 2): carry no data yet.
    Adjustment,
    Text,
    Smart,
    Fill,
}

impl NodeKind {
    pub fn is_group(&self) -> bool {
        matches!(self, NodeKind::Group { .. })
    }

    pub fn kind_name(&self) -> &'static str {
        match self {
            NodeKind::Group { .. } => "Group",
            NodeKind::Raster(_) => "Raster",
            NodeKind::Vector(_) => "Vector",
            NodeKind::Adjustment => "Adjustment",
            NodeKind::Text => "Text",
            NodeKind::Smart => "Smart Object",
            NodeKind::Fill => "Fill",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub props: LayerProps,
    pub kind: NodeKind,
    pub parent: Option<NodeId>,
    /// Top-most layer first (panel order). Only non-empty for groups.
    pub children: Vec<NodeId>,
}

impl Node {
    pub fn new(props: LayerProps, kind: NodeKind) -> Self {
        Self { props, kind, parent: None, children: Vec::new() }
    }

    pub fn group(name: impl Into<String>) -> Self {
        let mut props = LayerProps::named(name);
        props.blend = BlendMode::PassThrough;
        Self::new(props, NodeKind::Group { expanded: true })
    }
}
