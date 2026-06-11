//! Canvas tab: checkerboard background (GPU callback) + Phase-1 placeholder
//! layer rects painted with egui until the raster/vector engines land.

use atelier_gpu::{CheckerParams, CheckerboardRenderer, Viewport};
use atelier_core::{Editor, NodeId, NodeKind};
use eframe::egui_wgpu::{self, wgpu};

pub fn canvas_ui(ui: &mut egui::Ui, viewport: &mut Viewport, editor: Option<&Editor>) {
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

    if let Some(editor) = editor {
        paint_document_overlay(ui, rect, viewport, editor);
    }
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

fn paint_document_overlay(ui: &egui::Ui, rect: egui::Rect, vp: &Viewport, editor: &Editor) {
    let painter = ui.painter().with_clip_rect(rect);
    let doc = &editor.doc;

    let doc_rect =
        doc_rect_to_screen(rect, vp, [0.0, 0.0, doc.size[0] as f32, doc.size[1] as f32]);
    painter.rect_stroke(
        doc_rect,
        0.0,
        egui::Stroke::new(1.0, egui::Color32::from_gray(200)),
        egui::StrokeKind::Outside,
    );

    paint_children(&painter, rect, vp, editor, doc.root(), 1.0);
}

/// Bottom-of-panel-list renders first (painter's algorithm): children vecs are
/// top-first, so iterate in reverse.
fn paint_children(
    painter: &egui::Painter,
    rect: egui::Rect,
    vp: &Viewport,
    editor: &Editor,
    parent: NodeId,
    opacity: f32,
) {
    for &id in editor.doc.children(parent).iter().rev() {
        let Some(node) = editor.doc.node(id) else { continue };
        if !node.props.visible {
            continue;
        }
        let opacity = opacity * node.props.opacity;
        match &node.kind {
            NodeKind::Group { .. } => paint_children(painter, rect, vp, editor, id, opacity),
            NodeKind::Raster(art) | NodeKind::Vector(art) => {
                let r = doc_rect_to_screen(rect, vp, art.bounds);
                let c = art.color;
                let fill = egui::Color32::from_rgba_unmultiplied(
                    (c[0] * 255.0) as u8,
                    (c[1] * 255.0) as u8,
                    (c[2] * 255.0) as u8,
                    (c[3] * opacity * 255.0) as u8,
                );
                painter.rect_filled(r, 2.0, fill);
                if editor.selection == Some(id) {
                    painter.rect_stroke(
                        r,
                        2.0,
                        egui::Stroke::new(2.0, egui::Color32::from_rgb(90, 170, 255)),
                        egui::StrokeKind::Outside,
                    );
                }
            }
            _ => {}
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
