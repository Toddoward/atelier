//! Canvas tab: checkerboard background (GPU callback) + the composited
//! document as a nearest-filtered texture, recomposited when the history
//! revision changes (spec 0004).

use crate::{ActiveTool, EditorState, SelectDrag, StrokeState};
use atelier_core::command::{PaintTiles, SetOffset, SetSelection};
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

fn raster_offset(state: &EditorState, id: NodeId) -> [i32; 2] {
    match &state.editor.doc.node(id).expect("checked").kind {
        NodeKind::Raster(c) => c.offset,
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
            let Some(id) = editable_raster(state) else { return };
            if response.drag_started_by(egui::PointerButton::Primary) {
                state.editor.history.set_merging(true);
            }
            if response.dragged_by(egui::PointerButton::Primary) {
                let d = response.drag_delta();
                if d != egui::Vec2::ZERO {
                    let old = raster_offset(state, id);
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
                let off = raster_offset(state, id);
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
                    let off = raster_offset(state, stroke.layer);
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
    }
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
        egui::Stroke::new(1.0, egui::Color32::from_gray(200)),
        egui::StrokeKind::Outside,
    );

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
                painter.line_segment([pa, pb], egui::Stroke::new(2.0, egui::Color32::BLACK));
                painter.line_segment([pa, pb], egui::Stroke::new(1.0, egui::Color32::WHITE));
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
        let stroke = egui::Stroke::new(1.0, egui::Color32::from_gray(230));
        match state.tool {
            ActiveTool::SelectRect | ActiveTool::SelectEllipse => {
                let r = egui::Rect::from_two_pos(to_screen(drag.start), to_screen(drag.current));
                if state.tool == ActiveTool::SelectRect {
                    painter.rect_stroke(r, 0.0, stroke, egui::StrokeKind::Middle);
                } else {
                    painter.add(egui::Shape::ellipse_stroke(
                        r.center(),
                        r.size() * 0.5,
                        stroke,
                    ));
                }
            }
            ActiveTool::Lasso => {
                let pts: Vec<egui::Pos2> = drag.points.iter().map(|&p| to_screen(p)).collect();
                painter.add(egui::Shape::line(pts, stroke));
            }
            _ => {}
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
                    egui::Stroke::new(1.5, egui::Color32::from_rgb(90, 170, 255)),
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
