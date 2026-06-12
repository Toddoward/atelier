//! Flat composite-op list built from the layer tree — the structural walk the
//! GPU compositor executes (spec 0004). Mirrors `compositor::composite_children`
//! semantics exactly; golden tests pin the two together.

use atelier_core::{BlendMode, Document, NodeId, NodeKind, TileMap};

#[derive(Debug)]
pub enum CompositeOp<'a> {
    /// Composite a raster layer's tiles (drawn at `offset`) onto the stack top.
    Layer { tiles: &'a TileMap, offset: [i32; 2], mode: BlendMode, opacity: f32 },
    /// Open an isolated transparent buffer (non-pass-through group).
    Push,
    /// Blend the isolated buffer onto the previous top and discard it.
    Pop { mode: BlendMode, opacity: f32 },
}

pub fn build_op_list(doc: &Document) -> Vec<CompositeOp<'_>> {
    let mut ops = Vec::new();
    walk(doc, doc.root(), &mut ops);
    ops
}

fn walk<'a>(doc: &'a Document, parent: NodeId, ops: &mut Vec<CompositeOp<'a>>) {
    // Children are top-first; compositing goes bottom-first.
    for &id in doc.children(parent).iter().rev() {
        let Some(node) = doc.node(id) else { continue };
        if !node.props.visible {
            continue;
        }
        let props = &node.props;
        match &node.kind {
            NodeKind::Raster(content) => {
                ops.push(CompositeOp::Layer {
                    tiles: &content.tiles,
                    offset: content.offset,
                    mode: props.blend,
                    opacity: props.opacity,
                });
            }
            NodeKind::Group { .. } => {
                if props.blend == BlendMode::PassThrough && props.opacity >= 1.0 {
                    walk(doc, id, ops);
                } else {
                    ops.push(CompositeOp::Push);
                    walk(doc, id, ops);
                    ops.push(CompositeOp::Pop { mode: props.blend, opacity: props.opacity });
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use atelier_core::command::AddNode;
    use atelier_core::{Command, LayerProps, Node, PlaceholderArt, ProjectFocus, RasterContent};

    fn leaf(name: &str) -> Node {
        Node::new(
            LayerProps::named(name),
            NodeKind::Raster(RasterContent::from_placeholder(PlaceholderArt {
                bounds: [0.0, 0.0, 2.0, 2.0],
                color: [1.0; 4],
            })),
        )
    }

    fn add(doc: &mut Document, node: Node, parent: NodeId, index: usize) -> NodeId {
        let mut cmd = AddNode::new(doc, node, parent, index);
        cmd.apply(doc);
        cmd.id
    }

    #[test]
    fn op_list_reflects_tree_structure_and_isolation() {
        let mut doc = Document::new([4, 4], ProjectFocus::Raster);
        let root = doc.root();
        add(&mut doc, leaf("bottom"), root, 0);
        let g = add(&mut doc, Node::group("g"), root, 0);
        doc.node_mut(g).unwrap().props.blend = BlendMode::Multiply; // isolated
        add(&mut doc, leaf("inner"), g, 0);
        let hidden = add(&mut doc, leaf("hidden"), root, 0);
        doc.node_mut(hidden).unwrap().props.visible = false;

        let ops = build_op_list(&doc);
        let shape: Vec<&str> = ops
            .iter()
            .map(|op| match op {
                CompositeOp::Layer { .. } => "layer",
                CompositeOp::Push => "push",
                CompositeOp::Pop { .. } => "pop",
            })
            .collect();
        // bottom-first: leaf, then isolated group, hidden skipped.
        assert_eq!(shape, vec!["layer", "push", "layer", "pop"]);
    }

    #[test]
    fn pass_through_group_emits_children_inline() {
        let mut doc = Document::new([4, 4], ProjectFocus::Raster);
        let root = doc.root();
        let g = add(&mut doc, Node::group("pt"), root, 0); // PassThrough default
        add(&mut doc, leaf("a"), g, 0);
        let ops = build_op_list(&doc);
        assert_eq!(ops.len(), 1, "no push/pop for pass-through at full opacity");
        assert!(matches!(ops[0], CompositeOp::Layer { .. }));
    }
}
