//! Canvas tab: checkerboard background (GPU callback) + the composited
//! document as a nearest-filtered texture, recomposited when the history
//! revision changes (spec 0004).

use crate::{ActiveTool, EditorState, StrokeState};
use atelier_core::command::{PaintTiles, SetOffset};
use atelier_core::{NodeId, NodeKind};
use atelier_gpu::{CheckerParams, CheckerboardRenderer, Viewport};
use atelier_raster::BrushParams;
use eframe::egui_wgpu::{self, wgpu};

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
    let _ = ui;
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

            if response.drag_started_by(egui::PointerButton::Primary) {
                let Some(id) = editable_raster(state) else { return };
                let Some(pos) = response.interact_pointer_pos() else { return };
                let doc = pointer_doc(pos);
                let off = raster_offset(state, id);
                let p = [doc[0] - off[0] as f32, doc[1] - off[1] as f32];
                let mut stroke =
                    StrokeState { layer: id, last: p, capture: Default::default(), erase };
                stroke_segment(state_doc(state, id), p, p, &params, &mut stroke);
                state.stroke = Some(stroke);
                state.editor.history.touch();
            } else if response.dragged_by(egui::PointerButton::Primary) {
                let Some(mut stroke) = state.stroke.take() else { return };
                if let Some(pos) = response.interact_pointer_pos() {
                    let off = raster_offset(state, stroke.layer);
                    let doc = pointer_doc(pos);
                    let p = [doc[0] - off[0] as f32, doc[1] - off[1] as f32];
                    let last = stroke.last;
                    stroke_segment(state_doc(state, stroke.layer), last, p, &params, &mut stroke);
                    stroke.last = p;
                    state.editor.history.touch();
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
    }
}

/// Mutable access to a raster layer's tiles (live-stroke path; the committed
/// PaintTiles command preserves undo integrity — spec 0005 design note).
fn state_doc(state: &mut EditorState, id: NodeId) -> &mut atelier_core::TileMap {
    match &mut state.editor.doc.node_mut(id).expect("layer exists").kind {
        NodeKind::Raster(c) => &mut c.tiles,
        _ => unreachable!("editable_raster guards kind"),
    }
}

/// Capture-then-stamp one segment.
fn stroke_segment(
    tiles: &mut atelier_core::TileMap,
    from: [f32; 2],
    to: [f32; 2],
    params: &BrushParams,
    stroke: &mut StrokeState,
) {
    for coord in atelier_raster::segment_tiles(from, to, params.radius) {
        stroke.capture.entry(coord).or_insert_with(|| tiles.tile_at(coord).cloned());
    }
    atelier_raster::stamp_segment(tiles, from, to, params);
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
