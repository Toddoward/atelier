//! Layers, History, and Properties panels (spec 0002).

use crate::EditorState;
use atelier_core::command::{AddNode, MoveNode, RemoveNode, SetBlend, SetName, SetOpacity, SetVisible};
use atelier_core::{BlendMode, LayerProps, Node, NodeId, NodeKind, PlaceholderArt};

const PALETTE: [[f32; 4]; 6] = [
    [0.86, 0.39, 0.35, 0.9],
    [0.40, 0.69, 0.42, 0.9],
    [0.36, 0.55, 0.84, 0.9],
    [0.87, 0.70, 0.34, 0.9],
    [0.65, 0.46, 0.78, 0.9],
    [0.38, 0.72, 0.70, 0.9],
];

pub fn layers_ui(ui: &mut egui::Ui, state: &mut EditorState) {
    toolbar(ui, state);
    ui.separator();
    selected_layer_controls(ui, state);
    ui.separator();

    // Rows in panel order, skipping children of collapsed groups.
    let rows = visible_rows(state);
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        for (id, depth) in rows {
            layer_row(ui, state, id, depth);
        }
    });
}

fn toolbar(ui: &mut egui::Ui, state: &mut EditorState) {
    ui.horizontal_wrapped(|ui| {
        if ui.button("+ Layer").on_hover_text("Add raster layer").clicked() {
            let node = new_layer(state);
            add_node(state, node);
        }
        if ui.button("+ Group").clicked() {
            let node = Node::group(format!("Group {}", state.layer_counter()));
            add_node(state, node);
        }
        let sel = state.editor.selection;
        if ui.add_enabled(sel.is_some(), egui::Button::new("Delete")).clicked() {
            let id = sel.expect("button enabled only with selection");
            let cmd = RemoveNode::new(&state.editor.doc, id);
            state.editor.apply(Box::new(cmd));
            state.editor.selection = None;
        }
        move_buttons(ui, state);
    });
}

fn move_buttons(ui: &mut egui::Ui, state: &mut EditorState) {
    let Some(id) = state.editor.selection else {
        ui.add_enabled_ui(false, |ui| {
            let _ = ui.button("Up");
            let _ = ui.button("Down");
            let _ = ui.button("Into Group");
            let _ = ui.button("Out");
        });
        return;
    };
    // Read everything up front so no doc borrow is alive across `apply`.
    let (parent, index, sibling_count, group_above, out_target) = {
        let doc = &state.editor.doc;
        let parent = doc.node(id).and_then(|n| n.parent).unwrap_or(doc.root());
        let siblings = doc.children(parent);
        let index = siblings.iter().position(|&c| c == id).unwrap_or(0);
        let group_above = (index > 0)
            .then(|| siblings[index - 1])
            .filter(|&g| doc.node(g).is_some_and(|n| n.kind.is_group()));
        let out_target = doc.node(parent).and_then(|n| n.parent).map(|gp| {
            let pos = doc.children(gp).iter().position(|&c| c == parent).unwrap_or(0);
            (gp, pos + 1)
        });
        (parent, index, siblings.len(), group_above, out_target)
    };

    if ui.add_enabled(index > 0, egui::Button::new("Up")).on_hover_text("Move up").clicked() {
        let cmd = MoveNode::new(&state.editor.doc, id, parent, index - 1);
        state.editor.apply(Box::new(cmd));
    }
    if ui
        .add_enabled(index + 1 < sibling_count, egui::Button::new("Down"))
        .on_hover_text("Move down")
        .clicked()
    {
        let cmd = MoveNode::new(&state.editor.doc, id, parent, index + 1);
        state.editor.apply(Box::new(cmd));
    }
    if ui
        .add_enabled(group_above.is_some(), egui::Button::new("Into Group"))
        .on_hover_text("Move into the group above")
        .clicked()
    {
        let g = group_above.expect("enabled only when present");
        let cmd = MoveNode::new(&state.editor.doc, id, g, 0);
        state.editor.apply(Box::new(cmd));
    }
    if ui
        .add_enabled(out_target.is_some(), egui::Button::new("Out"))
        .on_hover_text("Move out of the current group")
        .clicked()
    {
        let (gp, pos) = out_target.expect("enabled only when present");
        let cmd = MoveNode::new(&state.editor.doc, id, gp, pos);
        state.editor.apply(Box::new(cmd));
    }
}

fn selected_layer_controls(ui: &mut egui::Ui, state: &mut EditorState) {
    let Some(id) = state.editor.selection else {
        ui.weak("No layer selected");
        return;
    };
    let Some(node) = state.editor.doc.node(id) else { return };
    let mut blend = node.props.blend;
    let mut opacity = node.props.opacity;

    ui.horizontal(|ui| {
        ui.label("Blend");
        let before = blend;
        egui::ComboBox::from_id_salt("blend_mode")
            .selected_text(blend.name())
            .show_ui(ui, |ui| {
                for mode in BlendMode::ALL {
                    ui.selectable_value(&mut blend, mode, mode.name());
                }
            });
        if blend != before {
            let cmd = SetBlend::new(&state.editor.doc, id, blend);
            state.editor.apply(Box::new(cmd));
        }
    });
    ui.horizontal(|ui| {
        ui.label("Opacity");
        let resp = ui.add(egui::Slider::new(&mut opacity, 0.0..=1.0).show_value(true));
        if resp.drag_started() {
            state.editor.history.set_merging(true);
        }
        if resp.changed() {
            let cmd = SetOpacity::new(&state.editor.doc, id, opacity);
            state.editor.apply(Box::new(cmd));
        }
        if resp.drag_stopped() {
            state.editor.history.set_merging(false);
        }
    });
}

fn visible_rows(state: &EditorState) -> Vec<(NodeId, usize)> {
    fn walk(state: &EditorState, parent: NodeId, depth: usize, out: &mut Vec<(NodeId, usize)>) {
        for &id in state.editor.doc.children(parent) {
            out.push((id, depth));
            if let Some(node) = state.editor.doc.node(id) {
                if let NodeKind::Group { expanded: true } = node.kind {
                    walk(state, id, depth + 1, out);
                }
            }
        }
    }
    let mut out = Vec::new();
    walk(state, state.editor.doc.root(), 0, &mut out);
    out
}

fn layer_row(ui: &mut egui::Ui, state: &mut EditorState, id: NodeId, depth: usize) {
    let Some(node) = state.editor.doc.node(id) else { return };
    let is_group = node.kind.is_group();
    let expanded = matches!(node.kind, NodeKind::Group { expanded: true });
    let mut visible = node.props.visible;
    let name = node.props.name.clone();
    let kind = node.kind.kind_name();
    let selected = state.editor.selection == Some(id);

    ui.horizontal(|ui| {
        ui.add_space(depth as f32 * 14.0);
        if is_group {
            // Expand/collapse is view state, not an edit: the one sanctioned
            // direct mutation (recorded in spec 0002 notes).
            if ui.selectable_label(false, if expanded { "▼" } else { "▶" }).clicked() {
                if let Some(n) = state.editor.doc.node_mut(id) {
                    n.kind = NodeKind::Group { expanded: !expanded };
                }
            }
        } else {
            ui.add_space(18.0);
        }
        if ui.checkbox(&mut visible, "").on_hover_text("Visibility").changed() {
            let cmd = SetVisible::new(&state.editor.doc, id, visible);
            state.editor.apply(Box::new(cmd));
        }

        if state.rename.as_ref().is_some_and(|(rid, _)| *rid == id) {
            let (_, buf) = state.rename.as_mut().expect("checked above");
            let resp = ui.text_edit_singleline(buf);
            resp.request_focus();
            let commit = resp.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter));
            if commit {
                let (_, buf) = state.rename.take().expect("renaming");
                if !buf.is_empty() && buf != name {
                    let cmd = SetName::new(&state.editor.doc, id, buf);
                    state.editor.apply(Box::new(cmd));
                }
            }
        } else {
            let label = if is_group { format!("[G] {name}") } else { name.clone() };
            let resp = ui.selectable_label(selected, label).on_hover_text(kind);
            if resp.clicked() {
                state.editor.selection = Some(id);
            }
            if resp.double_clicked() {
                state.rename = Some((id, name));
            }
        }
    });
}

fn new_layer(state: &EditorState) -> Node {
    let n = state.layer_counter();
    let [w, h] = state.editor.doc.size;
    let (w, h) = (w as f32, h as f32);
    let offset = 16.0 * ((n % 8) as f32);
    Node::new(
        LayerProps::named(format!("Layer {n}")),
        NodeKind::Raster(PlaceholderArt {
            bounds: [w * 0.1 + offset, h * 0.1 + offset, w * 0.5, h * 0.5],
            color: PALETTE[n % PALETTE.len()],
        }),
    )
}

/// Insert above the selection (same parent), or at the top of the root.
fn add_node(state: &mut EditorState, node: Node) {
    let doc = &state.editor.doc;
    let (parent, index) = match state.editor.selection.and_then(|s| doc.node(s).map(|n| (s, n))) {
        Some((sel, node_ref)) => {
            let parent = node_ref.parent.unwrap_or(doc.root());
            let index = doc.children(parent).iter().position(|&c| c == sel).unwrap_or(0);
            (parent, index)
        }
        None => (doc.root(), 0),
    };
    let cmd = AddNode::new(&mut state.editor.doc, node, parent, index);
    let id = cmd.id;
    state.editor.apply(Box::new(cmd));
    state.editor.selection = Some(id);
}

pub fn history_ui(ui: &mut egui::Ui, state: &mut EditorState) {
    let applied = state.editor.history.applied_len();
    let undo_labels: Vec<String> = state.editor.history.undo_labels().collect();
    let redo_labels: Vec<String> = state.editor.history.redo_labels().collect();

    let mut jump: Option<usize> = None;
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        if ui.selectable_label(applied == 0, "(document opened)").clicked() {
            jump = Some(0);
        }
        for (i, label) in undo_labels.iter().enumerate() {
            if ui.selectable_label(i + 1 == applied, label).clicked() {
                jump = Some(i + 1);
            }
        }
        for (i, label) in redo_labels.iter().enumerate() {
            if ui.selectable_label(false, egui::RichText::new(label).weak()).clicked() {
                jump = Some(applied + i + 1);
            }
        }
    });
    if let Some(target) = jump {
        state.editor.history.jump_to(&mut state.editor.doc, target);
    }
}

pub fn properties_ui(ui: &mut egui::Ui, state: &EditorState) {
    let doc = &state.editor.doc;
    ui.label(format!("Size: {} × {} px", doc.size[0], doc.size[1]));
    ui.label(format!("Focus: {:?}", doc.focus));
    ui.label(format!("Color: {}", doc.color_mode));
    ui.separator();
    match state.editor.selection.and_then(|id| doc.node(id)) {
        Some(node) => {
            ui.label(format!("Selected: {} ({})", node.props.name, node.kind.kind_name()));
            ui.label(format!(
                "Blend: {} · Opacity: {:.0}%",
                node.props.blend.name(),
                node.props.opacity * 100.0
            ));
        }
        None => {
            ui.weak("Nothing selected");
        }
    }
}
