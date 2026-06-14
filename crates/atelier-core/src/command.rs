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
prop_command!(SetClip, bool, clip, "Clipping Mask", merge: false);

fn raster_content_mut(doc: &mut Document, id: NodeId) -> &mut crate::RasterContent {
    match &mut doc.node_mut(id).expect("node present").kind {
        crate::NodeKind::Raster(content) => content,
        _ => panic!("raster command on non-raster node"),
    }
}

/// Mutable placement offset of a `Raster` or `Smart` node (spec 0054).
fn offset_mut(doc: &mut Document, id: NodeId) -> &mut [i32; 2] {
    match &mut doc.node_mut(id).expect("node present").kind {
        crate::NodeKind::Raster(c) => &mut c.offset,
        crate::NodeKind::Smart(c) => &mut c.offset,
        _ => panic!("offset command on a node without an offset"),
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
            crate::NodeKind::Smart(c) => c.offset,
            _ => panic!("offset command on a node without an offset"),
        };
        Self { id, old, new }
    }
}

impl Command for SetOffset {
    fn label(&self) -> String {
        "Move Layer".into()
    }
    fn apply(&mut self, doc: &mut Document) {
        *offset_mut(doc, self.id) = self.new;
    }
    fn revert(&mut self, doc: &mut Document) {
        *offset_mut(doc, self.id) = self.old;
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

/// Set a smart object's non-destructive scale factor (spec 0055). Undoable.
#[derive(Debug)]
pub struct SetSmartScale {
    pub id: NodeId,
    pub old: [f32; 2],
    pub new: [f32; 2],
}

impl SetSmartScale {
    pub fn new(doc: &Document, id: NodeId, new: [f32; 2]) -> Self {
        let old = match &doc.node(id).expect("node present").kind {
            crate::NodeKind::Smart(c) => c.scale,
            _ => panic!("SetSmartScale on a non-smart node"),
        };
        Self { id, old, new }
    }
}

impl Command for SetSmartScale {
    fn label(&self) -> String {
        "Scale Smart Object".into()
    }
    fn apply(&mut self, doc: &mut Document) {
        if let crate::NodeKind::Smart(c) = &mut doc.node_mut(self.id).expect("node present").kind {
            c.scale = self.new;
        }
    }
    fn revert(&mut self, doc: &mut Document) {
        if let crate::NodeKind::Smart(c) = &mut doc.node_mut(self.id).expect("node present").kind {
            c.scale = self.old;
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Set a smart object's non-destructive rotation (radians). Undoable (spec 0056).
#[derive(Debug)]
pub struct SetSmartRotation {
    pub id: NodeId,
    pub old: f32,
    pub new: f32,
}

impl SetSmartRotation {
    pub fn new(doc: &Document, id: NodeId, new: f32) -> Self {
        let old = match &doc.node(id).expect("node present").kind {
            crate::NodeKind::Smart(c) => c.rotation,
            _ => panic!("SetSmartRotation on a non-smart node"),
        };
        Self { id, old, new }
    }
}

impl Command for SetSmartRotation {
    fn label(&self) -> String {
        "Rotate Smart Object".into()
    }
    fn apply(&mut self, doc: &mut Document) {
        if let crate::NodeKind::Smart(c) = &mut doc.node_mut(self.id).expect("node present").kind {
            c.rotation = self.new;
        }
    }
    fn revert(&mut self, doc: &mut Document) {
        if let crate::NodeKind::Smart(c) = &mut doc.node_mut(self.id).expect("node present").kind {
            c.rotation = self.old;
        }
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

/// Bake a raster layer's mask into its pixel alpha and clear the mask
/// (spec 0049). Undoable: restores the pre-bake tiles and mask.
#[derive(Debug)]
pub struct ApplyLayerMask {
    pub id: NodeId,
    old_tiles: Option<crate::TileMap>,
    old_mask: Option<crate::Mask>,
}

impl ApplyLayerMask {
    pub fn new(_doc: &Document, id: NodeId) -> Self {
        Self { id, old_tiles: None, old_mask: None }
    }
}

impl Command for ApplyLayerMask {
    fn label(&self) -> String {
        "Apply Layer Mask".into()
    }
    fn apply(&mut self, doc: &mut Document) {
        let Some(crate::NodeKind::Raster(c)) = doc.node_mut(self.id).map(|n| &mut n.kind) else {
            return;
        };
        self.old_tiles = Some(c.tiles.clone());
        self.old_mask = c.mask.clone();
        let Some(mask) = c.mask.take() else { return };
        let off = c.offset;
        let t = crate::TILE_SIZE as i32;
        let coords: Vec<crate::TileCoord> = c.tiles.tiles().map(|(k, _)| *k).collect();
        for (tx, ty) in coords {
            for iy in 0..t {
                for ix in 0..t {
                    let (lx, ly) = (tx * t + ix, ty * t + iy);
                    let mut px = c.tiles.pixel(lx, ly);
                    if px[3] == 0 {
                        continue;
                    }
                    let cov = mask.get(lx + off[0], ly + off[1]) as u32;
                    px[3] = (px[3] as u32 * cov / 255) as u8;
                    c.tiles.set_pixel(lx, ly, px);
                }
            }
        }
        c.tiles.prune_blank();
    }
    fn revert(&mut self, doc: &mut Document) {
        if let Some(crate::NodeKind::Raster(c)) = doc.node_mut(self.id).map(|n| &mut n.kind) {
            if let Some(t) = self.old_tiles.take() {
                c.tiles = t;
            }
            c.mask = self.old_mask.take();
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Set or clear a raster layer's mask (spec 0047). Undoable.
#[derive(Debug)]
pub struct SetLayerMask {
    pub id: NodeId,
    old: Option<crate::Mask>,
    new: Option<crate::Mask>,
}

impl SetLayerMask {
    pub fn new(doc: &Document, id: NodeId, new: Option<crate::Mask>) -> Self {
        let old = match &doc.node(id).expect("node present").kind {
            crate::NodeKind::Raster(c) => c.mask.clone(),
            _ => None,
        };
        Self { id, old, new }
    }
    fn set(&self, doc: &mut Document, m: Option<crate::Mask>) {
        if let Some(crate::NodeKind::Raster(c)) = doc.node_mut(self.id).map(|n| &mut n.kind) {
            c.mask = m;
        }
    }
}

impl Command for SetLayerMask {
    fn label(&self) -> String {
        "Layer Mask".into()
    }
    fn apply(&mut self, doc: &mut Document) {
        self.set(doc, self.new.clone());
    }
    fn revert(&mut self, doc: &mut Document) {
        self.set(doc, self.old.clone());
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

/// Replace a vector layer's shapes (direct-select anchor edits — spec 0017).
/// Snapshots the whole shapes vec; mergeable so one drag = one undo entry.
#[derive(Debug)]
pub struct SetVectorShapes {
    pub id: NodeId,
    pub old: Vec<crate::atelier_vector::Shape>,
    pub new: Vec<crate::atelier_vector::Shape>,
}

impl SetVectorShapes {
    pub fn new(doc: &Document, id: NodeId, new: Vec<crate::atelier_vector::Shape>) -> Self {
        let old = match &doc.node(id).expect("node present").kind {
            crate::NodeKind::Vector(c) => c.shapes.clone(),
            _ => panic!("SetVectorShapes on non-vector node"),
        };
        Self { id, old, new }
    }

    fn set(&self, doc: &mut Document, shapes: Vec<crate::atelier_vector::Shape>) {
        if let crate::NodeKind::Vector(c) = &mut doc.node_mut(self.id).expect("node").kind {
            c.shapes = shapes;
        }
    }
}

impl Command for SetVectorShapes {
    fn label(&self) -> String {
        "Edit Path".into()
    }
    fn apply(&mut self, doc: &mut Document) {
        self.set(doc, self.new.clone());
    }
    fn revert(&mut self, doc: &mut Document) {
        self.set(doc, self.old.clone());
    }
    fn try_merge(&mut self, next: &dyn Any) -> bool {
        if let Some(n) = next.downcast_ref::<Self>() {
            if n.id == self.id {
                self.new = n.new.clone();
                return true;
            }
        }
        false
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Several commands applied/undone as one history entry (spec 0029). Applied in
/// order, reverted in reverse.
#[derive(Debug)]
pub struct Batch {
    cmds: Vec<Box<dyn Command>>,
    label: String,
}

impl Batch {
    pub fn new(cmds: Vec<Box<dyn Command>>, label: impl Into<String>) -> Self {
        Self { cmds, label: label.into() }
    }

    pub fn is_empty(&self) -> bool {
        self.cmds.is_empty()
    }
}

impl Command for Batch {
    fn label(&self) -> String {
        self.label.clone()
    }
    fn apply(&mut self, doc: &mut Document) {
        for c in &mut self.cmds {
            c.apply(doc);
        }
    }
    fn revert(&mut self, doc: &mut Document) {
        for c in self.cmds.iter_mut().rev() {
            c.revert(doc);
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Replace the whole layer tree with one pre-built raster layer (Flatten
/// Image — spec 0040). The flattened pixels are composited by the app and
/// passed in, keeping core free of the compositor.
#[derive(Debug)]
pub struct FlattenDocument {
    raster_id: NodeId,
    raster: Node,
    /// Captured on apply (root children subtrees, in original order).
    removed: Vec<ExtractedSubtree>,
    label: String,
}

impl FlattenDocument {
    pub fn new(doc: &mut Document, raster: Node) -> Self {
        Self {
            raster_id: doc.alloc_id(),
            raster,
            removed: Vec::new(),
            label: "Flatten Image".into(),
        }
    }
}

impl Command for FlattenDocument {
    fn label(&self) -> String {
        self.label.clone()
    }
    fn apply(&mut self, doc: &mut Document) {
        let root = doc.root();
        let children = doc.children(root).to_vec();
        self.removed.clear();
        // Remove last-first so each removal's recorded index is the original one.
        for id in children.iter().rev() {
            self.removed.push(doc.remove_subtree(*id).expect("child present"));
        }
        doc.insert_node(self.raster_id, self.raster.clone(), root, 0)
            .expect("root accepts raster");
    }
    fn revert(&mut self, doc: &mut Document) {
        doc.remove_subtree(self.raster_id).expect("flattened raster present");
        // `removed` is last→first; restore first→last at original indices.
        for (nodes, parent, index) in std::mem::take(&mut self.removed).into_iter().rev() {
            doc.restore_subtree(nodes, parent, index).expect("restore child");
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Merge specific top-level layers into one pre-built raster (Merge Visible —
/// spec 0042), leaving the others in place. `targets` must be root children;
/// the raster is inserted at the top. App composites the pixels.
#[derive(Debug)]
pub struct MergeVisible {
    raster_id: NodeId,
    raster: Node,
    targets: Vec<NodeId>,
    removed: Vec<ExtractedSubtree>,
    label: String,
}

impl MergeVisible {
    pub fn new(doc: &mut Document, raster: Node, targets: Vec<NodeId>) -> Self {
        Self {
            raster_id: doc.alloc_id(),
            raster,
            targets,
            removed: Vec::new(),
            label: "Merge Visible".into(),
        }
    }
}

impl Command for MergeVisible {
    fn label(&self) -> String {
        self.label.clone()
    }
    fn apply(&mut self, doc: &mut Document) {
        let root = doc.root();
        // Remove targets high-index-first so recorded indices are original.
        let mut ordered: Vec<(NodeId, usize)> = self
            .targets
            .iter()
            .filter_map(|&id| doc.children(root).iter().position(|&c| c == id).map(|i| (id, i)))
            .collect();
        ordered.sort_by_key(|&(_, i)| std::cmp::Reverse(i));
        self.removed.clear();
        for (id, _) in &ordered {
            self.removed.push(doc.remove_subtree(*id).expect("target present"));
        }
        doc.insert_node(self.raster_id, self.raster.clone(), root, 0)
            .expect("root accepts raster");
    }
    fn revert(&mut self, doc: &mut Document) {
        doc.remove_subtree(self.raster_id).expect("merged raster present");
        let mut rem = std::mem::take(&mut self.removed);
        // Restore ascending by original index so positions reconstruct.
        rem.sort_by_key(|(_, _, index)| *index);
        for (nodes, parent, index) in rem {
            doc.restore_subtree(nodes, parent, index).expect("restore target");
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Group sibling nodes under a new group (DOC-2, spec 0028). All `ids` must
/// share a parent. The group takes the position of the topmost member; members
/// keep their relative order inside it.
#[derive(Debug)]
pub struct GroupNodes {
    ids: Vec<NodeId>,
    group_id: NodeId,
    name: String,
    /// Captured on apply for an exact revert.
    parent: Option<NodeId>,
    original_children: Vec<NodeId>,
    label: String,
}

impl GroupNodes {
    /// Returns None if `ids` is empty or not all siblings under one parent.
    pub fn new(doc: &mut Document, ids: &[NodeId], name: impl Into<String>) -> Option<Self> {
        let first = *ids.first()?;
        let parent = doc.node(first)?.parent?;
        if !ids.iter().all(|&i| doc.node(i).map(|n| n.parent) == Some(Some(parent))) {
            return None;
        }
        Some(Self {
            ids: ids.to_vec(),
            group_id: doc.alloc_id(),
            name: name.into(),
            parent: None,
            original_children: Vec::new(),
            label: "Group Layers".into(),
        })
    }

    pub fn group_id(&self) -> NodeId {
        self.group_id
    }
}

impl Command for GroupNodes {
    fn label(&self) -> String {
        self.label.clone()
    }
    fn apply(&mut self, doc: &mut Document) {
        let parent = doc.node(self.ids[0]).expect("member present").parent.expect("non-root");
        let siblings = doc.children(parent).to_vec();
        self.parent = Some(parent);
        self.original_children = siblings.clone();
        // Members in their current sibling order; group goes where the topmost sits.
        let members: Vec<NodeId> =
            siblings.iter().copied().filter(|c| self.ids.contains(c)).collect();
        let g_index = siblings
            .iter()
            .position(|c| self.ids.contains(c))
            .expect("member is a child");
        let mut group = Node::group(self.name.clone());
        group.props.blend = crate::BlendMode::PassThrough;
        doc.insert_node(self.group_id, group, parent, g_index).expect("valid group insert");
        for m in members {
            doc.move_node(m, self.group_id, usize::MAX).expect("move into group");
        }
    }
    fn revert(&mut self, doc: &mut Document) {
        let parent = self.parent.expect("applied before revert");
        // Move members out (anywhere), drop the empty group, restore order.
        for &m in &self.ids {
            doc.move_node(m, parent, 0).expect("move out of group");
        }
        doc.remove_subtree(self.group_id).expect("group present");
        doc.set_children_order(parent, self.original_children.clone()).expect("restore order");
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Ungroup a group: move its children into the parent at the group's position,
/// then remove the (now empty) group. Spec 0028.
#[derive(Debug)]
pub struct UngroupNode {
    group_id: NodeId,
    /// Captured on apply for revert.
    restore: Option<(NodeId, usize, Node, Vec<NodeId>)>,
    label: String,
}

impl UngroupNode {
    pub fn new(group_id: NodeId) -> Self {
        Self { group_id, restore: None, label: "Ungroup".into() }
    }
}

impl Command for UngroupNode {
    fn label(&self) -> String {
        self.label.clone()
    }
    fn apply(&mut self, doc: &mut Document) {
        let node = doc.node(self.group_id).expect("group present");
        let parent = node.parent.expect("non-root group");
        let children = node.children.clone();
        let gi = doc.children(parent).iter().position(|&c| c == self.group_id).expect("child");
        // Empty-group template for revert (props preserved, children cleared).
        let mut template = node.clone();
        template.children.clear();
        self.restore = Some((parent, gi, template, children.clone()));
        // Move children out, taking the group's slot in order.
        for (k, &c) in children.iter().enumerate() {
            doc.move_node(c, parent, gi + k).expect("move child out");
        }
        doc.remove_subtree(self.group_id).expect("empty group removed");
    }
    fn revert(&mut self, doc: &mut Document) {
        let (parent, gi, template, children) =
            self.restore.take().expect("applied before revert");
        doc.insert_node(self.group_id, template, parent, gi).expect("reinsert group");
        for (k, &c) in children.iter().enumerate() {
            doc.move_node(c, self.group_id, k).expect("move child back in");
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Insert a pre-built subtree (e.g. a duplicated layer — spec 0027). Built via
/// `Document::clone_subtree`; apply restores it, revert removes it by root.
#[derive(Debug)]
pub struct InsertSubtree {
    pub root: NodeId,
    pub nodes: Vec<(NodeId, Node)>,
    pub parent: NodeId,
    pub index: usize,
    label: String,
}

impl InsertSubtree {
    pub fn new(
        root: NodeId,
        nodes: Vec<(NodeId, Node)>,
        parent: NodeId,
        index: usize,
        label: impl Into<String>,
    ) -> Self {
        Self { root, nodes, parent, index, label: label.into() }
    }
}

impl Command for InsertSubtree {
    fn label(&self) -> String {
        self.label.clone()
    }
    fn apply(&mut self, doc: &mut Document) {
        doc.restore_subtree(self.nodes.clone(), self.parent, self.index)
            .expect("valid insert target");
    }
    fn revert(&mut self, doc: &mut Document) {
        doc.remove_subtree(self.root).expect("subtree present to remove");
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Swap a node's `kind` wholesale (e.g. rasterize a vector layer — INT-2,
/// spec 0023). Props/children/parent are untouched.
#[derive(Debug)]
pub struct ReplaceNodeKind {
    pub id: NodeId,
    pub old: Option<crate::NodeKind>,
    pub new: Option<crate::NodeKind>,
    label: String,
}

impl ReplaceNodeKind {
    pub fn new(doc: &Document, id: NodeId, new: crate::NodeKind, label: impl Into<String>) -> Self {
        let old = doc.node(id).map(|n| n.kind.clone());
        Self { id, old, new: Some(new), label: label.into() }
    }
}

impl Command for ReplaceNodeKind {
    fn label(&self) -> String {
        self.label.clone()
    }
    fn apply(&mut self, doc: &mut Document) {
        if let (Some(node), Some(new)) = (doc.node_mut(self.id), self.new.clone()) {
            node.kind = new;
        }
    }
    fn revert(&mut self, doc: &mut Document) {
        if let (Some(node), Some(old)) = (doc.node_mut(self.id), self.old.clone()) {
            node.kind = old;
        }
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

    #[test]
    fn set_offset_moves_smart_object_and_reverts() {
        use crate::SmartContent;
        let mut doc = Document::new([32, 32], ProjectFocus::Raster);
        let root = doc.root();
        let inner = Document::new([32, 32], ProjectFocus::Raster);
        let smart = Node::new(
            LayerProps::named("smart"),
            NodeKind::Smart(SmartContent { doc: Box::new(inner), offset: [0, 0], scale: [1.0, 1.0], rotation: 0.0 }),
        );
        let mut add = AddNode::new(&mut doc, smart, root, 0);
        add.apply(&mut doc);
        let id = add.id;

        let mut cmd = SetOffset::new(&doc, id, [5, 7]);
        cmd.apply(&mut doc);
        let off = |d: &Document| match &d.node(id).unwrap().kind {
            NodeKind::Smart(c) => c.offset,
            _ => panic!("smart expected"),
        };
        assert_eq!(off(&doc), [5, 7], "smart object moved");
        cmd.revert(&mut doc);
        assert_eq!(off(&doc), [0, 0], "revert restores the smart object's offset");
    }

    #[test]
    fn set_smart_scale_applies_and_reverts() {
        use crate::SmartContent;
        let mut doc = Document::new([16, 16], ProjectFocus::Raster);
        let root = doc.root();
        let smart = Node::new(
            LayerProps::named("s"),
            NodeKind::Smart(SmartContent::embed(Document::new([16, 16], ProjectFocus::Raster))),
        );
        let mut add = AddNode::new(&mut doc, smart, root, 0);
        add.apply(&mut doc);
        let id = add.id;

        let mut cmd = SetSmartScale::new(&doc, id, [2.0, 3.0]);
        cmd.apply(&mut doc);
        let scale = |d: &Document| match &d.node(id).unwrap().kind {
            NodeKind::Smart(c) => c.scale,
            _ => panic!("smart expected"),
        };
        assert_eq!(scale(&doc), [2.0, 3.0], "scale applied");
        cmd.revert(&mut doc);
        assert_eq!(scale(&doc), [1.0, 1.0], "revert restores unit scale");
    }

    #[test]
    fn set_smart_rotation_applies_and_reverts() {
        use crate::SmartContent;
        let mut doc = Document::new([16, 16], ProjectFocus::Raster);
        let root = doc.root();
        let smart = Node::new(
            LayerProps::named("s"),
            NodeKind::Smart(SmartContent::embed(Document::new([16, 16], ProjectFocus::Raster))),
        );
        let mut add = AddNode::new(&mut doc, smart, root, 0);
        add.apply(&mut doc);
        let id = add.id;

        let mut cmd = SetSmartRotation::new(&doc, id, 1.25);
        cmd.apply(&mut doc);
        let rot = |d: &Document| match &d.node(id).unwrap().kind {
            NodeKind::Smart(c) => c.rotation,
            _ => panic!("smart expected"),
        };
        assert_eq!(rot(&doc), 1.25, "rotation applied");
        cmd.revert(&mut doc);
        assert_eq!(rot(&doc), 0.0, "revert restores zero rotation");
    }

    fn raster_with_pixels() -> (Document, NodeId) {
        let mut doc = Document::new([32, 32], ProjectFocus::Raster);
        let root = doc.root();
        let mut tiles = crate::TileMap::new();
        tiles.fill_rect(0, 0, 16, 16, [9, 9, 9, 255]);
        let node = Node::new(
            LayerProps::named("r"),
            NodeKind::Raster(crate::RasterContent { tiles, ..Default::default() }),
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
    fn set_vector_shapes_apply_revert_and_merge() {
        use crate::atelier_vector::{Path, Shape};
        let mut doc = Document::new([32, 32], ProjectFocus::Raster);
        let root = doc.root();
        let orig = vec![Shape::filled(Path::rect(0.0, 0.0, 10.0, 10.0), [1.0; 4])];
        let mut add = AddNode::new(
            &mut doc,
            Node::new(
                LayerProps::named("v"),
                NodeKind::Vector(crate::VectorContent { shapes: orig.clone() }),
            ),
            root,
            0,
        );
        add.apply(&mut doc);
        let id = add.id;
        let baseline = doc.clone();

        let mut edited = orig.clone();
        edited[0].path.move_anchor(0, [-5.0, -5.0]);
        let mut cmd = SetVectorShapes::new(&doc, id, edited.clone());
        cmd.apply(&mut doc);
        match &doc.node(id).unwrap().kind {
            NodeKind::Vector(c) => assert_eq!(c.shapes[0].path.anchors()[0], [-5.0, -5.0]),
            _ => panic!(),
        }
        cmd.revert(&mut doc);
        assert_eq!(doc, baseline);

        // Merge coalesces same-target edits (one undo per drag).
        let next = SetVectorShapes::new(&doc, id, edited);
        assert!(cmd.try_merge(next.as_any()));
    }

    #[test]
    fn merge_visible_keeps_hidden_layers() {
        let mut doc = Document::new([8, 8], ProjectFocus::Raster);
        let root = doc.root();
        let mut ids = Vec::new();
        for n in ["v0", "h1", "v2"] {
            let mut add = AddNode::new(&mut doc, leaf(n), root, usize::MAX);
            add.apply(&mut doc);
            ids.push(add.id);
        }
        // Children top-first now: [v2, h1, v0] (each added at end → order a,b,c).
        // Use explicit visible targets v0 and v2; h1 stays.
        let raster = leaf("Merged");
        let mut cmd = MergeVisible::new(&mut doc, raster, vec![ids[0], ids[2]]);
        let baseline = doc.clone();
        cmd.apply(&mut doc);
        let kids = doc.children(root).to_vec();
        assert_eq!(kids.len(), 2, "two visible merged into one + h1 kept");
        assert!(kids.contains(&ids[1]), "hidden layer h1 retained");
        assert!(!kids.contains(&ids[0]) && !kids.contains(&ids[2]), "targets removed");

        cmd.revert(&mut doc);
        assert_eq!(doc, baseline, "revert restores exactly");
    }

    #[test]
    fn flatten_replaces_tree_and_reverts() {
        let mut doc = Document::new([8, 8], ProjectFocus::Raster);
        let root = doc.root();
        for n in ["a", "b", "c"] {
            let mut add = AddNode::new(&mut doc, leaf(n), root, usize::MAX);
            add.apply(&mut doc);
        }
        let raster = leaf("Flattened");
        let mut cmd = FlattenDocument::new(&mut doc, raster);
        // Snapshot after new() — it allocs the raster id (ids never reused).
        let baseline = doc.clone();
        cmd.apply(&mut doc);
        assert_eq!(doc.children(root).len(), 1, "tree replaced by one layer");
        assert_eq!(doc.node(doc.children(root)[0]).unwrap().props.name, "Flattened");

        cmd.revert(&mut doc);
        assert_eq!(doc, baseline, "revert restores the full tree");
    }

    #[test]
    fn batch_applies_in_order_and_reverts_in_reverse() {
        let mut doc = Document::new([16, 16], ProjectFocus::Raster);
        let root = doc.root();
        let mut add = AddNode::new(&mut doc, leaf("a"), root, 0);
        add.apply(&mut doc);
        let id = add.id;
        let baseline = doc.clone();

        let mut batch = Batch::new(
            vec![
                Box::new(SetName::new(&doc, id, "x".into())),
                Box::new(SetOpacity::new(&doc, id, 0.5)),
            ],
            "Batch",
        );
        batch.apply(&mut doc);
        assert_eq!(doc.node(id).unwrap().props.name, "x");
        assert_eq!(doc.node(id).unwrap().props.opacity, 0.5);
        batch.revert(&mut doc);
        assert_eq!(doc, baseline, "batch revert restores everything");
    }

    #[test]
    fn group_and_ungroup_round_trip() {
        let mut doc = Document::new([16, 16], ProjectFocus::Raster);
        let root = doc.root();
        let mut ids = Vec::new();
        for n in ["a", "b", "c"] {
            let mut add = AddNode::new(&mut doc, leaf(n), root, usize::MAX);
            add.apply(&mut doc);
            ids.push(add.id);
        }
        // root children = [a, b, c]
        let before = doc.children(root).to_vec();

        // Group a + c (non-contiguous).
        let mut g = GroupNodes::new(&mut doc, &[ids[0], ids[2]], "G").expect("same parent");
        let gid = g.group_id();
        g.apply(&mut doc);
        let rc = doc.children(root).to_vec();
        assert_eq!(rc.len(), 2, "group + leftover b");
        assert!(rc.contains(&gid) && rc.contains(&ids[1]));
        assert_eq!(doc.children(gid), &[ids[0], ids[2]], "members in order");
        assert_eq!(doc.node(ids[0]).unwrap().parent, Some(gid));

        // Ungroup drops the group's contents at its slot: [a, c, b].
        let mut u = UngroupNode::new(gid);
        u.apply(&mut doc);
        assert_eq!(doc.children(root), &[ids[0], ids[2], ids[1]], "contents take group slot");
        assert!(doc.node(gid).is_none());

        // Undo ungroup re-groups; undo group restores the original [a, b, c].
        u.revert(&mut doc);
        assert_eq!(doc.children(gid), &[ids[0], ids[2]], "ungroup undone");
        g.revert(&mut doc);
        assert_eq!(doc.children(root), before.as_slice(), "group undone");
        assert!(doc.node(gid).is_none());
    }

    #[test]
    fn group_rejects_cross_parent_members() {
        let mut doc = Document::new([16, 16], ProjectFocus::Raster);
        let root = doc.root();
        let mut a = AddNode::new(&mut doc, leaf("a"), root, usize::MAX);
        a.apply(&mut doc);
        let mut gadd = AddNode::new(&mut doc, Node::group("outer"), root, usize::MAX);
        gadd.apply(&mut doc);
        let mut inner = AddNode::new(&mut doc, leaf("inner"), gadd.id, 0);
        inner.apply(&mut doc);
        // a (under root) + inner (under outer) → different parents → None.
        assert!(GroupNodes::new(&mut doc, &[a.id, inner.id], "X").is_none());
    }

    #[test]
    fn insert_subtree_duplicates_with_fresh_ids() {
        let mut doc = Document::new([32, 32], ProjectFocus::Raster);
        let root = doc.root();
        // group { a }
        let mut add_g = AddNode::new(&mut doc, Node::group("g"), root, 0);
        add_g.apply(&mut doc);
        let g = add_g.id;
        let mut add_a = AddNode::new(&mut doc, leaf("a"), g, 0);
        add_a.apply(&mut doc);

        let before = doc.children(root).len();
        let (new_root, nodes) = doc.clone_subtree(g, root).expect("clone");
        assert_eq!(nodes.len(), 2, "group + child cloned");
        assert_ne!(new_root, g, "fresh root id");
        for (nid, _) in &nodes {
            assert_ne!(*nid, g);
            assert_ne!(*nid, add_a.id);
        }

        let mut cmd =
            InsertSubtree::new(new_root, nodes, root, 0, "Duplicate Layer");
        cmd.apply(&mut doc);
        assert_eq!(doc.children(root).len(), before + 1, "duplicate inserted");
        assert!(doc.node(new_root).is_some());
        assert_eq!(doc.children(new_root).len(), 1, "child cloned under new group");

        cmd.revert(&mut doc);
        assert_eq!(doc.children(root).len(), before, "revert removed duplicate");
        assert!(doc.node(new_root).is_none());
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
