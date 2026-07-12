//! Document: layer tree + metadata. Tree mutation methods are the primitive
//! operations used by commands (`crate::command`); UI must not call them
//! directly (CLAUDE.md invariant).

use crate::node::{Node, NodeId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ProjectFocus {
    #[default]
    Raster,
    Vector,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    /// Width, height in document pixels.
    pub size: [u32; 2],
    pub focus: ProjectFocus,
    /// Color stub until Phase 6 (atelier-color): mode tag only.
    pub color_mode: String,
    /// Active selection (None = nothing selected ⇒ everything editable).
    /// Session state — not persisted (spec 0007); mutate via `SetSelection`.
    #[serde(skip)]
    pub selection: Option<std::sync::Arc<crate::Mask>>,
    nodes: BTreeMap<NodeId, Node>,
    root: NodeId,
    next_id: u64,
}

/// Result of [`Document::remove_subtree`]: nodes in removal order (subtree root
/// first, links intact) plus the original `(parent, index)` — everything
/// [`Document::restore_subtree`] needs to undo the removal.
pub type ExtractedSubtree = (Vec<(NodeId, Node)>, NodeId, usize);

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum TreeError {
    #[error("node {0:?} not found")]
    NotFound(NodeId),
    #[error("target parent {0:?} is not a group")]
    NotAGroup(NodeId),
    #[error("moving {0:?} into its own subtree would create a cycle")]
    Cycle(NodeId),
    #[error("the root group cannot be moved or removed")]
    RootImmutable,
}

impl Document {
    pub fn new(size: [u32; 2], focus: ProjectFocus) -> Self {
        let root_id = NodeId(0);
        let mut nodes = BTreeMap::new();
        nodes.insert(root_id, Node::group("__root__"));
        Self {
            size,
            focus,
            color_mode: "RGB8".into(),
            selection: None,
            nodes,
            root: root_id,
            next_id: 1,
        }
    }

    pub fn root(&self) -> NodeId {
        self.root
    }

    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(&id)
    }

    pub fn node_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        self.nodes.get_mut(&id)
    }

    pub fn children(&self, id: NodeId) -> &[NodeId] {
        self.nodes.get(&id).map(|n| n.children.as_slice()).unwrap_or(&[])
    }

    /// Total nodes including the root group.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn alloc_id(&mut self) -> NodeId {
        let id = NodeId(self.next_id);
        self.next_id += 1;
        id
    }

    pub fn is_ancestor(&self, maybe_ancestor: NodeId, of: NodeId) -> bool {
        let mut cur = self.nodes.get(&of).and_then(|n| n.parent);
        while let Some(p) = cur {
            if p == maybe_ancestor {
                return true;
            }
            cur = self.nodes.get(&p).and_then(|n| n.parent);
        }
        false
    }

    /// Depth-first, top-of-stack-first traversal (panel order), excluding the root.
    pub fn iter_tree(&self) -> Vec<(NodeId, usize)> {
        let mut out = Vec::new();
        let mut stack: Vec<(NodeId, usize)> =
            self.children(self.root).iter().map(|&c| (c, 0)).collect();
        stack.reverse();
        while let Some((id, depth)) = stack.pop() {
            out.push((id, depth));
            let kids = self.children(id);
            for &k in kids.iter().rev() {
                stack.push((k, depth + 1));
            }
        }
        out
    }

    /// Insert `node` under `parent` at `index` (clamped) with a pre-allocated id.
    pub fn insert_node(
        &mut self,
        id: NodeId,
        mut node: Node,
        parent: NodeId,
        index: usize,
    ) -> Result<(), TreeError> {
        let p = self.nodes.get_mut(&parent).ok_or(TreeError::NotFound(parent))?;
        if !p.kind.is_group() {
            return Err(TreeError::NotAGroup(parent));
        }
        let index = index.min(p.children.len());
        p.children.insert(index, id);
        node.parent = Some(parent);
        self.nodes.insert(id, node);
        Ok(())
    }

    /// Remove a node and its whole subtree. Returns everything needed to undo:
    /// `(nodes in removal order, parent, index)`.
    pub fn remove_subtree(&mut self, id: NodeId) -> Result<ExtractedSubtree, TreeError> {
        if id == self.root {
            return Err(TreeError::RootImmutable);
        }
        let parent = self
            .nodes
            .get(&id)
            .ok_or(TreeError::NotFound(id))?
            .parent
            .ok_or(TreeError::RootImmutable)?;
        let index = self
            .children(parent)
            .iter()
            .position(|&c| c == id)
            .expect("child listed in parent");
        self.nodes.get_mut(&parent).expect("parent exists").children.remove(index);

        let mut removed = Vec::new();
        let mut stack = vec![id];
        while let Some(cur) = stack.pop() {
            let node = self.nodes.remove(&cur).expect("subtree node exists");
            stack.extend(node.children.iter().copied());
            removed.push((cur, node));
        }
        Ok((removed, parent, index))
    }

    /// Reinsert a subtree removed by [`remove_subtree`] (ids and links intact).
    pub fn restore_subtree(
        &mut self,
        removed: Vec<(NodeId, Node)>,
        parent: NodeId,
        index: usize,
    ) -> Result<(), TreeError> {
        let root_id = removed.first().map(|(id, _)| *id).ok_or(TreeError::RootImmutable)?;
        for (id, node) in removed {
            self.nodes.insert(id, node);
        }
        let p = self.nodes.get_mut(&parent).ok_or(TreeError::NotFound(parent))?;
        let index = index.min(p.children.len());
        p.children.insert(index, root_id);
        Ok(())
    }

    /// Reorder a parent's children to `order` (must be a permutation of the
    /// current children). Child `parent` links are unchanged. Spec 0028.
    pub fn set_children_order(&mut self, parent: NodeId, order: Vec<NodeId>) -> Result<(), TreeError> {
        let p = self.nodes.get_mut(&parent).ok_or(TreeError::NotFound(parent))?;
        let mut cur = p.children.clone();
        cur.sort();
        let mut want = order.clone();
        want.sort();
        if cur != want {
            return Err(TreeError::NotFound(parent)); // not a permutation
        }
        self.nodes.get_mut(&parent).expect("checked").children = order;
        Ok(())
    }

    /// Deep-clone the subtree rooted at `id` with fresh NodeIds, re-parented
    /// under `new_parent`. Returns `(new_root, nodes)` ready for
    /// [`restore_subtree`] (root first, links remapped). Spec 0027.
    pub fn clone_subtree(
        &mut self,
        id: NodeId,
        new_parent: NodeId,
    ) -> Option<(NodeId, Vec<(NodeId, Node)>)> {
        self.nodes.get(&id)?;
        // DFS, root first.
        let mut old_order = Vec::new();
        let mut stack = vec![id];
        while let Some(c) = stack.pop() {
            old_order.push(c);
            if let Some(n) = self.nodes.get(&c) {
                stack.extend(n.children.iter().copied());
            }
        }
        let mut map: BTreeMap<NodeId, NodeId> = BTreeMap::new();
        for &old in &old_order {
            let fresh = self.alloc_id();
            map.insert(old, fresh);
        }
        let mut out = Vec::with_capacity(old_order.len());
        for &old in &old_order {
            let mut n = self.nodes.get(&old).expect("subtree node").clone();
            n.parent = if old == id { Some(new_parent) } else { n.parent.map(|p| map[&p]) };
            n.children = n.children.iter().map(|c| map[c]).collect();
            out.push((map[&old], n));
        }
        Some((map[&id], out))
    }

    /// Owned deep copy of the subtree at `id` (root first, ids/links as-is) —
    /// the document-independent clipboard form (spec 0058). Pair with
    /// [`import_subtree`] on the destination document.
    pub fn snapshot_subtree(&self, id: NodeId) -> Option<Vec<(NodeId, Node)>> {
        self.nodes.get(&id)?;
        let mut out = Vec::new();
        let mut stack = vec![id];
        while let Some(c) = stack.pop() {
            let n = self.nodes.get(&c).expect("subtree node").clone();
            stack.extend(n.children.iter().copied());
            out.push((c, n));
        }
        Some(out)
    }

    /// Import a foreign subtree (from [`snapshot_subtree`], possibly another
    /// document): remap every id to a fresh local id, fix parent/child links,
    /// re-parent the root under `new_parent`. Returns `(new_root, nodes)` ready
    /// for `InsertSubtree`/[`restore_subtree`]. Spec 0058.
    pub fn import_subtree(
        &mut self,
        nodes: &[(NodeId, Node)],
        new_parent: NodeId,
    ) -> Option<(NodeId, Vec<(NodeId, Node)>)> {
        let root_old = nodes.first()?.0;
        let mut map: BTreeMap<NodeId, NodeId> = BTreeMap::new();
        for (old, _) in nodes {
            map.insert(*old, self.alloc_id());
        }
        let mut out = Vec::with_capacity(nodes.len());
        for (old, node) in nodes {
            let mut n = node.clone();
            n.parent =
                if *old == root_old { Some(new_parent) } else { n.parent.map(|p| map[&p]) };
            n.children = n.children.iter().map(|c| map[c]).collect();
            out.push((map[old], n));
        }
        Some((map[&root_old], out))
    }

    /// Move `id` to `new_parent` at `new_index`. Returns the old `(parent, index)`.
    pub fn move_node(
        &mut self,
        id: NodeId,
        new_parent: NodeId,
        new_index: usize,
    ) -> Result<(NodeId, usize), TreeError> {
        if id == self.root {
            return Err(TreeError::RootImmutable);
        }
        if id == new_parent || self.is_ancestor(id, new_parent) {
            return Err(TreeError::Cycle(id));
        }
        if !self.nodes.get(&new_parent).ok_or(TreeError::NotFound(new_parent))?.kind.is_group() {
            return Err(TreeError::NotAGroup(new_parent));
        }
        let old_parent =
            self.nodes.get(&id).ok_or(TreeError::NotFound(id))?.parent.expect("non-root");
        let old_index = self
            .children(old_parent)
            .iter()
            .position(|&c| c == id)
            .expect("child listed in parent");

        self.nodes.get_mut(&old_parent).expect("parent exists").children.remove(old_index);
        // Removing first shifts indices when re-inserting into the same parent.
        let p = self.nodes.get_mut(&new_parent).expect("checked above");
        let new_index = new_index.min(p.children.len());
        p.children.insert(new_index, id);
        self.nodes.get_mut(&id).expect("node exists").parent = Some(new_parent);
        Ok((old_parent, old_index))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::{LayerProps, NodeKind, PlaceholderArt, RasterContent};

    fn leaf(name: &str) -> Node {
        Node::new(
            LayerProps::named(name),
            NodeKind::Raster(RasterContent::from_placeholder(PlaceholderArt { bounds: [0.0; 4], color: [1.0; 4] })),
        )
    }

    fn doc_with(names: &[&str]) -> (Document, Vec<NodeId>) {
        let mut doc = Document::new([100, 100], ProjectFocus::Raster);
        let mut ids = Vec::new();
        for (i, n) in names.iter().enumerate() {
            let id = doc.alloc_id();
            doc.insert_node(id, leaf(n), doc.root(), i).unwrap();
            ids.push(id);
        }
        (doc, ids)
    }

    #[test]
    fn insert_orders_children() {
        let (doc, ids) = doc_with(&["a", "b", "c"]);
        assert_eq!(doc.children(doc.root()), ids.as_slice());
    }

    /// Spec 0058: a snapshot from one document imports into another with fresh
    /// ids (source ids collide with the target's), structure and pixels intact.
    #[test]
    fn import_subtree_remaps_ids_and_content() {
        let (src, src_ids) = doc_with(&["group-content"]);
        // Give the source layer a group wrapper + a pixel so content is checkable.
        let (mut src, _) = (src, src_ids);
        let g = src.alloc_id();
        src.insert_node(g, Node::group("g"), src.root(), 0).unwrap();
        let inner = src.alloc_id();
        let mut inner_node = leaf("inner");
        if let NodeKind::Raster(c) = &mut inner_node.kind {
            c.tiles.fill_rect(0, 0, 4, 4, [7, 8, 9, 255]);
        }
        src.insert_node(inner, inner_node, g, 0).unwrap();
        let snap = src.snapshot_subtree(g).unwrap();

        // Target has its own nodes occupying the same id range.
        let (mut dst, _) = doc_with(&["x", "y", "z"]);
        let n0 = dst.node_count();
        let (new_root, nodes) = dst.import_subtree(&snap, dst.root()).unwrap();
        assert!(dst.node(new_root).is_none(), "import allocates, does not insert");
        for (id, _) in &nodes {
            assert!(dst.node(*id).is_none(), "all imported ids are fresh in dst");
        }
        dst.restore_subtree(nodes, dst.root(), 0).unwrap();
        assert_eq!(dst.node_count(), n0 + 2, "group + inner imported");
        let kids = dst.children(new_root).to_vec();
        assert_eq!(kids.len(), 1, "group structure preserved");
        match &dst.node(kids[0]).unwrap().kind {
            NodeKind::Raster(c) => assert_eq!(c.tiles.pixel(2, 2), [7, 8, 9, 255]),
            _ => panic!("raster expected"),
        }
    }

    #[test]
    fn remove_and_restore_round_trips() {
        let (mut doc, ids) = doc_with(&["a", "b", "c"]);
        let before = doc.clone();
        let (removed, parent, index) = doc.remove_subtree(ids[1]).unwrap();
        assert_eq!(doc.children(doc.root()).len(), 2);
        doc.restore_subtree(removed, parent, index).unwrap();
        assert_eq!(doc, before);
    }

    #[test]
    fn remove_group_takes_subtree() {
        let (mut doc, ids) = doc_with(&["a"]);
        let g = doc.alloc_id();
        doc.insert_node(g, Node::group("g"), doc.root(), 1).unwrap();
        doc.move_node(ids[0], g, 0).unwrap();
        let (removed, ..) = doc.remove_subtree(g).unwrap();
        assert_eq!(removed.len(), 2);
        assert!(doc.node(ids[0]).is_none());
    }

    #[test]
    fn move_rejects_cycles_and_non_groups() {
        let (mut doc, ids) = doc_with(&["a"]);
        let g = doc.alloc_id();
        doc.insert_node(g, Node::group("g"), doc.root(), 1).unwrap();
        doc.move_node(g, g, 0).unwrap_err();
        assert_eq!(doc.move_node(ids[0], ids[0], 0), Err(TreeError::Cycle(ids[0])));
        let inner = doc.alloc_id();
        doc.insert_node(inner, Node::group("inner"), g, 0).unwrap();
        assert_eq!(doc.move_node(g, inner, 0), Err(TreeError::Cycle(g)));
        assert_eq!(doc.move_node(g, ids[0], 0), Err(TreeError::NotAGroup(ids[0])));
    }

    #[test]
    fn move_within_same_parent_keeps_all_children() {
        let (mut doc, ids) = doc_with(&["a", "b", "c"]);
        let (old_parent, old_index) = doc.move_node(ids[0], doc.root(), 2).unwrap();
        assert_eq!((old_parent, old_index), (doc.root(), 0));
        assert_eq!(doc.children(doc.root()), &[ids[1], ids[2], ids[0]]);
    }

    #[test]
    fn iter_tree_is_depth_first_panel_order() {
        let (mut doc, ids) = doc_with(&["a", "b"]);
        let g = doc.alloc_id();
        doc.insert_node(g, Node::group("g"), doc.root(), 1).unwrap();
        doc.move_node(ids[1], g, 0).unwrap();
        let flat: Vec<_> = doc.iter_tree();
        assert_eq!(flat, vec![(ids[0], 0), (g, 0), (ids[1], 1)]);
    }

    #[test]
    fn ids_never_reused_after_removal() {
        let (mut doc, ids) = doc_with(&["a"]);
        doc.remove_subtree(ids[0]).unwrap();
        let fresh = doc.alloc_id();
        assert!(fresh > ids[0]);
    }
}
