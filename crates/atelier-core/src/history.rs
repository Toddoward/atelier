//! Undo/redo history (DOC-6) and the `Editor` pairing a document with one.

use crate::command::Command;
use crate::document::{Document, ProjectFocus};
use crate::node::NodeId;

pub struct History {
    undo: Vec<Box<dyn Command>>,
    redo: Vec<Box<dyn Command>>,
    limit: usize,
    /// While true, the next mergeable command coalesces with the top of the
    /// undo stack (one history entry per slider drag, not per frame).
    merging: bool,
    /// Bumped on every document mutation (apply/undo/redo, merged or not) —
    /// cheap cache key for recomposite/redraw decisions.
    revision: u64,
}

impl Default for History {
    fn default() -> Self {
        Self::new(200)
    }
}

impl History {
    pub fn new(limit: usize) -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            limit: limit.max(1),
            merging: false,
            revision: 0,
        }
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Apply `cmd` to `doc` and record it. Clears the redo stack.
    pub fn push_apply(&mut self, doc: &mut Document, mut cmd: Box<dyn Command>) {
        cmd.apply(doc);
        self.revision += 1;
        self.redo.clear();
        if self.merging {
            if let Some(top) = self.undo.last_mut() {
                if top.try_merge(cmd.as_any()) {
                    return;
                }
            }
        }
        self.undo.push(cmd);
        if self.undo.len() > self.limit {
            self.undo.remove(0);
        }
    }

    /// Record a command whose effect is ALREADY in the document (live-edit
    /// commit, e.g. a finished brush stroke): no apply, but redo will re-apply.
    pub fn push_committed(&mut self, cmd: Box<dyn Command>) {
        self.revision += 1;
        self.redo.clear();
        self.undo.push(cmd);
        if self.undo.len() > self.limit {
            self.undo.remove(0);
        }
    }

    /// Revision bump without a command — live preview mutation tick (the
    /// in-progress stroke), so revision-keyed caches refresh.
    pub fn touch(&mut self) {
        self.revision += 1;
    }

    /// Begin/end a coalescing run (call on slider drag start/stop).
    pub fn set_merging(&mut self, merging: bool) {
        self.merging = merging;
    }

    pub fn undo(&mut self, doc: &mut Document) -> Option<String> {
        let mut cmd = self.undo.pop()?;
        cmd.revert(doc);
        self.revision += 1;
        let label = cmd.label();
        self.redo.push(cmd);
        Some(label)
    }

    pub fn redo(&mut self, doc: &mut Document) -> Option<String> {
        let mut cmd = self.redo.pop()?;
        cmd.apply(doc);
        self.revision += 1;
        let label = cmd.label();
        self.undo.push(cmd);
        Some(label)
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Labels of applied commands, oldest first (History panel).
    pub fn undo_labels(&self) -> impl Iterator<Item = String> + '_ {
        self.undo.iter().map(|c| c.label())
    }

    /// Labels of undone commands, next-redo first.
    pub fn redo_labels(&self) -> impl Iterator<Item = String> + '_ {
        self.redo.iter().rev().map(|c| c.label())
    }

    /// Jump so that exactly `target` commands are applied (History panel click).
    pub fn jump_to(&mut self, doc: &mut Document, target: usize) {
        while self.undo.len() > target && self.can_undo() {
            self.undo(doc);
        }
        while self.undo.len() < target && self.can_redo() {
            self.redo(doc);
        }
    }

    pub fn applied_len(&self) -> usize {
        self.undo.len()
    }
}

/// One open document plus its editing state. The app owns one per document tab.
pub struct Editor {
    pub doc: Document,
    pub history: History,
    pub selection: Option<NodeId>,
    /// Saved-state marker: history length at last save; None = never saved dirty-tracking.
    saved_at: Option<usize>,
}

impl Editor {
    pub fn new(size: [u32; 2], focus: ProjectFocus) -> Self {
        Self::from_document(Document::new(size, focus))
    }

    pub fn from_document(doc: Document) -> Self {
        Self { doc, history: History::default(), selection: None, saved_at: Some(0) }
    }

    pub fn apply(&mut self, cmd: Box<dyn Command>) {
        self.history.push_apply(&mut self.doc, cmd);
    }

    pub fn is_dirty(&self) -> bool {
        self.saved_at != Some(self.history.applied_len())
    }

    pub fn mark_saved(&mut self) {
        self.saved_at = Some(self.history.applied_len());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::{AddNode, SetOpacity};
    use crate::node::{LayerProps, Node, NodeKind, PlaceholderArt, RasterContent};

    fn leaf(name: &str) -> Node {
        Node::new(
            LayerProps::named(name),
            NodeKind::Raster(RasterContent::from_placeholder(PlaceholderArt { bounds: [0.0; 4], color: [1.0; 4] })),
        )
    }

    #[test]
    fn undo_redo_round_trip_restores_states() {
        let mut ed = Editor::new([64, 64], ProjectFocus::Raster);
        let root = ed.doc.root();
        let add = AddNode::new(&mut ed.doc, leaf("a"), root, 0);
        let id = add.id;
        // Snapshot after id allocation: the id counter advances at command
        // construction and is intentionally not rolled back by undo.
        let empty = ed.doc.clone();
        ed.apply(Box::new(add));
        let one_layer = ed.doc.clone();
        ed.apply(Box::new(SetOpacity::new(&ed.doc, id, 0.5)));

        assert!(ed.history.undo(&mut ed.doc).is_some());
        assert_eq!(ed.doc, one_layer);
        assert!(ed.history.undo(&mut ed.doc).is_some());
        assert_eq!(ed.doc, empty);
        assert!(ed.history.undo(&mut ed.doc).is_none());
        assert!(ed.history.redo(&mut ed.doc).is_some());
        assert!(ed.history.redo(&mut ed.doc).is_some());
        assert_eq!(ed.doc.node(id).unwrap().props.opacity, 0.5);
    }

    #[test]
    fn new_command_clears_redo() {
        let mut ed = Editor::new([64, 64], ProjectFocus::Raster);
        let root = ed.doc.root();
        let add = AddNode::new(&mut ed.doc, leaf("a"), root, 0);
        ed.apply(Box::new(add));
        ed.history.undo(&mut ed.doc);
        assert!(ed.history.can_redo());
        let add2 = AddNode::new(&mut ed.doc, leaf("b"), root, 0);
        ed.apply(Box::new(add2));
        assert!(!ed.history.can_redo());
    }

    #[test]
    fn merging_coalesces_slider_drag() {
        let mut ed = Editor::new([64, 64], ProjectFocus::Raster);
        let root = ed.doc.root();
        let add = AddNode::new(&mut ed.doc, leaf("a"), root, 0);
        let id = add.id;
        ed.apply(Box::new(add));

        ed.history.set_merging(true);
        for v in [0.9_f32, 0.7, 0.4] {
            let cmd = SetOpacity::new(&ed.doc, id, v);
            ed.apply(Box::new(cmd));
        }
        ed.history.set_merging(false);

        assert_eq!(ed.history.applied_len(), 2); // AddNode + one merged opacity
        ed.history.undo(&mut ed.doc);
        assert_eq!(ed.doc.node(id).unwrap().props.opacity, 1.0); // back to pre-drag
    }

    #[test]
    fn jump_to_walks_both_directions() {
        let mut ed = Editor::new([64, 64], ProjectFocus::Raster);
        let root = ed.doc.root();
        for i in 0..4 {
            let add = AddNode::new(&mut ed.doc, leaf(&format!("l{i}")), root, 0);
            ed.apply(Box::new(add));
        }
        ed.history.jump_to(&mut ed.doc, 1);
        assert_eq!(ed.doc.children(root).len(), 1);
        ed.history.jump_to(&mut ed.doc, 3);
        assert_eq!(ed.doc.children(root).len(), 3);
    }

    #[test]
    fn dirty_tracking_follows_history_position() {
        let mut ed = Editor::new([64, 64], ProjectFocus::Raster);
        assert!(!ed.is_dirty());
        let root = ed.doc.root();
        let add = AddNode::new(&mut ed.doc, leaf("a"), root, 0);
        ed.apply(Box::new(add));
        assert!(ed.is_dirty());
        ed.mark_saved();
        assert!(!ed.is_dirty());
        ed.history.undo(&mut ed.doc);
        assert!(ed.is_dirty());
        ed.history.redo(&mut ed.doc);
        assert!(!ed.is_dirty());
    }
}
