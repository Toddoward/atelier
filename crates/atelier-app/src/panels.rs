//! Layers, History, and Properties panels (spec 0002).

use crate::EditorState;
use atelier_core::command::{AddNode, MoveNode, RemoveNode, SetBlend, SetName, SetOpacity, SetVisible};
use atelier_core::{BlendMode, LayerProps, Node, NodeId, NodeKind, PlaceholderArt, RasterContent};

const PALETTE: [[f32; 4]; 6] = [
    [0.86, 0.39, 0.35, 0.9],
    [0.40, 0.69, 0.42, 0.9],
    [0.36, 0.55, 0.84, 0.9],
    [0.87, 0.70, 0.34, 0.9],
    [0.65, 0.46, 0.78, 0.9],
    [0.38, 0.72, 0.70, 0.9],
];

pub fn tools_ui(ui: &mut egui::Ui, state: &mut EditorState) {
    use crate::ActiveTool;
    for (tool, label) in [
        (ActiveTool::Move, "Move (V)"),
        (ActiveTool::Brush, "Brush (B)"),
        (ActiveTool::Eraser, "Eraser (E)"),
        (ActiveTool::SelectRect, "Select Rect (M)"),
        (ActiveTool::SelectEllipse, "Select Ellipse"),
        (ActiveTool::Lasso, "Lasso (L)"),
        (ActiveTool::MagicWand, "Magic Wand (W)"),
        (ActiveTool::ShapeRect, "Rectangle (U)"),
        (ActiveTool::ShapeEllipse, "Ellipse"),
        (ActiveTool::ShapePolygon, "Polygon"),
        (ActiveTool::ShapeStar, "Star"),
        (ActiveTool::Eyedropper, "Eyedropper (I)"),
        (ActiveTool::Gradient, "Gradient (G)"),
        (ActiveTool::Bucket, "Paint Bucket (K)"),
        (ActiveTool::Pen, "Pen (P)"),
        (ActiveTool::DirectSelect, "Direct Select (A)"),
    ] {
        if ui.selectable_label(state.tool == tool, label).clicked() {
            state.tool = tool;
        }
    }
    if state.tool == ActiveTool::MagicWand {
        ui.separator();
        ui.label("Tolerance");
        ui.add(egui::Slider::new(&mut state.brush.wand_tolerance, 0..=128));
    }
    if state.tool == ActiveTool::Gradient {
        ui.separator();
        ui.checkbox(&mut state.brush.gradient_radial, "Radial");
    }
    if state.tool.shape_kind().is_some() {
        ui.separator();
        ui.label("Fill");
        let mut rgba = egui::Rgba::from_rgba_unmultiplied(
            state.brush.vector_fill[0],
            state.brush.vector_fill[1],
            state.brush.vector_fill[2],
            state.brush.vector_fill[3],
        );
        if egui::color_picker::color_edit_button_rgba(
            ui,
            &mut rgba,
            egui::color_picker::Alpha::OnlyBlend,
        )
        .changed()
        {
            let [r, g, b, a] = rgba.to_rgba_unmultiplied();
            state.brush.vector_fill = [r, g, b, a];
        }
    }
    if matches!(state.tool, ActiveTool::Brush | ActiveTool::Eraser) {
        ui.separator();
        ui.label("Size");
        ui.add(egui::Slider::new(&mut state.brush.radius, 1.0..=256.0).logarithmic(true));
        ui.label("Hardness");
        ui.add(egui::Slider::new(&mut state.brush.hardness, 0.0..=1.0));
        if state.tool == ActiveTool::Brush {
            ui.label("Color");
            let mut rgba = egui::Rgba::from_rgba_unmultiplied(
                state.brush.color[0],
                state.brush.color[1],
                state.brush.color[2],
                state.brush.color[3],
            );
            if egui::color_picker::color_edit_button_rgba(
                ui,
                &mut rgba,
                egui::color_picker::Alpha::OnlyBlend,
            )
            .changed()
            {
                let [r, g, b, a] = rgba.to_rgba_unmultiplied();
                state.brush.color = [r, g, b, a];
            }
        }
    }
}

pub fn layers_ui(ui: &mut egui::Ui, state: &mut EditorState) {
    toolbar(ui, state);
    ui.separator();
    selected_layer_controls(ui, state);
    // Cross-layer align/distribute appears with a multi-selection (spec 0029).
    if selected_set(state).len() >= 2 {
        ui.label("Align layers");
        ui.horizontal(|ui| {
            for (label, a) in [
                ("L", Align::Left),
                ("C", Align::HCenter),
                ("R", Align::Right),
                ("T", Align::Top),
                ("M", Align::VMiddle),
                ("B", Align::Bottom),
            ] {
                if ui.small_button(label).clicked() {
                    align_layers(state, a);
                }
            }
        });
        if selected_set(state).len() >= 3 {
            ui.horizontal(|ui| {
                if ui.button("Distribute H").clicked() {
                    distribute_layers(state, true);
                }
                if ui.button("Distribute V").clicked() {
                    distribute_layers(state, false);
                }
            });
        }
    }
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
    let selected = state.editor.selection == Some(id) || state.selected_extra.contains(&id);

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
                // Shift/Ctrl-click extends the selection (spec 0028 multi-select).
                let additive = ui.input(|i| i.modifiers.shift || i.modifiers.command);
                if additive && state.editor.selection.is_some() && state.editor.selection != Some(id)
                {
                    if let Some(pos) = state.selected_extra.iter().position(|&e| e == id) {
                        state.selected_extra.remove(pos); // toggle off
                    } else {
                        state.selected_extra.push(id);
                    }
                } else {
                    state.editor.selection = Some(id);
                    state.selected_extra.clear();
                }
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
        NodeKind::Raster(RasterContent::from_placeholder(PlaceholderArt {
            bounds: [w * 0.1 + offset, h * 0.1 + offset, w * 0.5, h * 0.5],
            color: PALETTE[n % PALETTE.len()],
        })),
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

pub fn properties_ui(ui: &mut egui::Ui, state: &mut EditorState) {
    let doc = &state.editor.doc;
    ui.label(format!("Size: {} × {} px", doc.size[0], doc.size[1]));
    ui.label(format!("Focus: {:?}", doc.focus));
    ui.label(format!("Color: {}", doc.color_mode));
    ui.separator();

    let Some(id) = state.editor.selection else {
        ui.weak("Nothing selected");
        return;
    };
    let Some(node) = doc.node(id) else {
        ui.weak("Nothing selected");
        return;
    };
    ui.label(format!("Selected: {} ({})", node.props.name, node.kind.kind_name()));
    ui.label(format!(
        "Blend: {} · Opacity: {:.0}%",
        node.props.blend.name(),
        node.props.opacity * 100.0
    ));

    // Adjustment layers expose their parameters here (undoable, live).
    if let NodeKind::Adjustment(adj) = node.kind {
        ui.separator();
        adjustment_editor(ui, state, id, adj);
    }

    // Vector layers: edit the fill color of all shapes (spec 0017 follow-up).
    // Recompute in a scoped borrow so the node borrow doesn't span the edit.
    let vec_fill = match &state.editor.doc.node(id).expect("selected").kind {
        NodeKind::Vector(c) => Some(c.shapes.iter().find_map(|s| s.fill).unwrap_or([0.0; 4])),
        _ => None,
    };
    if let Some(current) = vec_fill {
        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Fill");
            let mut rgba = egui::Rgba::from_rgba_unmultiplied(
                current[0], current[1], current[2], current[3],
            );
            if egui::color_picker::color_edit_button_rgba(
                ui,
                &mut rgba,
                egui::color_picker::Alpha::OnlyBlend,
            )
            .changed()
            {
                let [r, g, b, a] = rgba.to_rgba_unmultiplied();
                state.editor.history.set_merging(true);
                apply_vector_fill(state, id, [r, g, b, a]);
                state.editor.history.set_merging(false);
            }
        });
        ui.label("Align to canvas");
        ui.horizontal(|ui| {
            for (label, a) in [
                ("L", Align::Left),
                ("C", Align::HCenter),
                ("R", Align::Right),
                ("T", Align::Top),
                ("M", Align::VMiddle),
                ("B", Align::Bottom),
            ] {
                if ui.small_button(label).clicked() {
                    align_vector_to_canvas(state, id, a);
                }
            }
        });
        ui.horizontal(|ui| {
            if ui.button("Make Compound").clicked() {
                make_compound_path(state, id);
            }
            if ui.button("Release").clicked() {
                release_compound_path(state, id);
            }
        });
        ui.label("Pathfinder");
        ui.horizontal(|ui| {
            use atelier_core::atelier_vector::BoolOp;
            for (label, op) in [
                ("Unite", BoolOp::Union),
                ("Intersect", BoolOp::Intersect),
                ("Minus", BoolOp::Difference),
                ("Exclude", BoolOp::Exclude),
            ] {
                if ui.small_button(label).clicked() {
                    pathfinder(state, id, op);
                }
            }
        });
        ui.label("Align shapes");
        ui.horizontal(|ui| {
            for (label, a) in [
                ("L", Align::Left),
                ("C", Align::HCenter),
                ("R", Align::Right),
                ("T", Align::Top),
                ("M", Align::VMiddle),
                ("B", Align::Bottom),
            ] {
                if ui.small_button(label).clicked() {
                    align_shapes_in_layer(state, id, a);
                }
            }
        });
        ui.horizontal(|ui| {
            if ui.button("Distribute H").clicked() {
                distribute_shapes_in_layer(state, id, true);
            }
            if ui.button("Distribute V").clicked() {
                distribute_shapes_in_layer(state, id, false);
            }
        });
    }
}

/// Canvas-relative alignment for a vector layer (spec 0022).
#[derive(Clone, Copy)]
pub enum Align {
    Left,
    HCenter,
    Right,
    Top,
    VMiddle,
    Bottom,
}

/// Align a vector layer's shapes (as a group) to the document bounds. Undoable.
pub fn align_vector_to_canvas(state: &mut EditorState, id: NodeId, a: Align) {
    let [w, h] = state.editor.doc.size;
    let shapes = match state.editor.doc.node(id).map(|n| &n.kind) {
        Some(NodeKind::Vector(c)) => c.shapes.clone(),
        _ => return,
    };
    // Union bounds across all shapes.
    let mut bb: Option<[f32; 4]> = None;
    for s in &shapes {
        if let Some(b) = s.path.bounds() {
            bb = Some(match bb {
                None => b,
                Some(o) => [o[0].min(b[0]), o[1].min(b[1]), o[2].max(b[2]), o[3].max(b[3])],
            });
        }
    }
    let Some([x0, y0, x1, y1]) = bb else { return };
    let (w, h) = (w as f32, h as f32);
    let (dx, dy) = match a {
        Align::Left => (-x0, 0.0),
        Align::Right => (w - x1, 0.0),
        Align::HCenter => ((w - (x0 + x1)) * 0.5, 0.0),
        Align::Top => (0.0, -y0),
        Align::Bottom => (0.0, h - y1),
        Align::VMiddle => (0.0, (h - (y0 + y1)) * 0.5),
    };
    if dx == 0.0 && dy == 0.0 {
        return;
    }
    let mut new_shapes = shapes;
    for s in &mut new_shapes {
        s.path.translate(dx, dy);
    }
    let cmd = atelier_core::command::SetVectorShapes::new(&state.editor.doc, id, new_shapes);
    state.editor.apply(Box::new(cmd));
}

/// Per-shape bounds for every shape in a vector layer (None if any is empty).
fn shape_bounds(shapes: &[atelier_core::atelier_vector::Shape]) -> Option<Vec<[f32; 4]>> {
    shapes.iter().map(|s| s.path.bounds()).collect()
}

/// Align a vector layer's shapes to each other (relative to their union bounds).
/// Undoable; no-op with < 2 shapes. Spec 0026 (VEC-6).
pub fn align_shapes_in_layer(state: &mut EditorState, id: NodeId, a: Align) {
    let shapes = match state.editor.doc.node(id).map(|n| &n.kind) {
        Some(NodeKind::Vector(c)) => c.shapes.clone(),
        _ => return,
    };
    if shapes.len() < 2 {
        return;
    }
    let Some(bounds) = shape_bounds(&shapes) else { return };
    let u = bounds.iter().fold(bounds[0], |o, b| {
        [o[0].min(b[0]), o[1].min(b[1]), o[2].max(b[2]), o[3].max(b[3])]
    });
    let mut new = shapes.clone();
    for (s, b) in new.iter_mut().zip(&bounds) {
        let (dx, dy) = match a {
            Align::Left => (u[0] - b[0], 0.0),
            Align::Right => (u[2] - b[2], 0.0),
            Align::HCenter => ((u[0] + u[2]) * 0.5 - (b[0] + b[2]) * 0.5, 0.0),
            Align::Top => (0.0, u[1] - b[1]),
            Align::Bottom => (0.0, u[3] - b[3]),
            Align::VMiddle => (0.0, (u[1] + u[3]) * 0.5 - (b[1] + b[3]) * 0.5),
        };
        s.path.translate(dx, dy);
    }
    let cmd = atelier_core::command::SetVectorShapes::new(&state.editor.doc, id, new);
    state.editor.apply(Box::new(cmd));
}

/// Evenly distribute a vector layer's shapes by center along an axis.
/// Undoable; no-op with < 3 shapes. Spec 0026 (VEC-6).
pub fn distribute_shapes_in_layer(state: &mut EditorState, id: NodeId, horizontal: bool) {
    let shapes = match state.editor.doc.node(id).map(|n| &n.kind) {
        Some(NodeKind::Vector(c)) => c.shapes.clone(),
        _ => return,
    };
    let n = shapes.len();
    if n < 3 {
        return;
    }
    let Some(bounds) = shape_bounds(&shapes) else { return };
    let center = |b: &[f32; 4]| if horizontal { (b[0] + b[2]) * 0.5 } else { (b[1] + b[3]) * 0.5 };
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&i, &j| center(&bounds[i]).partial_cmp(&center(&bounds[j])).expect("finite"));
    let first = center(&bounds[order[0]]);
    let last = center(&bounds[order[n - 1]]);
    let step = (last - first) / (n as f32 - 1.0);
    let mut new = shapes.clone();
    for (rank, &idx) in order.iter().enumerate() {
        let target = first + step * rank as f32;
        let d = target - center(&bounds[idx]);
        if horizontal {
            new[idx].path.translate(d, 0.0);
        } else {
            new[idx].path.translate(0.0, d);
        }
    }
    let cmd = atelier_core::command::SetVectorShapes::new(&state.editor.doc, id, new);
    state.editor.apply(Box::new(cmd));
}

/// Currently selected nodes (primary + valid extras, deduped).
fn selected_set(state: &EditorState) -> Vec<NodeId> {
    let mut out = Vec::new();
    if let Some(p) = state.editor.selection {
        out.push(p);
    }
    for &e in &state.selected_extra {
        if Some(e) != state.editor.selection && state.editor.doc.node(e).is_some() {
            out.push(e);
        }
    }
    out
}

/// Doc-space bounds of a raster (tiles+offset) or vector (shape union) layer.
fn layer_doc_bounds(node: &atelier_core::Node) -> Option<[f32; 4]> {
    match &node.kind {
        NodeKind::Raster(c) => {
            let [x0, y0, x1, y1] = c.tiles.content_bounds()?;
            let [ox, oy] = c.offset;
            Some([(x0 + ox) as f32, (y0 + oy) as f32, (x1 + ox) as f32, (y1 + oy) as f32])
        }
        NodeKind::Vector(c) => {
            let mut bb: Option<[f32; 4]> = None;
            for s in &c.shapes {
                if let Some(b) = s.path.bounds() {
                    bb = Some(match bb {
                        None => b,
                        Some(o) => [o[0].min(b[0]), o[1].min(b[1]), o[2].max(b[2]), o[3].max(b[3])],
                    });
                }
            }
            bb
        }
        _ => None,
    }
}

/// Command translating a whole layer by `(dx, dy)` doc px (raster=offset,
/// vector=shape translate). None for other kinds.
fn translate_layer_cmd(
    doc: &atelier_core::Document,
    id: NodeId,
    dx: f32,
    dy: f32,
) -> Option<Box<dyn atelier_core::Command>> {
    match &doc.node(id)?.kind {
        NodeKind::Raster(c) => {
            let new = [c.offset[0] + dx.round() as i32, c.offset[1] + dy.round() as i32];
            Some(Box::new(atelier_core::command::SetOffset::new(doc, id, new)))
        }
        NodeKind::Vector(c) => {
            let mut shapes = c.shapes.clone();
            for s in &mut shapes {
                s.path.translate(dx, dy);
            }
            Some(Box::new(atelier_core::command::SetVectorShapes::new(doc, id, shapes)))
        }
        _ => None,
    }
}

/// Align selected raster/vector layers to each other (union bounds). Spec 0029.
pub fn align_layers(state: &mut EditorState, a: Align) {
    let ids = selected_set(state);
    let items: Vec<(NodeId, [f32; 4])> = ids
        .iter()
        .filter_map(|&id| state.editor.doc.node(id).and_then(layer_doc_bounds).map(|b| (id, b)))
        .collect();
    if items.len() < 2 {
        return;
    }
    let u = items.iter().map(|(_, b)| *b).fold(items[0].1, |o, b| {
        [o[0].min(b[0]), o[1].min(b[1]), o[2].max(b[2]), o[3].max(b[3])]
    });
    let mut cmds: Vec<Box<dyn atelier_core::Command>> = Vec::new();
    for (id, b) in &items {
        let (dx, dy) = match a {
            Align::Left => (u[0] - b[0], 0.0),
            Align::Right => (u[2] - b[2], 0.0),
            Align::HCenter => ((u[0] + u[2]) * 0.5 - (b[0] + b[2]) * 0.5, 0.0),
            Align::Top => (0.0, u[1] - b[1]),
            Align::Bottom => (0.0, u[3] - b[3]),
            Align::VMiddle => (0.0, (u[1] + u[3]) * 0.5 - (b[1] + b[3]) * 0.5),
        };
        if dx != 0.0 || dy != 0.0 {
            if let Some(cmd) = translate_layer_cmd(&state.editor.doc, *id, dx, dy) {
                cmds.push(cmd);
            }
        }
    }
    if !cmds.is_empty() {
        state.editor.apply(Box::new(atelier_core::command::Batch::new(cmds, "Align Layers")));
    }
}

/// Evenly distribute selected layers by center along an axis. Spec 0029.
pub fn distribute_layers(state: &mut EditorState, horizontal: bool) {
    let ids = selected_set(state);
    let items: Vec<(NodeId, [f32; 4])> = ids
        .iter()
        .filter_map(|&id| state.editor.doc.node(id).and_then(layer_doc_bounds).map(|b| (id, b)))
        .collect();
    let n = items.len();
    if n < 3 {
        return;
    }
    let center = |b: &[f32; 4]| if horizontal { (b[0] + b[2]) * 0.5 } else { (b[1] + b[3]) * 0.5 };
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&i, &j| center(&items[i].1).partial_cmp(&center(&items[j].1)).expect("finite"));
    let first = center(&items[order[0]].1);
    let last = center(&items[order[n - 1]].1);
    let step = (last - first) / (n as f32 - 1.0);
    let mut cmds: Vec<Box<dyn atelier_core::Command>> = Vec::new();
    for (rank, &i) in order.iter().enumerate() {
        let target = first + step * rank as f32;
        let d = target - center(&items[i].1);
        let (dx, dy) = if horizontal { (d, 0.0) } else { (0.0, d) };
        if dx != 0.0 || dy != 0.0 {
            if let Some(cmd) = translate_layer_cmd(&state.editor.doc, items[i].0, dx, dy) {
                cmds.push(cmd);
            }
        }
    }
    if !cmds.is_empty() {
        state.editor.apply(Box::new(atelier_core::command::Batch::new(cmds, "Distribute Layers")));
    }
}

/// Apply a boolean Pathfinder op across a vector layer's shapes (folded
/// left), replacing them with the single resulting shape. Undoable; no-op with
/// < 2 shapes. Spec 0031 (VEC-5).
pub fn pathfinder(state: &mut EditorState, id: NodeId, op: atelier_core::atelier_vector::BoolOp) {
    use atelier_core::atelier_vector::{boolean, Shape};
    let shapes = match state.editor.doc.node(id).map(|n| &n.kind) {
        Some(NodeKind::Vector(c)) => c.shapes.clone(),
        _ => return,
    };
    if shapes.len() < 2 {
        return;
    }
    let mut acc = shapes[0].path.clone();
    for s in &shapes[1..] {
        acc = boolean(&acc, &s.path, op);
    }
    let result = if acc.subpaths.is_empty() {
        Vec::new()
    } else {
        vec![Shape { path: acc, fill: shapes[0].fill, stroke: shapes[0].stroke }]
    };
    let cmd = atelier_core::command::SetVectorShapes::new(&state.editor.doc, id, result);
    state.editor.apply(Box::new(cmd));
}

/// Merge a vector layer's shapes into one compound path (even-odd fill so
/// overlaps cut holes). Undoable; no-op with <2 shapes. Spec 0024.
pub fn make_compound_path(state: &mut EditorState, id: NodeId) {
    use atelier_core::atelier_vector::{FillRule, Shape};
    let shapes = match state.editor.doc.node(id).map(|n| &n.kind) {
        Some(NodeKind::Vector(c)) => c.shapes.clone(),
        _ => return,
    };
    if shapes.len() < 2 {
        return;
    }
    let mut path = shapes[0].path.clone();
    for s in &shapes[1..] {
        path.append(&s.path);
    }
    path.fill_rule = FillRule::EvenOdd;
    let merged = Shape { path, fill: shapes[0].fill, stroke: shapes[0].stroke };
    let cmd = atelier_core::command::SetVectorShapes::new(&state.editor.doc, id, vec![merged]);
    state.editor.apply(Box::new(cmd));
}

/// Release a compound path: split each shape's subpaths into separate shapes.
/// Undoable; no-op when nothing has multiple subpaths. Spec 0024.
pub fn release_compound_path(state: &mut EditorState, id: NodeId) {
    use atelier_core::atelier_vector::Shape;
    let shapes = match state.editor.doc.node(id).map(|n| &n.kind) {
        Some(NodeKind::Vector(c)) => c.shapes.clone(),
        _ => return,
    };
    let mut out = Vec::new();
    let mut changed = false;
    for s in &shapes {
        let parts = s.path.split_subpaths();
        if parts.len() > 1 {
            changed = true;
        }
        for p in parts {
            out.push(Shape { path: p, fill: s.fill, stroke: s.stroke });
        }
    }
    if !changed {
        return;
    }
    let cmd = atelier_core::command::SetVectorShapes::new(&state.editor.doc, id, out);
    state.editor.apply(Box::new(cmd));
}

/// Set the fill color of every shape in a vector layer (undoable, merged).
pub fn apply_vector_fill(state: &mut EditorState, id: NodeId, color: [f32; 4]) {
    let shapes = match state.editor.doc.node(id).map(|n| &n.kind) {
        Some(NodeKind::Vector(c)) => {
            let mut s = c.shapes.clone();
            for sh in &mut s {
                sh.fill = Some(color);
            }
            s
        }
        _ => return,
    };
    let cmd = atelier_core::command::SetVectorShapes::new(&state.editor.doc, id, shapes);
    state.editor.apply(Box::new(cmd));
}

fn adjustment_editor(
    ui: &mut egui::Ui,
    state: &mut EditorState,
    id: NodeId,
    current: atelier_core::Adjustment,
) {
    use atelier_core::Adjustment;
    let mut edited = current;
    let mut changed = false;
    match &mut edited {
        Adjustment::Invert => {
            ui.weak("Invert has no parameters");
        }
        Adjustment::BrightnessContrast { brightness, contrast } => {
            changed |= ui.add(egui::Slider::new(brightness, -1.0..=1.0).text("Brightness")).changed();
            changed |= ui.add(egui::Slider::new(contrast, -1.0..=1.0).text("Contrast")).changed();
        }
        Adjustment::Levels { black, white, gamma } => {
            changed |= ui.add(egui::Slider::new(black, 0.0..=1.0).text("Black")).changed();
            changed |= ui.add(egui::Slider::new(white, 0.0..=1.0).text("White")).changed();
            changed |= ui.add(egui::Slider::new(gamma, 0.1..=5.0).text("Gamma")).changed();
        }
        Adjustment::HueSaturation { hue, sat, light } => {
            changed |= ui.add(egui::Slider::new(hue, -180.0..=180.0).text("Hue")).changed();
            changed |= ui.add(egui::Slider::new(sat, -1.0..=1.0).text("Saturation")).changed();
            changed |= ui.add(egui::Slider::new(light, -1.0..=1.0).text("Lightness")).changed();
        }
    }
    if changed {
        // Coalesce slider drags into one undo entry.
        state.editor.history.set_merging(true);
        let cmd = atelier_core::command::SetAdjustment::new(&state.editor.doc, id, edited);
        state.editor.apply(Box::new(cmd));
        state.editor.history.set_merging(false);
    }
}
