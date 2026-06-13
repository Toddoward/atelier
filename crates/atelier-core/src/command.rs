//! Undoable commands (DOC-6). Every model mutation is a `Command`; the UI
//! builds them and hands them to `History::push_apply`.

use crate::document::{Document, ExtractedSubtree};
use crate::node::{Node, NodeId};
use crate::BlendMode;
use std::any::Any;

pub trait Command: std::fmt::Debug + Send {
    /// Short human-readable label for the History panel ("Add Layer", "Opacity").
    fn label(&self) -> String;
    fn apply(&mut self, doc: &mut Document);
    fn revert(&mut self, doc: &mut Document);
    /// Coalescing hook for slider-style edits: if `next` continues this command
    /// (same target/kind), absorb it and return true. Default: never.
    fn try_merge(&mut self, _next: &dyn Any) -> bool {
        false
    }
    fn as_any(&self) -> &dyn Any;
}

/// Insert a node (with pre-captured id) under `parent` at `index`.
#[derive(Debug)]
pub struct AddNode {
    pub id: NodeId,
    pub parent: NodeId,
    pub index: usize,
    /// Holds the node when not in the document (before apply / after revert).
    pub node: Option<Node>,
    label: String,
}

impl AddNode {
    pub fn new(doc: &mut Document, node: Node, parent: NodeId, index: usize) -> Self {
        let label = format!("Add {}", node.kind.kind_name());
        Self { id: doc.alloc_id(), parent, index, node: Some(node), label }
    }
}

impl Command for AddNode {
    fn label(&self) -> String {
        self.label.clone()
    }
    fn apply(&mut self, doc: &mut Document) {
        let node = self.node.take().expect("apply called with node in hand");
        doc.insert_node(self.id, node, self.parent, self.index).expect("valid insert target");
    }
    fn revert(&mut self, doc: &mut Document) {
        let (mut removed, ..) = doc.remove_subtree(self.id).expect("node present to revert");
        debug_assert_eq!(removed.len(), 1, "AddNode only ever inserts a leaf/empty group");
        self.node = Some(removed.pop().expect("one node").1);
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Remove a node and its subtree.
#[derive(Debug)]
pub struct RemoveNode {
    pub id: NodeId,
    /// Captured on apply.
    state: Option<ExtractedSubtree>,
    label: String,
}

impl RemoveNode {
    pub fn new(doc: &Document, id: NodeId) -> Self {
        let name = doc.node(id).map(|n| n.props.name.clone()).unwrap_or_default();
        Self { id, state: None, label: format!("Delete \"{name}\"") }
    }
}

impl Command for RemoveNode {
    fn label(&self) -> String {
        self.label.clone()
    }
    fn apply(&mut self, doc: &mut Document) {
        self.state = Some(doc.remove_subtree(self.id).expect("node present"));
    }
    fn revert(&mut self, doc: &mut Document) {
        let (nodes, parent, index) = self.state.take().expect("applied before revert");
        doc.restore_subtree(nodes, parent, index).expect("restore target present");
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Move a node to a new parent/index.
#[derive(Debug)]
pub struct MoveNode {
    pub id: NodeId,
    pub to_parent: NodeId,
    pub to_index: usize,
    from: Option<(NodeId, usize)>,
    label: String,
}

impl MoveNode {
    pub fn new(doc: &Document, id: NodeId, to_parent: NodeId, to_index: usize) -> Self {
        let name = doc.node(id).map(|n| n.props.name.clone()).unwrap_or_default();
        Self { id, to_parent, to_index, from: None, label: format!("Move \"{name}\"") }
    }
}

impl Command for MoveNode {
    fn label(&self) -> String {
        self.label.clone()
    }
    fn apply(&mut self, doc: &mut Document) {
        self.from = Some(doc.move_node(self.id, self.to_parent, self.to_index).expect("valid move"));
    }
    fn revert(&mut self, doc: &mut Document) {
        let (parent, index) = self.from.take().expect("applied before revert");
        doc.move_node(self.id, parent, index).expect("revert move");
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Property edits share one generic command per field.
macro_rules! prop_command {
    ($name:ident, $ty:ty, $field:ident, $label:expr, merge: $merge:tt) => {
        #[derive(Debug)]
        pub struct $name {
            pub id: NodeId,
            pub old: $ty,
            pub new: $ty,
        }

        #[allow(clippy::clone_on_copy)] // $ty may or may not be Copy
        impl $name {
            pub fn new(doc: &Document, id: NodeId, new: $ty) -> Self {
                let old = doc.node(id).expect("node present").props.$field.clone();
                Self { id, old, new }
            }
        }

        #[allow(clippy::clone_on_copy)]
        impl Command for $name {
            fn label(&self) -> String {
                $label.to_string()
            }
            fn apply(&mut self, doc: &mut Document) {
                doc.node_mut(self.id).expect("node present").props.$field = self.new.clone();
            }
            fn revert(&mut self, doc: &mut Document) {
                doc.node_mut(self.id).expect("node present").props.$field = self.old.clone();
            }
            fn try_merge(&mut self, _next: &dyn Any) -> bool {
                prop_command!(@merge $merge, self, _next)
            }
            fn as_any(&self) -> &dyn Any {
                self
            }
        }
    };
    (@merge true, $self:ident, $next:ident) => {{
        if let Some(n) = $next.downcast_ref::<Self>() {
            if n.id == $self.id {
                $self.new = n.new.clone();
                return true;
            }
        }
        false
    }};
    (@merge false, $self:ident, $next:ident) => {
        false
    };
}

prop_command!(SetName, String, name, "Rename Layer", merge: false);
prop_command!(SetVisible, bool, visible, "Toggle Visibility", merge: false);
prop_command!(SetOpacity, f32, opacity, "Layer Opacity", merge: true);
prop_command!(SetBlend, BlendMode, blend, "Blend Mode", merge: false);

fn raster_content_mut(doc: &mut Document, id: NodeId) -> &mut crate::RasterContent {
    match &mut doc.node_mut(id).expect("node present").kind {
        crate::NodeKind::Raster(content) => content,
        _ => panic!("raster command on non-raster node"),
    }
}

/// Move a raster layer (offset in doc pixels). Mergeable: one history entry
/// per move-tool drag.
#[derive(Debug)]
pub struct SetOffset {
    pub id: NodeId,
    pub old: [i32; 2],
    pub new: [i32; 2],
}

impl SetOffset {
    pub fn new(doc: &Document, id: NodeId, new: [i32; 2]) -> Self {
        let old = match &doc.node(id).expect("node present").kind {
            crate::NodeKind::Raster(c) => c.offset,
            _ => panic!("raster command on non-raster node"),
        };
        Self { id, old, new }
    }
}

impl Command for SetOffset {
    fn label(&self) -> String {
        "Move Layer".into()
    }
    fn apply(&mut self, doc: &mut Document) {
        raster_content_mut(doc, self.id).offset = self.new;
    }
    fn revert(&mut self, doc: &mut Document) {
        raster_content_mut(doc, self.id).offset = self.old;
    }
    fn try_merge(&mut self, next: &dyn Any) -> bool {
        if let Some(n) = next.downcast_ref::<Self>() {
            if n.id == self.id {
                self.new = n.new;
                return true;
            }
        }
        false
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Pixel edits captured as before/after tile snapshots. Built after the live
/// stroke already mutated the tiles; record via `History::push_committed`.
#[derive(Debug)]
pub struct PaintTiles {
    pub id: NodeId,
    label: String,
    /// (coord, tile before, tile after); None = tile absent.
    diffs: Vec<(crate::TileCoord, Option<crate::Tile>, Option<crate::Tile>)>,
}

impl PaintTiles {
    pub fn from_capture(
        doc: &Document,
        id: NodeId,
        label: impl Into<String>,
        before: impl IntoIterator<Item = (crate::TileCoord, Option<crate::Tile>)>,
    ) -> Self {
        let tiles = match &doc.node(id).expect("node present").kind {
            crate::NodeKind::Raster(c) => &c.tiles,
            _ => panic!("raster command on non-raster node"),
        };
        let diffs = before
            .into_iter()
            .map(|(coord, before)| (coord, before, tiles.tile_at(coord).cloned()))
            .collect();
        Self { id, label: label.into(), diffs }
    }

    fn restore(&self, doc: &mut Document, use_after: bool) {
        let content = raster_content_mut(doc, self.id);
        for (coord, before, after) in &self.diffs {
            let tile = if use_after { after } else { before };
            match tile {
                Some(t) => content.tiles.insert_tile(*coord, t.clone()),
                None => content.tiles.remove_tile(*coord),
            }
        }
    }
}

impl Command for PaintTiles {
    fn label(&self) -> String {
        self.label.clone()
    }
    fn apply(&mut self, doc: &mut Document) {
        self.restore(doc, true);
    }
    fn revert(&mut self, doc: &mut Document) {
        self.restore(doc, false);
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Change the document selection (marquee/lasso/deselect — spec 0007).
/// Arc snapshots make undo cheap regardless of mask size.
#[derive(Debug)]
pub struct SetSelection {
    old: Option<std::sync::Arc<crate::Mask>>,
    new: Option<std::sync::Arc<crate::Mask>>,
    label: String,
}

impl SetSelection {
    pub fn new(
        doc: &Document,
        new: Option<std::sync::Arc<crate::Mask>>,
        label: impl Into<String>,
    ) -> Self {
        Self { old: doc.selection.clone(), new, label: label.into() }
    }
}

impl Command for SetSelection {
    fn label(&self) -> String {
        self.label.clone()
    }
    fn apply(&mut self, doc: &mut Document) {
        doc.selection = self.new.clone();
    }
    fn revert(&mut self, doc: &mut Document) {
        doc.selection = self.old.clone();
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Edit an adjustment layer's parameters (Properties panel — spec 0009).
#[derive(Debug)]
pub struct SetAdjustment {
    pub id: NodeId,
    pub old: crate::Adjustment,
    pub new: crate::Adjustment,
}

impl SetAdjustment {
    pub fn new(doc: &Document, id: NodeId, new: crate::Adjustment) -> Self {
        let old = match &doc.node(id).expect("node present").kind {
            crate::NodeKind::Adjustment(a) => *a,
            _ => panic!("SetAdjustment on non-adjustment node"),
        };
        Self { id, old, new }
    }
}

impl SetAdjustment {
    fn set(&self, doc: &mut Document, a: crate::Adjustment) {
        if let crate::NodeKind::Adjustment(slot) = &mut doc.node_mut(self.id).expect("node").kind {
            *slot = a;
        }
    }
}

impl Command for SetAdjustment {
    fn label(&self) -> String {
        "Edit Adjustment".into()
    }
    fn apply(&mut self, doc: &mut Document) {
        self.set(doc, self.new);
    }
    fn revert(&mut self, doc: &mut Document) {
        self.set(doc, self.old);
    }
    fn try_merge(&mut self, next: &dyn Any) -> bool {
        if let Some(n) = next.downcast_ref::<Self>() {
            if n.id == self.id {
                self.new = n.new;
                return true;
            }
        }
        false
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Swap a raster layer's whole tile set + offset (transform bake, spec 0010).
/// The app computes the resampled tiles via `atelier-raster`; this just stores
/// before/after for undo.
#[derive(Debug)]
pub struct ReplaceLayerTiles {
    pub id: NodeId,
    old: Option<(crate::TileMap, [i32; 2])>,
    new: Option<(crate::TileMap, [i32; 2])>,
    label: String,
}

impl ReplaceLayerTiles {
    pub fn new(
        doc: &Document,
        id: NodeId,
        new_tiles: crate::TileMap,
        new_offset: [i32; 2],
        label: impl Into<String>,
    ) -> Self {
        let old = match &doc.node(id).expect("node present").kind {
            crate::NodeKind::Raster(c) => (c.tiles.clone(), c.offset),
            _ => panic!("ReplaceLayerTiles on non-raster node"),
        };
        Self { id, old: Some(old), new: Some((new_tiles, new_offset)), label: label.into() }
    }

    fn put(&self, doc: &mut Document, src: &(crate::TileMap, [i32; 2])) {
        if let crate::NodeKind::Raster(c) = &mut doc.node_mut(self.id).expect("node").kind {
            c.tiles = src.0.clone();
            c.offset = src.1;
        }
    }
}

impl Command for ReplaceLayerTiles {
    fn label(&self) -> String {
        self.label.clone()
    }
    fn apply(&mut self, doc: &mut Document) {
        let new = self.new.clone().expect("new present");
        self.put(doc, &new);
    }
    fn revert(&mut self, doc: &mut Document) {
        let old = self.old.clone().expect("old present");
        self.put(doc, &old);
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Resample the whole image to a new pixel size (spec 0010). Snapshots every
/// affected raster layer's tiles+offset before and after.
#[derive(Debug)]
pub struct ResizeImage {
    old_size: [u32; 2],
    new_size: [u32; 2],
    /// (id, (old tiles, old offset), (new tiles, new offset))
    #[allow(clippy::type_complexity)]
    layers: Vec<(NodeId, (crate::TileMap, [i32; 2]), (crate::TileMap, [i32; 2]))>,
}

impl ResizeImage {
    pub fn new(
        doc: &Document,
        new_size: [u32; 2],
        baked: Vec<(NodeId, (crate::TileMap, [i32; 2]))>,
    ) -> Self {
        let layers = baked
            .into_iter()
            .map(|(id, new)| {
                let old = match &doc.node(id).expect("node present").kind {
                    crate::NodeKind::Raster(c) => (c.tiles.clone(), c.offset),
                    _ => panic!("ResizeImage layer is not raster"),
                };
                (id, old, new)
            })
            .collect();
        Self { old_size: doc.size, new_size, layers }
    }
}

impl Command for ResizeImage {
    fn label(&self) -> String {
        "Image Size".into()
    }
    fn apply(&mut self, doc: &mut Document) {
        doc.size = self.new_size;
        for (id, _, new) in &self.layers {
            if let crate::NodeKind::Raster(c) = &mut doc.node_mut(*id).expect("node").kind {
                c.tiles = new.0.clone();
                c.offset = new.1;
            }
        }
    }
    fn revert(&mut self, doc: &mut Document) {
        doc.size = self.old_size;
        for (id, old, _) in &self.layers {
            if let crate::NodeKind::Raster(c) = &mut doc.node_mut(*id).expect("node").kind {
                c.tiles = old.0.clone();
                c.offset = old.1;
            }
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Crop the canvas to a doc-space rect: resize + shift every raster layer's
/// offset by −rect.min (no resampling). spec 0010.
#[derive(Debug)]
pub struct CropCanvas {
    old_size: [u32; 2],
    new_size: [u32; 2],
    shift: [i32; 2],
    rasters: Vec<NodeId>,
}

impl CropCanvas {
    pub fn new(doc: &Document, rect: [i32; 4]) -> Self {
        let new_size = [(rect[2] - rect[0]).max(1) as u32, (rect[3] - rect[1]).max(1) as u32];
        let rasters = doc
            .iter_tree()
            .into_iter()
            .filter(|(id, _)| matches!(doc.node(*id).map(|n| &n.kind), Some(crate::NodeKind::Raster(_))))
            .map(|(id, _)| id)
            .collect();
        Self { old_size: doc.size, new_size, shift: [rect[0], rect[1]], rasters }
    }

    fn shift_all(&self, doc: &mut Document, dx: i32, dy: i32) {
        for &id in &self.rasters {
            if let Some(crate::NodeKind::Raster(c)) = doc.node_mut(id).map(|n| &mut n.kind) {
                c.offset[0] += dx;
                c.offset[1] += dy;
            }
        }
    }
}

impl Command for CropCanvas {
    fn label(&self) -> String {
        "Crop".into()
    }
    fn apply(&mut self, doc: &mut Document) {
        doc.size = self.new_size;
        self.shift_all(doc, -self.shift[0], -self.shift[1]);
    }
    fn revert(&mut self, doc: &mut Document) {
        doc.size = self.old_size;
        self.shift_all(doc, self.shift[0], self.shift[1]);
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Document canvas resize (anchor top-left; no resampling — RAS-5 subset).
#[derive(Debug)]
pub struct CanvasResize {
    pub old: [u32; 2],
    pub new: [u32; 2],
}

impl CanvasResize {
    pub fn new(doc: &Document, new: [u32; 2]) -> Self {
        Self { old: doc.size, new }
    }
}

impl Command for CanvasResize {
    fn label(&self) -> String {
        "Canvas Size".into()
    }
    fn apply(&mut self, doc: &mut Document) {
        doc.size = self.new;
    }
    fn revert(&mut self, doc: &mut Document) {
        doc.size = self.old;
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::ProjectFocus;
    use crate::node::{LayerProps, NodeKind, PlaceholderArt, RasterContent};

    fn leaf(name: &str) -> Node {
        Node::new(
            LayerProps::named(name),
            NodeKind::Raster(RasterContent::from_placeholder(PlaceholderArt { bounds: [0.0; 4], color: [1.0; 4] })),
        )
    }

    /// Every command must satisfy: apply → revert restores the exact document.
    #[test]
    fn apply_revert_is_identity_for_every_command() {
        let mut doc = Document::new([64, 64], ProjectFocus::Raster);
        let root = doc.root();
        let mut add_a = AddNode::new(&mut doc, leaf("a"), root, 0);
        add_a.apply(&mut doc);
        let a = add_a.id;
        let mut add_g = AddNode::new(&mut doc, Node::group("g"), root, 1);
        add_g.apply(&mut doc);
        let g = add_g.id;

        let mut cmds: Vec<Box<dyn Command>> = vec![
            Box::new(AddNode::new(&mut doc, leaf("b"), g, 0)),
            Box::new(RemoveNode::new(&doc, a)),
            Box::new(MoveNode::new(&doc, a, g, 0)),
            Box::new(SetName::new(&doc, a, "renamed".into())),
            Box::new(SetVisible::new(&doc, a, false)),
            Box::new(SetOpacity::new(&doc, a, 0.5)),
            Box::new(SetBlend::new(&doc, a, BlendMode::Multiply)),
        ];
        // Baseline after construction: AddNode::new pre-allocates its NodeId,
        // which advances the document's id counter (ids are never reused).
        let baseline = doc.clone();
        for cmd in &mut cmds {
            cmd.apply(&mut doc);
            cmd.revert(&mut doc);
            assert_eq!(doc, baseline, "{} broke apply/revert identity", cmd.label());
        }
    }

    #[test]
    fn add_then_undo_then_redo_keeps_id() {
        let mut doc = Document::new([64, 64], ProjectFocus::Raster);
        let root = doc.root();
        let mut add = AddNode::new(&mut doc, leaf("a"), root, 0);
        add.apply(&mut doc);
        let id = add.id;
        add.revert(&mut doc);
        assert!(doc.node(id).is_none());
        add.apply(&mut doc);
        assert!(doc.node(id).is_some());
    }

    fn raster_with_pixels() -> (Document, NodeId) {
        let mut doc = Document::new([32, 32], ProjectFocus::Raster);
        let root = doc.root();
        let mut tiles = crate::TileMap::new();
        tiles.fill_rect(0, 0, 16, 16, [9, 9, 9, 255]);
        let node = Node::new(
            LayerProps::named("r"),
            NodeKind::Raster(crate::RasterContent { art: None, offset: [0, 0], tiles }),
        );
        let mut add = AddNode::new(&mut doc, node, root, 0);
        add.apply(&mut doc);
        (doc, add.id)
    }

    #[test]
    fn replace_layer_tiles_apply_revert_identity() {
        let (mut doc, id) = raster_with_pixels();
        let baseline = doc.clone();
        let mut new = crate::TileMap::new();
        new.fill_rect(0, 0, 4, 4, [1, 2, 3, 255]);
        let mut cmd = ReplaceLayerTiles::new(&doc, id, new, [5, 6], "Transform Layer");
        cmd.apply(&mut doc);
        match &doc.node(id).unwrap().kind {
            NodeKind::Raster(c) => {
                assert_eq!(c.offset, [5, 6]);
                assert_eq!(c.tiles.pixel(1, 1), [1, 2, 3, 255]);
            }
            _ => panic!(),
        }
        cmd.revert(&mut doc);
        assert_eq!(doc, baseline);
    }

    #[test]
    fn crop_canvas_resizes_and_shifts_offsets() {
        let (mut doc, id) = raster_with_pixels();
        let baseline = doc.clone();
        let mut cmd = CropCanvas::new(&doc, [4, 6, 20, 22]); // 16×16 crop at (4,6)
        cmd.apply(&mut doc);
        assert_eq!(doc.size, [16, 16]);
        match &doc.node(id).unwrap().kind {
            NodeKind::Raster(c) => assert_eq!(c.offset, [-4, -6]),
            _ => panic!(),
        }
        cmd.revert(&mut doc);
        assert_eq!(doc, baseline);
    }

    #[test]
    fn resize_image_apply_revert_identity() {
        let (mut doc, id) = raster_with_pixels();
        let baseline = doc.clone();
        let mut new = crate::TileMap::new();
        new.fill_rect(0, 0, 8, 8, [4, 5, 6, 255]);
        let cmd_new = vec![(id, (new, [0, 0]))];
        let mut cmd = ResizeImage::new(&doc, [16, 16], cmd_new);
        cmd.apply(&mut doc);
        assert_eq!(doc.size, [16, 16]);
        cmd.revert(&mut doc);
        assert_eq!(doc, baseline);
    }

    #[test]
    fn set_adjustment_apply_revert_and_merge() {
        let mut doc = Document::new([16, 16], ProjectFocus::Raster);
        let root = doc.root();
        let mut add = AddNode::new(
            &mut doc,
            Node::new(
                LayerProps::named("adj"),
                NodeKind::Adjustment(crate::Adjustment::Invert),
            ),
            root,
            0,
        );
        add.apply(&mut doc);
        let id = add.id;
        let baseline = doc.clone();

        let new = crate::Adjustment::BrightnessContrast { brightness: 0.2, contrast: 0.1 };
        let mut cmd = SetAdjustment::new(&doc, id, new);
        cmd.apply(&mut doc);
        match doc.node(id).unwrap().kind {
            NodeKind::Adjustment(a) => assert_eq!(a, new),
            _ => panic!(),
        }
        cmd.revert(&mut doc);
        assert_eq!(doc, baseline);

        // Merge coalesces same-target edits.
        let next = SetAdjustment::new(&doc, id, crate::Adjustment::Invert);
        assert!(cmd.try_merge(next.as_any()));
    }

    #[test]
    fn set_selection_apply_revert_identity() {
        let mut doc = Document::new([32, 32], ProjectFocus::Raster);
        let baseline = doc.clone();
        let mut mask = crate::Mask::new();
        mask.set(3, 3, 255);
        let mut cmd =
            SetSelection::new(&doc, Some(std::sync::Arc::new(mask)), "Rectangular Select");
        cmd.apply(&mut doc);
        assert!(doc.selection.is_some());
        cmd.revert(&mut doc);
        assert_eq!(doc, baseline);

        // Deselect path round-trips too.
        let mask2 = {
            let mut m = crate::Mask::new();
            m.set(1, 1, 200);
            m
        };
        doc.selection = Some(std::sync::Arc::new(mask2));
        let with_sel = doc.clone();
        let mut clear = SetSelection::new(&doc, None, "Deselect");
        clear.apply(&mut doc);
        assert!(doc.selection.is_none());
        clear.revert(&mut doc);
        assert_eq!(doc, with_sel);
    }

    #[test]
    fn opacity_merges_same_target_only() {
        let mut doc = Document::new([64, 64], ProjectFocus::Raster);
        let root = doc.root();
        let mut add_a = AddNode::new(&mut doc, leaf("a"), root, 0);
        add_a.apply(&mut doc);
        let mut add_b = AddNode::new(&mut doc, leaf("b"), root, 1);
        add_b.apply(&mut doc);

        let mut first = SetOpacity::new(&doc, add_a.id, 0.8);
        first.apply(&mut doc);
        let mut second = SetOpacity::new(&doc, add_a.id, 0.6);
        second.apply(&mut doc);
        assert!(first.try_merge(second.as_any()));
        assert_eq!(first.new, 0.6);
        assert_eq!(first.old, 1.0);

        let other = SetOpacity::new(&doc, add_b.id, 0.3);
        assert!(!first.try_merge(other.as_any()));
        let rename = SetName::new(&doc, add_a.id, "x".into());
        assert!(!first.try_merge(rename.as_any()));
    }
}
