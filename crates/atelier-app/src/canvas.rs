//! Canvas tab: checkerboard background (GPU callback) + the composited
//! document as a nearest-filtered texture, recomposited when the history
//! revision changes (spec 0004).

use crate::{ActiveTool, EditorState, SelectDrag, StrokeState};
use atelier_core::command::{PaintTiles, SetLayerMask, SetOffset, SetSelection, SetVectorShapes};
use atelier_core::{CombineOp, NodeId, NodeKind};
use atelier_gpu::{CheckerParams, CheckerboardRenderer, Viewport};
use atelier_raster::{selection, BrushParams};
use eframe::egui_wgpu::{self, wgpu};
use std::sync::Arc;

pub fn canvas_ui(ui: &mut egui::Ui, viewport: &mut Viewport, state: Option<&mut EditorState>) {
    let (rect, response) =
        ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());

    let space_down = ui.input(|i| i.key_down(egui::Key::Space));
    if response.dragged_by(egui::PointerButton::Middle)
        || (space_down && response.dragged_by(egui::PointerButton::Primary))
    {
        let d = response.drag_delta();
        viewport.pan_by([d.x, d.y]);
    }

    if response.hovered() {
        let zoom = ui.input(|i| i.zoom_delta());
        if zoom != 1.0 {
            if let Some(pos) = response.hover_pos() {
                viewport.zoom_about([pos.x - rect.min.x, pos.y - rect.min.y], zoom);
            }
        }

        // Keyboard navigation (Photoshop-style): Ctrl+= / Ctrl+- zoom about the
        // canvas center, Ctrl+0 resets to 100%, arrow keys pan.
        let center = [rect.width() * 0.5, rect.height() * 0.5];
        let (zoom_in, zoom_out, zoom_reset, pan) = ui.input(|i| {
            let cmd = i.modifiers.command;
            let mut pan = egui::Vec2::ZERO;
            if i.key_pressed(egui::Key::ArrowLeft) {
                pan.x += 64.0;
            }
            if i.key_pressed(egui::Key::ArrowRight) {
                pan.x -= 64.0;
            }
            if i.key_pressed(egui::Key::ArrowUp) {
                pan.y += 64.0;
            }
            if i.key_pressed(egui::Key::ArrowDown) {
                pan.y -= 64.0;
            }
            (
                cmd && (i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals)),
                cmd && i.key_pressed(egui::Key::Minus),
                cmd && i.key_pressed(egui::Key::Num0),
                pan,
            )
        });
        if zoom_in {
            viewport.zoom_about(center, 1.25);
        }
        if zoom_out {
            viewport.zoom_about(center, 0.8);
        }
        if zoom_reset {
            let factor = 1.0 / viewport.zoom;
            viewport.zoom_about(center, factor);
        }
        if pan != egui::Vec2::ZERO {
            viewport.pan_by([pan.x, pan.y]);
        }
    }

    let ppp = ui.ctx().pixels_per_point();
    let params = CheckerParams {
        transform: [
            (rect.min.x + viewport.pan[0]) * ppp,
            (rect.min.y + viewport.pan[1]) * ppp,
            viewport.zoom * ppp,
            8.0,
        ],
        ..Default::default()
    };
    ui.painter()
        .add(egui_wgpu::Callback::new_paint_callback(rect, CanvasCallback { params }));

    if let Some(state) = state {
        if !space_down {
            handle_tools(ui, rect, &response, viewport, state);
        }
        paint_document(ui, rect, viewport, state);
    }
}

/// Selected raster layer that can take pixel edits right now.
fn editable_raster(state: &EditorState) -> Option<NodeId> {
    let id = state.editor.selection?;
    let node = state.editor.doc.node(id)?;
    let editable = matches!(node.kind, NodeKind::Raster(_))
        && node.props.visible
        && !node.props.locked;
    editable.then_some(id)
}

/// Selected editable raster layer that has a mask (mask-edit target, spec 0050).
fn mask_layer(state: &EditorState) -> Option<NodeId> {
    let id = editable_raster(state)?;
    match &state.editor.doc.node(id)?.kind {
        NodeKind::Raster(c) if c.mask.is_some() => Some(id),
        _ => None,
    }
}

fn mask_mut(state: &mut EditorState, id: NodeId) -> Option<&mut atelier_core::Mask> {
    match &mut state.editor.doc.node_mut(id)?.kind {
        NodeKind::Raster(c) => c.mask.as_mut(),
        _ => None,
    }
}

fn mask_clone(state: &EditorState, id: NodeId) -> Option<atelier_core::Mask> {
    match &state.editor.doc.node(id)?.kind {
        NodeKind::Raster(c) => c.mask.clone(),
        _ => None,
    }
}

/// Brush/eraser painting into the active layer mask (doc space). Commits one
/// undoable `SetLayerMask` on release (spec 0050).
fn handle_mask_paint(
    state: &mut EditorState,
    response: &egui::Response,
    pointer_doc: impl Fn(egui::Pos2) -> [f32; 2],
    erase: bool,
) {
    let (r, hard) = (state.brush.radius, state.brush.hardness);
    if response.drag_started_by(egui::PointerButton::Primary) {
        let Some(id) = mask_layer(state) else { return };
        let Some(pos) = response.interact_pointer_pos() else { return };
        let before = mask_clone(state, id).expect("mask present");
        let p = pointer_doc(pos);
        if let Some(m) = mask_mut(state, id) {
            atelier_raster::stamp_mask_segment(m, p, p, r, hard, erase);
        }
        state.mask_stroke = Some((id, before, p));
        state.editor.history.touch();
    } else if response.dragged_by(egui::PointerButton::Primary) {
        if let Some((id, from)) = state.mask_stroke.as_ref().map(|(i, _, l)| (*i, *l)) {
            if let Some(pos) = response.interact_pointer_pos() {
                let p = pointer_doc(pos);
                if let Some(m) = mask_mut(state, id) {
                    atelier_raster::stamp_mask_segment(m, from, p, r, hard, erase);
                }
                if let Some(s) = &mut state.mask_stroke {
                    s.2 = p;
                }
                state.editor.history.touch();
            }
        }
    }
    if response.drag_stopped_by(egui::PointerButton::Primary) {
        if let Some((id, before, _)) = state.mask_stroke.take() {
            let new = mask_clone(state, id);
            // Restore the pre-stroke mask, then apply as an undoable command.
            if let Some(NodeKind::Raster(c)) = state.editor.doc.node_mut(id).map(|n| &mut n.kind) {
                c.mask = Some(before);
            }
            let cmd = SetLayerMask::new(&state.editor.doc, id, new);
            state.editor.apply(Box::new(cmd));
        }
    }
}

/// Selected layer the Move tool can reposition: a visible, unlocked `Raster` or
/// `Smart` node (spec 0054).
fn movable_layer(state: &EditorState) -> Option<NodeId> {
    let id = state.editor.selection?;
    let node = state.editor.doc.node(id)?;
    let movable = matches!(node.kind, NodeKind::Raster(_) | NodeKind::Smart(_))
        && node.props.visible
        && !node.props.locked;
    movable.then_some(id)
}

fn layer_offset(state: &EditorState, id: NodeId) -> [i32; 2] {
    match &state.editor.doc.node(id).expect("checked").kind {
        NodeKind::Raster(c) => c.offset,
        NodeKind::Smart(c) => c.offset,
        _ => [0, 0],
    }
}

fn handle_tools(
    ui: &egui::Ui,
    rect: egui::Rect,
    response: &egui::Response,
    vp: &Viewport,
    state: &mut EditorState,
) {
    let pointer_doc = |pos: egui::Pos2| {
        vp.screen_to_doc([pos.x - rect.min.x, pos.y - rect.min.y])
    };

    match state.tool {
        ActiveTool::Move => {
            let Some(id) = movable_layer(state) else { return };
            if response.drag_started_by(egui::PointerButton::Primary) {
                state.editor.history.set_merging(true);
            }
            if response.dragged_by(egui::PointerButton::Primary) {
                let d = response.drag_delta();
                if d != egui::Vec2::ZERO {
                    let old = layer_offset(state, id);
                    let new = [
                        old[0] + (d.x / vp.zoom).round() as i32,
                        old[1] + (d.y / vp.zoom).round() as i32,
                    ];
                    if new != old {
                        let cmd = SetOffset::new(&state.editor.doc, id, new);
                        state.editor.apply(Box::new(cmd));
                    }
                }
            }
            if response.drag_stopped_by(egui::PointerButton::Primary) {
                state.editor.history.set_merging(false);
            }
        }
        ActiveTool::Brush | ActiveTool::Eraser => {
            let erase = state.tool == ActiveTool::Eraser;

            // Mask-edit mode: brush/eraser paint into the selected layer's mask.
            if state.mask_edit && mask_layer(state).is_some() {
                handle_mask_paint(state, response, pointer_doc, erase);
                return;
            }

            let params = BrushParams {
                radius: state.brush.radius,
                hardness: state.brush.hardness,
                color: state.brush.color,
                erase,
            };

            // Selection clip (Arc clone keeps it alive while tiles are borrowed).
            let clip_mask = state.editor.doc.selection.clone();

            if response.drag_started_by(egui::PointerButton::Primary) {
                let Some(id) = editable_raster(state) else { return };
                let Some(pos) = response.interact_pointer_pos() else { return };
                let doc = pointer_doc(pos);
                let off = layer_offset(state, id);
                let p = [doc[0] - off[0] as f32, doc[1] - off[1] as f32];
                let mut stroke =
                    StrokeState { layer: id, last: p, capture: Default::default(), erase };
                let clip = clip_mask.as_deref().map(|m| (m, off));
                stroke_segment(state_doc(state, id), p, p, &params, clip, &mut stroke);
                mark_dirty(state, p, p, off, params.radius);
                state.stroke = Some(stroke);
            } else if response.dragged_by(egui::PointerButton::Primary) {
                let Some(mut stroke) = state.stroke.take() else { return };
                if let Some(pos) = response.interact_pointer_pos() {
                    let off = layer_offset(state, stroke.layer);
                    let doc = pointer_doc(pos);
                    let p = [doc[0] - off[0] as f32, doc[1] - off[1] as f32];
                    let last = stroke.last;
                    let clip = clip_mask.as_deref().map(|m| (m, off));
                    stroke_segment(state_doc(state, stroke.layer), last, p, &params, clip, &mut stroke);
                    mark_dirty(state, last, p, off, params.radius);
                    stroke.last = p;
                }
                state.stroke = Some(stroke);
            }
            if response.drag_stopped_by(egui::PointerButton::Primary) {
                if let Some(stroke) = state.stroke.take() {
                    let label = if stroke.erase { "Eraser Stroke" } else { "Brush Stroke" };
                    let cmd = PaintTiles::from_capture(
                        &state.editor.doc,
                        stroke.layer,
                        label,
                        stroke.capture,
                    );
                    state.editor.history.push_committed(Box::new(cmd));
                }
            }
        }
        ActiveTool::MagicWand => {
            if response.clicked() {
                if let Some(pos) = response.interact_pointer_pos() {
                    let d = pointer_doc(pos);
                    let (shift, alt) = ui.input(|i| (i.modifiers.shift, i.modifiers.alt));
                    // Defer to the app helper via a queued doc-space click.
                    state.wand_click = Some(([d[0].floor() as i32, d[1].floor() as i32], shift, alt));
                }
            }
        }
        ActiveTool::SelectRect | ActiveTool::SelectEllipse | ActiveTool::Lasso => {
            if response.drag_started_by(egui::PointerButton::Primary) {
                // `press_origin` is where the button went down; by drag-start the
                // live pointer has already moved, so `interact_pointer_pos` would
                // collapse start onto current.
                let origin = ui
                    .input(|i| i.pointer.press_origin())
                    .or_else(|| response.interact_pointer_pos());
                if let Some(pos) = origin {
                    let doc = pointer_doc(pos);
                    state.select_drag =
                        Some(SelectDrag { start: doc, current: doc, points: vec![doc] });
                }
            }
            if response.dragged_by(egui::PointerButton::Primary) {
                let latest = response.interact_pointer_pos();
                if let (Some(drag), Some(pos)) = (&mut state.select_drag, latest) {
                    let doc = pointer_doc(pos);
                    drag.current = doc;
                    // Lasso: only record meaningfully spaced points.
                    let last = drag.points.last().copied().unwrap_or(doc);
                    if (doc[0] - last[0]).abs() + (doc[1] - last[1]).abs() > 1.0 {
                        drag.points.push(doc);
                    }
                }
            }
            if response.drag_stopped_by(egui::PointerButton::Primary) {
                if let Some(drag) = state.select_drag.take() {
                    finish_selection(ui, state, drag);
                }
            }
        }
        ActiveTool::ShapeRect
        | ActiveTool::ShapeEllipse
        | ActiveTool::ShapePolygon
        | ActiveTool::ShapeStar => {
            // Rubber-band a shape (reuses select_drag for the live preview).
            if response.drag_started_by(egui::PointerButton::Primary) {
                let origin = ui
                    .input(|i| i.pointer.press_origin())
                    .or_else(|| response.interact_pointer_pos());
                if let Some(pos) = origin {
                    let doc = pointer_doc(pos);
                    state.select_drag =
                        Some(SelectDrag { start: doc, current: doc, points: vec![doc] });
                }
            }
            if response.dragged_by(egui::PointerButton::Primary) {
                if let (Some(drag), Some(pos)) =
                    (&mut state.select_drag, response.interact_pointer_pos())
                {
                    drag.current = pointer_doc(pos);
                }
            }
            if response.drag_stopped_by(egui::PointerButton::Primary) {
                if let Some(drag) = state.select_drag.take() {
                    let min =
                        [drag.start[0].min(drag.current[0]), drag.start[1].min(drag.current[1])];
                    let max =
                        [drag.start[0].max(drag.current[0]), drag.start[1].max(drag.current[1])];
                    if max[0] - min[0] >= 1.0 && max[1] - min[1] >= 1.0 {
                        if let Some(kind) = state.tool.shape_kind() {
                            state.pending_shape = Some((kind, min, max));
                        }
                    }
                }
            }
        }
        ActiveTool::DirectSelect => {
            // Drag an on-path anchor of the selected vector layer.
            let Some(id) = state.editor.selection else { return };
            let shapes = match state.editor.doc.node(id).map(|n| &n.kind) {
                Some(NodeKind::Vector(c)) => c.shapes.clone(),
                _ => return,
            };
            // Double-click on a segment inserts an anchor there (spec 0019).
            if response.double_clicked() {
                if let Some(p) = response.interact_pointer_pos() {
                    let q = pointer_doc(p);
                    // Pick the shape+segment closest in screen space (~10 px).
                    let mut hit: Option<(usize, usize)> = None;
                    let mut best = f32::INFINITY;
                    for (si, sh) in shapes.iter().enumerate() {
                        if let Some((ai, d)) = sh.path.closest_segment(q) {
                            let dscreen = d * vp.zoom;
                            if dscreen < 10.0 && dscreen < best {
                                best = dscreen;
                                hit = Some((si, ai));
                            }
                        }
                    }
                    if let Some((si, ai)) = hit {
                        let mut new_shapes = shapes.clone();
                        if new_shapes[si].path.insert_anchor(ai, q) {
                            let cmd = SetVectorShapes::new(&state.editor.doc, id, new_shapes);
                            state.editor.apply(Box::new(cmd));
                        }
                    }
                }
            }
            // Alt+click an anchor removes it (spec 0018).
            if response.clicked() && ui.input(|i| i.modifiers.alt) {
                if let Some(p) = response.interact_pointer_pos() {
                    if let Some((si, ai)) = nearest_anchor(&shapes, vp, pointer_doc(p)) {
                        let mut new_shapes = shapes.clone();
                        if new_shapes[si].path.remove_anchor(ai) {
                            let cmd = SetVectorShapes::new(&state.editor.doc, id, new_shapes);
                            state.editor.apply(Box::new(cmd));
                        }
                    }
                }
            }
            // Plain click selects an anchor (shows its bezier handles, spec 0021).
            if response.clicked() && !ui.input(|i| i.modifiers.alt) {
                if let Some(p) = response.interact_pointer_pos() {
                    state.selected_anchor = nearest_anchor(&shapes, vp, pointer_doc(p));
                }
            }
            if response.drag_started_by(egui::PointerButton::Primary) {
                let press = ui
                    .input(|i| i.pointer.press_origin())
                    .or_else(|| response.interact_pointer_pos());
                if let Some(p) = press {
                    let q = pointer_doc(p);
                    // Prefer grabbing a handle of the selected anchor; else an anchor.
                    if let Some(grab) = nearest_handle(&shapes, vp, q, state.selected_anchor) {
                        state.handle_drag = Some(grab);
                        state.editor.history.set_merging(true);
                    } else if let Some(idx) = nearest_anchor(&shapes, vp, q) {
                        state.anchor_drag = Some(idx);
                        state.selected_anchor = Some(idx);
                        state.editor.history.set_merging(true);
                    }
                }
            }
            if response.dragged_by(egui::PointerButton::Primary) {
                if let Some(pos) = response.interact_pointer_pos() {
                    let to = pointer_doc(pos);
                    if let Some((si, ai, is_out)) = state.handle_drag {
                        let mut new_shapes = shapes.clone();
                        if is_out {
                            new_shapes[si].path.set_out_handle(ai, to);
                        } else {
                            new_shapes[si].path.set_in_handle(ai, to);
                        }
                        let cmd = SetVectorShapes::new(&state.editor.doc, id, new_shapes);
                        state.editor.apply(Box::new(cmd));
                    } else if let Some((si, ai)) = state.anchor_drag {
                        let mut new_shapes = shapes.clone();
                        new_shapes[si].path.move_anchor(ai, to);
                        let cmd = SetVectorShapes::new(&state.editor.doc, id, new_shapes);
                        state.editor.apply(Box::new(cmd));
                    }
                }
            }
            if response.drag_stopped_by(egui::PointerButton::Primary) {
                let was = state.anchor_drag.take().is_some() || state.handle_drag.take().is_some();
                if was {
                    state.editor.history.set_merging(false);
                }
            }
        }
        ActiveTool::Pen => {
            // Click drops an anchor; clicking near the first anchor closes.
            if response.clicked() {
                if let Some(pos) = response.interact_pointer_pos() {
                    let doc = pointer_doc(pos);
                    let close = state.pen_points.len() >= 3 && {
                        let f = vp.doc_to_screen(state.pen_points[0]);
                        let c = vp.doc_to_screen(doc);
                        ((f[0] - c[0]).powi(2) + (f[1] - c[1]).powi(2)).sqrt() < 8.0
                    };
                    if close {
                        finish_pen(state, true);
                    } else {
                        state.pen_points.push(doc);
                    }
                }
            }
            let (enter, esc) = ui
                .input(|i| (i.key_pressed(egui::Key::Enter), i.key_pressed(egui::Key::Escape)));
            if enter && state.pen_points.len() >= 2 {
                let closed = state.pen_points.len() >= 3;
                finish_pen(state, closed);
            } else if enter || esc {
                state.pen_points.clear();
            }
        }
        ActiveTool::Eyedropper => {
            // Sample the composited document color into the brush + vector fill.
            if response.clicked() || response.dragged_by(egui::PointerButton::Primary) {
                if let Some(pos) = response.interact_pointer_pos() {
                    let d = pointer_doc(pos);
                    if let Some(c) = sample_composite(state, d) {
                        state.brush.color = c;
                        state.brush.vector_fill = c;
                    }
                }
            }
        }
        ActiveTool::Bucket => {
            // Flood-fill the contiguous region under the click with the brush color.
            if response.clicked() {
                if let Some(pos) = response.interact_pointer_pos() {
                    let d = pointer_doc(pos);
                    apply_bucket(state, [d[0].floor() as i32, d[1].floor() as i32]);
                }
            }
        }
        ActiveTool::Gradient => {
            // Drag defines the axis; on release fill selection/layer with a
            // foreground→transparent linear gradient (spec 0037).
            if response.drag_started_by(egui::PointerButton::Primary) {
                let origin = ui
                    .input(|i| i.pointer.press_origin())
                    .or_else(|| response.interact_pointer_pos());
                if let Some(pos) = origin {
                    let doc = pointer_doc(pos);
                    state.select_drag =
                        Some(SelectDrag { start: doc, current: doc, points: vec![doc] });
                }
            }
            if response.dragged_by(egui::PointerButton::Primary) {
                if let (Some(drag), Some(pos)) =
                    (&mut state.select_drag, response.interact_pointer_pos())
                {
                    drag.current = pointer_doc(pos);
                }
            }
            if response.drag_stopped_by(egui::PointerButton::Primary) {
                if let Some(drag) = state.select_drag.take() {
                    apply_gradient(state, drag.start, drag.current);
                }
            }
        }
    }
}

/// Test hook for the paint-bucket (avoids synthesizing a canvas click).
#[cfg(test)]
pub(crate) fn apply_bucket_for_test(state: &mut EditorState, seed: [i32; 2]) {
    apply_bucket(state, seed);
}

/// Paint-bucket: flood-select the contiguous region under `seed` (doc px) on
/// the active raster layer and fill it with the brush color. Undoable (0038).
fn apply_bucket(state: &mut EditorState, seed: [i32; 2]) {
    use atelier_core::{NodeKind, TILE_SIZE};
    let Some(id) = editable_raster(state) else { return };
    let offset = layer_offset(state, id);
    let size = state.editor.doc.size;
    let tol = state.brush.wand_tolerance;
    let color = state.brush.color;
    let mask = match &state.editor.doc.node(id).expect("checked").kind {
        NodeKind::Raster(c) => atelier_raster::selection::magic_wand(&c.tiles, offset, seed, tol, size),
        _ => return,
    };
    let Some(region) = mask.bounds() else { return };
    let region = [
        region[0].max(0),
        region[1].max(0),
        region[2].min(size[0] as i32),
        region[3].min(size[1] as i32),
    ];
    if region[0] >= region[2] || region[1] >= region[3] {
        return;
    }
    let t = TILE_SIZE as i32;
    let (lx0, ly0) = ((region[0] - offset[0]).div_euclid(t), (region[1] - offset[1]).div_euclid(t));
    let (lx1, ly1) =
        ((region[2] - 1 - offset[0]).div_euclid(t), (region[3] - 1 - offset[1]).div_euclid(t));
    let mut before = Vec::new();
    if let NodeKind::Raster(c) = &state.editor.doc.node(id).expect("checked").kind {
        for ty in ly0..=ly1 {
            for tx in lx0..=lx1 {
                before.push(((tx, ty), c.tiles.tile_at((tx, ty)).cloned()));
            }
        }
    }
    if let NodeKind::Raster(c) = &mut state.editor.doc.node_mut(id).expect("checked").kind {
        atelier_raster::fill_region(&mut c.tiles, color, offset, region, Some(&mask));
    }
    let cmd =
        atelier_core::command::PaintTiles::from_capture(&state.editor.doc, id, "Paint Bucket", before);
    state.editor.history.push_committed(Box::new(cmd));
}

/// Test hook for the gradient (linear or radial per brush.gradient_radial).
#[cfg(test)]
pub(crate) fn apply_gradient_for_test(state: &mut EditorState, p0: [f32; 2], p1: [f32; 2]) {
    apply_gradient(state, p0, p1);
}

/// Fill the selection (or whole layer) of the selected raster layer with a
/// foreground→transparent linear gradient along `p0`→`p1` (doc space). Undoable.
fn apply_gradient(state: &mut EditorState, p0: [f32; 2], p1: [f32; 2]) {
    use atelier_core::{NodeKind, TILE_SIZE};
    let Some(id) = editable_raster(state) else { return };
    let offset = layer_offset(state, id);
    let fg = state.brush.color;
    let c0 = fg;
    let c1 = [fg[0], fg[1], fg[2], 0.0];
    let mask = state.editor.doc.selection.clone();
    let [w, h] = state.editor.doc.size;
    let region = match mask.as_deref().and_then(|m| m.bounds()) {
        Some(b) => [b[0].max(0), b[1].max(0), b[2].min(w as i32), b[3].min(h as i32)],
        None => [0, 0, w as i32, h as i32],
    };
    if region[0] >= region[2] || region[1] >= region[3] {
        return;
    }
    let t = TILE_SIZE as i32;
    let (lx0, ly0) = ((region[0] - offset[0]).div_euclid(t), (region[1] - offset[1]).div_euclid(t));
    let (lx1, ly1) =
        ((region[2] - 1 - offset[0]).div_euclid(t), (region[3] - 1 - offset[1]).div_euclid(t));
    let mut before = Vec::new();
    if let NodeKind::Raster(c) = &state.editor.doc.node(id).expect("checked").kind {
        for ty in ly0..=ly1 {
            for tx in lx0..=lx1 {
                before.push(((tx, ty), c.tiles.tile_at((tx, ty)).cloned()));
            }
        }
    }
    let radial = state.brush.gradient_radial;
    if let NodeKind::Raster(c) = &mut state.editor.doc.node_mut(id).expect("checked").kind {
        if radial {
            atelier_raster::gradient_region_radial(
                &mut c.tiles, c0, c1, p0, p1, offset, region, mask.as_deref(),
            );
        } else {
            atelier_raster::gradient_region(
                &mut c.tiles, c0, c1, p0, p1, offset, region, mask.as_deref(),
            );
        }
    }
    let cmd =
        atelier_core::command::PaintTiles::from_capture(&state.editor.doc, id, "Gradient", before);
    state.editor.history.push_committed(Box::new(cmd));
}

/// Composited straight-alpha color at doc pixel `d`, None if out of bounds.
pub(crate) fn sample_composite(state: &EditorState, d: [f32; 2]) -> Option<[f32; 4]> {
    let [w, h] = state.editor.doc.size;
    let (x, y) = (d[0].floor() as i32, d[1].floor() as i32);
    if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 {
        return None;
    }
    let rgba = atelier_raster::composite_rgba8(&state.editor.doc, w, h);
    let i = ((y as usize * w as usize) + x as usize) * 4;
    Some([
        rgba[i] as f32 / 255.0,
        rgba[i + 1] as f32 / 255.0,
        rgba[i + 2] as f32 / 255.0,
        rgba[i + 3] as f32 / 255.0,
    ])
}

/// Nearest on-path anchor (shape idx, anchor idx) within ~10 screen px of
/// `target_doc`, across all shapes (spec 0017/0018).
fn nearest_anchor(
    shapes: &[atelier_core::atelier_vector::Shape],
    vp: &Viewport,
    target_doc: [f32; 2],
) -> Option<(usize, usize)> {
    let t = vp.doc_to_screen(target_doc);
    let mut best: Option<((usize, usize), f32)> = None;
    for (si, sh) in shapes.iter().enumerate() {
        for (ai, a) in sh.path.anchors().iter().enumerate() {
            let s = vp.doc_to_screen(*a);
            let d = ((s[0] - t[0]).powi(2) + (s[1] - t[1]).powi(2)).sqrt();
            if d < 10.0 && best.is_none_or(|(_, bd)| d < bd) {
                best = Some(((si, ai), d));
            }
        }
    }
    best.map(|(idx, _)| idx)
}

/// Nearest grabbable bezier handle (only the selected anchor's in/out handles)
/// within ~10 screen px of `target_doc`. Returns (shape, anchor, is_out). Spec 0021.
fn nearest_handle(
    shapes: &[atelier_core::atelier_vector::Shape],
    vp: &Viewport,
    target_doc: [f32; 2],
    selected: Option<(usize, usize)>,
) -> Option<(usize, usize, bool)> {
    let (si, ai) = selected?;
    let sh = shapes.get(si)?;
    let t = vp.doc_to_screen(target_doc);
    let mut best: Option<((usize, usize, bool), f32)> = None;
    for (handle, is_out) in [(sh.path.out_handle(ai), true), (sh.path.in_handle(ai), false)] {
        if let Some(hp) = handle {
            let s = vp.doc_to_screen(hp);
            let d = ((s[0] - t[0]).powi(2) + (s[1] - t[1]).powi(2)).sqrt();
            if d < 10.0 && best.is_none_or(|(_, bd)| d < bd) {
                best = Some(((si, ai, is_out), d));
            }
        }
    }
    best.map(|(g, _)| g)
}

/// Build a filled vector layer from the in-progress pen anchors (spec 0016).
fn finish_pen(state: &mut EditorState, closed: bool) {
    use atelier_core::atelier_vector::{Path, Shape};
    use atelier_core::{LayerProps, Node, NodeKind, VectorContent};
    let pts = std::mem::take(&mut state.pen_points);
    if pts.len() < 2 {
        return;
    }
    let path = Path::polyline(&pts, closed);
    let content = VectorContent { shapes: vec![Shape::filled(path, state.brush.vector_fill)] };

    let doc = &state.editor.doc;
    let (parent, index) = match state.editor.selection.and_then(|s| doc.node(s).map(|n| (s, n))) {
        Some((sel, n)) => {
            let parent = n.parent.unwrap_or(doc.root());
            let index = doc.children(parent).iter().position(|&c| c == sel).unwrap_or(0);
            (parent, index)
        }
        None => (doc.root(), 0),
    };
    let cmd = atelier_core::command::AddNode::new(
        &mut state.editor.doc,
        Node::new(LayerProps::named("Path"), NodeKind::Vector(content)),
        parent,
        index,
    );
    let id = cmd.id;
    state.editor.apply(Box::new(cmd));
    state.editor.selection = Some(id);
}

/// Build the shape mask, combine per modifiers, push the undoable command.
fn finish_selection(ui: &egui::Ui, state: &mut EditorState, drag: SelectDrag) {
    let (s, c) = (drag.start, drag.current);
    let (shape, label) = match state.tool {
        ActiveTool::SelectRect => {
            (selection::rect_mask(s[0], s[1], c[0], c[1]), "Rectangular Select")
        }
        ActiveTool::SelectEllipse => {
            (selection::ellipse_mask(s[0], s[1], c[0], c[1]), "Elliptical Select")
        }
        ActiveTool::Lasso => (selection::polygon_mask(&drag.points), "Lasso Select"),
        _ => return,
    };

    let (shift, alt) = ui.input(|i| (i.modifiers.shift, i.modifiers.alt));
    let op = match (shift, alt) {
        (true, true) => CombineOp::Intersect,
        (true, false) => CombineOp::Add,
        (false, true) => CombineOp::Subtract,
        (false, false) => CombineOp::Replace,
    };

    let combined = match (&state.editor.doc.selection, op) {
        (Some(current), op) if op != CombineOp::Replace => {
            let mut m = (**current).clone();
            m.combine(&shape, op);
            m
        }
        _ => shape,
    };
    let new = (!combined.is_empty()).then(|| Arc::new(combined));
    // Replace with an empty drag = no-op rather than an accidental deselect.
    if new.is_none() && state.editor.doc.selection.is_none() {
        return;
    }
    let cmd = SetSelection::new(&state.editor.doc, new, label);
    state.editor.apply(Box::new(cmd));
}

/// Mutable access to a raster layer's tiles (live-stroke path; the committed
/// PaintTiles command preserves undo integrity — spec 0005 design note).
fn state_doc(state: &mut EditorState, id: NodeId) -> &mut atelier_core::TileMap {
    match &mut state.editor.doc.node_mut(id).expect("layer exists").kind {
        NodeKind::Raster(c) => &mut c.tiles,
        _ => unreachable!("editable_raster guards kind"),
    }
}

/// Union the segment's doc-space bbox (layer coords + offset, padded by the
/// brush radius) into the frame's dirty-patch rect (spec 0006).
fn mark_dirty(state: &mut EditorState, a: [f32; 2], b: [f32; 2], off: [i32; 2], radius: f32) {
    let r = radius + 2.0;
    let rect = [
        (a[0].min(b[0]) - r).floor() as i32 + off[0],
        (a[1].min(b[1]) - r).floor() as i32 + off[1],
        (a[0].max(b[0]) + r).ceil() as i32 + off[0],
        (a[1].max(b[1]) + r).ceil() as i32 + off[1],
    ];
    state.dirty_patch = Some(match state.dirty_patch {
        None => rect,
        Some(d) => [d[0].min(rect[0]), d[1].min(rect[1]), d[2].max(rect[2]), d[3].max(rect[3])],
    });
}

/// Capture-then-stamp one segment, optionally selection-clipped.
fn stroke_segment(
    tiles: &mut atelier_core::TileMap,
    from: [f32; 2],
    to: [f32; 2],
    params: &BrushParams,
    clip: Option<(&atelier_core::Mask, [i32; 2])>,
    stroke: &mut StrokeState,
) {
    for coord in atelier_raster::segment_tiles(from, to, params.radius) {
        stroke.capture.entry(coord).or_insert_with(|| tiles.tile_at(coord).cloned());
    }
    atelier_raster::stamp_segment_clipped(tiles, from, to, params, clip);
}

/// Map a document-space rect to screen space within the canvas rect.
fn doc_rect_to_screen(canvas: egui::Rect, vp: &Viewport, bounds: [f32; 4]) -> egui::Rect {
    let min = vp.doc_to_screen([bounds[0], bounds[1]]);
    let max = vp.doc_to_screen([bounds[0] + bounds[2], bounds[1] + bounds[3]]);
    egui::Rect::from_min_max(
        canvas.min + egui::vec2(min[0], min[1]),
        canvas.min + egui::vec2(max[0], max[1]),
    )
}

fn paint_document(ui: &egui::Ui, rect: egui::Rect, vp: &Viewport, state: &mut EditorState) {
    let [w, h] = state.editor.doc.size;

    // Recomposite only when the document actually changed.
    let rev = state.editor.history.revision();
    let stale = state.composite.as_ref().is_none_or(|(r, _)| *r != rev);
    if stale {
        let rgba = atelier_raster::composite_rgba8(&state.editor.doc, w, h);
        let image =
            egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
        let tex =
            ui.ctx().load_texture("doc-composite", image, egui::TextureOptions::NEAREST);
        state.composite = Some((rev, tex));
    }

    // Live-stroke partial update: recomposite only the dirtied region and
    // patch it into the cached texture (spec 0006).
    if let Some(d) = state.dirty_patch.take() {
        let (x0, y0) = (d[0].max(0), d[1].max(0));
        let (x1, y1) = (d[2].min(w as i32), d[3].min(h as i32));
        if x1 > x0 && y1 > y0 {
            let (pw, ph) = ((x1 - x0) as u32, (y1 - y0) as u32);
            let rgba =
                atelier_raster::compositor::composite_region_rgba8(&state.editor.doc, x0, y0, pw, ph);
            let image =
                egui::ColorImage::from_rgba_unmultiplied([pw as usize, ph as usize], &rgba);
            if let Some((_, tex)) = &mut state.composite {
                tex.set_partial([x0 as usize, y0 as usize], image, egui::TextureOptions::NEAREST);
            }
        }
    }

    let painter = ui.painter().with_clip_rect(rect);
    let doc_rect = doc_rect_to_screen(rect, vp, [0.0, 0.0, w as f32, h as f32]);

    let (_, tex) = state.composite.as_ref().expect("filled above");
    painter.image(
        tex.id(),
        doc_rect,
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );
    painter.rect_stroke(
        doc_rect,
        0.0,
        egui::Stroke::new(1.0_f32, egui::Color32::from_gray(200)),
        egui::StrokeKind::Outside,
    );

    // Vector layers now composite inline (spec 0051), so they're already in the
    // document texture above — no separate overlay.

    // Selection: marching ants (cached per revision) + live drag preview.
    if let Some(mask) = &state.editor.doc.selection {
        let rev = state.editor.history.revision();
        if state.ants.as_ref().is_none_or(|(r, _)| *r != rev) {
            state.ants = Some((rev, selection::boundary_segments(mask)));
        }
        if let Some((_, segs)) = &state.ants {
            let to_screen = |p: [f32; 2]| {
                let s = vp.doc_to_screen(p);
                rect.min + egui::vec2(s[0], s[1])
            };
            for (a, b) in segs {
                let (pa, pb) = (to_screen(*a), to_screen(*b));
                painter.line_segment([pa, pb], egui::Stroke::new(2.0_f32, egui::Color32::BLACK));
                painter.line_segment([pa, pb], egui::Stroke::new(1.0_f32, egui::Color32::WHITE));
            }
        }
    } else {
        state.ants = None;
    }
    if let Some(drag) = &state.select_drag {
        let to_screen = |p: [f32; 2]| {
            let s = vp.doc_to_screen(p);
            rect.min + egui::vec2(s[0], s[1])
        };
        let stroke = egui::Stroke::new(1.0_f32, egui::Color32::from_gray(230));
        match state.tool {
            ActiveTool::SelectRect
            | ActiveTool::ShapeRect
            | ActiveTool::ShapePolygon
            | ActiveTool::ShapeStar => {
                // Polygon/star preview as their bounding box (cheap rubber band).
                let r = egui::Rect::from_two_pos(to_screen(drag.start), to_screen(drag.current));
                painter.rect_stroke(r, 0.0, stroke, egui::StrokeKind::Middle);
            }
            ActiveTool::SelectEllipse | ActiveTool::ShapeEllipse => {
                let r = egui::Rect::from_two_pos(to_screen(drag.start), to_screen(drag.current));
                painter.add(egui::Shape::ellipse_stroke(r.center(), r.size() * 0.5, stroke));
            }
            ActiveTool::Lasso => {
                let pts: Vec<egui::Pos2> = drag.points.iter().map(|&p| to_screen(p)).collect();
                painter.add(egui::Shape::line(pts, stroke));
            }
            ActiveTool::Gradient => {
                painter.line_segment([to_screen(drag.start), to_screen(drag.current)], stroke);
            }
            _ => {}
        }
    }

    // Direct-select: anchor dots + bezier handles for the selected anchor.
    if state.tool == ActiveTool::DirectSelect {
        if let Some(node) = state.editor.selection.and_then(|id| state.editor.doc.node(id)) {
            if let NodeKind::Vector(c) = &node.kind {
                let to_screen = |p: [f32; 2]| {
                    let s = vp.doc_to_screen(p);
                    rect.min + egui::vec2(s[0], s[1])
                };
                let accent = egui::Color32::from_rgb(90, 170, 255);
                for sh in &c.shapes {
                    for a in sh.path.anchors() {
                        let p = to_screen(a);
                        painter.circle_filled(p, 3.5, egui::Color32::WHITE);
                        painter.circle_stroke(p, 3.5, egui::Stroke::new(1.0_f32, accent));
                    }
                }
                // Handles for the selected anchor (drag targets).
                if let Some((si, ai)) = state.selected_anchor {
                    if let Some(sh) = c.shapes.get(si) {
                        let ap = sh.path.anchors().get(ai).copied();
                        if let Some(ap) = ap {
                            let aps = to_screen(ap);
                            for hp in [sh.path.out_handle(ai), sh.path.in_handle(ai)]
                                .into_iter()
                                .flatten()
                            {
                                let hs = to_screen(hp);
                                painter.line_segment([aps, hs], egui::Stroke::new(1.0_f32, accent));
                                painter.circle_filled(hs, 3.0, accent);
                            }
                        }
                    }
                }
            }
        }
    }

    // Pen tool: in-progress polyline + anchor dots + rubber band to cursor.
    if state.tool == ActiveTool::Pen && !state.pen_points.is_empty() {
        let to_screen = |p: [f32; 2]| {
            let s = vp.doc_to_screen(p);
            rect.min + egui::vec2(s[0], s[1])
        };
        let stroke = egui::Stroke::new(1.5_f32, egui::Color32::from_rgb(90, 170, 255));
        let pts: Vec<egui::Pos2> = state.pen_points.iter().map(|&p| to_screen(p)).collect();
        if pts.len() >= 2 {
            painter.add(egui::Shape::line(pts.clone(), stroke));
        }
        for p in &pts {
            painter.circle_filled(*p, 3.0, egui::Color32::WHITE);
            painter.circle_stroke(*p, 3.0, egui::Stroke::new(1.0_f32, egui::Color32::BLACK));
        }
        if let Some(cur) = ui.input(|i| i.pointer.latest_pos()) {
            if rect.contains(cur) {
                painter.line_segment(
                    [*pts.last().expect("non-empty"), cur],
                    egui::Stroke::new(1.0_f32, egui::Color32::from_gray(160)),
                );
            }
        }
    }

    // Selected raster layer: coarse tile-bounds outline.
    if let Some(node) = state.editor.selection.and_then(|id| state.editor.doc.node(id)) {
        if let NodeKind::Raster(content) = &node.kind {
            if let Some([x0, y0, x1, y1]) = content.tiles.bounds() {
                let [ox, oy] = content.offset;
                let r = doc_rect_to_screen(
                    rect,
                    vp,
                    [(x0 + ox) as f32, (y0 + oy) as f32, (x1 - x0) as f32, (y1 - y0) as f32],
                );
                painter.rect_stroke(
                    r,
                    0.0,
                    egui::Stroke::new(1.5_f32, egui::Color32::from_rgb(90, 170, 255)),
                    egui::StrokeKind::Outside,
                );
            }
        }
    }
}

pub struct CanvasCallback {
    pub params: CheckerParams,
}

impl egui_wgpu::CallbackTrait for CanvasCallback {
    fn prepare(
        &self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen_descriptor: &egui_wgpu::ScreenDescriptor,
        _egui_encoder: &mut wgpu::CommandEncoder,
        callback_resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        if let Some(renderer) = callback_resources.get::<CheckerboardRenderer>() {
            renderer.update(queue, &self.params);
        }
        Vec::new()
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        callback_resources: &egui_wgpu::CallbackResources,
    ) {
        if let Some(renderer) = callback_resources.get::<CheckerboardRenderer>() {
            renderer.paint(render_pass);
        }
    }
}
