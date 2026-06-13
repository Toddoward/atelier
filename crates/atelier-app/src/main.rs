//! Atelier application shell: window, wgpu surface, docked panels, document
//! lifecycle (specs 0001 + 0002).

mod canvas;
mod panels;

use atelier_core::{Editor, NodeId, ProjectFocus};
use atelier_gpu::{CheckerboardRenderer, Viewport};
use egui_dock::{DockArea, DockState, NodeIndex};
use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt::init();
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!("panic: {info}");
        default_hook(info);
    }));

    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: egui::ViewportBuilder::default()
            .with_title("Atelier")
            .with_inner_size([1440.0, 900.0]),
        ..Default::default()
    };
    eframe::run_native("Atelier", options, Box::new(|cc| Ok(Box::new(AtelierApp::new(cc)))))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Canvas,
    Tools,
    Layers,
    Properties,
    History,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTool {
    Move,
    Brush,
    Eraser,
    SelectRect,
    SelectEllipse,
    Lasso,
    MagicWand,
    ShapeRect,
    ShapeEllipse,
}

/// Brush/eraser options (Tools panel).
pub struct BrushSettings {
    pub radius: f32,
    pub hardness: f32,
    pub color: [f32; 4],
    /// Magic-wand color tolerance (0..=255).
    pub wand_tolerance: u8,
    /// Fill color for newly drawn vector shapes.
    pub vector_fill: [f32; 4],
}

impl Default for BrushSettings {
    fn default() -> Self {
        Self {
            radius: 16.0,
            hardness: 0.8,
            color: [0.1, 0.1, 0.1, 1.0],
            wand_tolerance: 32,
            vector_fill: [0.2, 0.5, 0.9, 1.0],
        }
    }
}

/// Live brush stroke (committed as one PaintTiles command on release).
pub struct StrokeState {
    pub layer: NodeId,
    /// Last stamp position in layer coords.
    pub last: [f32; 2],
    /// Pre-stroke tiles for undo, captured before first mutation.
    pub capture: std::collections::BTreeMap<atelier_core::TileCoord, Option<atelier_core::Tile>>,
    pub erase: bool,
}

/// One open document plus its UI editing state.
pub struct EditorState {
    pub editor: Editor,
    pub path: Option<PathBuf>,
    /// In-progress rename in the Layers panel: (node, text buffer).
    pub rename: Option<(NodeId, String)>,
    /// Cached document composite keyed by history revision (spec 0004).
    pub composite: Option<(u64, egui::TextureHandle)>,
    pub tool: ActiveTool,
    pub brush: BrushSettings,
    pub stroke: Option<StrokeState>,
    /// Doc-space rect the live stroke dirtied this frame — the canvas patches
    /// just this region instead of recompositing the document (spec 0006).
    pub dirty_patch: Option<[i32; 4]>,
    /// In-progress selection drag (doc coords; points only used by Lasso).
    pub select_drag: Option<SelectDrag>,
    /// Marching-ants boundary cache, keyed by history revision (spec 0007).
    pub ants: Option<(u64, AntSegments)>,
    /// Pending magic-wand click (doc pixel, shift, alt) — drained by the app
    /// loop into `magic_wand_at` (canvas can't borrow the app helper).
    pub wand_click: Option<([i32; 2], bool, bool)>,
    /// Tessellated vector-layer meshes (doc space), cached by history revision
    /// (spec 0013). Re-tessellate only on document change.
    pub vector_cache: Option<(u64, Vec<(NodeId, atelier_core::atelier_vector::Mesh)>)>,
    /// Pending shape insertion (is_ellipse, doc min, doc max) from a shape-tool
    /// drag — drained by the app loop into `add_shape_layer` (spec 0014).
    pub pending_shape: Option<(bool, [f32; 2], [f32; 2])>,
}

/// Doc-space unit segments outlining the selection boundary.
pub type AntSegments = Vec<([f32; 2], [f32; 2])>;

pub struct SelectDrag {
    pub start: [f32; 2],
    pub current: [f32; 2],
    pub points: Vec<[f32; 2]>,
}

impl EditorState {
    /// Monotonic-ish counter for default layer/group names.
    pub fn layer_counter(&self) -> usize {
        self.editor.doc.node_count()
    }
}

struct NewDocDialog {
    width: u32,
    height: u32,
    focus: ProjectFocus,
}

impl Default for NewDocDialog {
    fn default() -> Self {
        Self { width: 1920, height: 1080, focus: ProjectFocus::Raster }
    }
}

#[derive(Clone, Copy)]
enum PendingAction {
    New,
    Open,
}

struct AtelierApp {
    dock: DockState<Tab>,
    viewport: Viewport,
    adapter_info: String,
    state: Option<EditorState>,
    new_doc: Option<NewDocDialog>,
    /// Image → Canvas Size… dialog (pending width/height).
    canvas_size: Option<[u32; 2]>,
    /// Adjust dialog (parametric adjustment being edited).
    adjust_dialog: Option<atelier_raster::Adjustment>,
    /// Layer → Transform… dialog (scale% x, scale% y, rotate°).
    transform_dialog: Option<[f32; 3]>,
    /// Image → Image Size… dialog (target size).
    image_size_dialog: Option<[u32; 2]>,
    pending: Option<PendingAction>,
    error: Option<String>,
    last_title: String,
}

impl AtelierApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let rs = cc
            .wgpu_render_state
            .as_ref()
            .expect("Atelier requires the wgpu renderer (eframe::Renderer::Wgpu)");
        rs.renderer
            .write()
            .callback_resources
            .insert(CheckerboardRenderer::new(&rs.device, rs.target_format));

        let info = rs.adapter.get_info();
        let adapter_info = format!("{} · {:?}", info.name, info.backend);
        tracing::info!(adapter = %info.name, backend = ?info.backend, "wgpu initialized");

        Self::with_adapter_info(adapter_info)
    }

    /// Construct without a GPU/eframe context (kittest UI tests). The canvas
    /// paint callback is inert when no CheckerboardRenderer resource exists.
    fn with_adapter_info(adapter_info: String) -> Self {
        let mut dock = DockState::new(vec![Tab::Canvas]);
        let surface = dock.main_surface_mut();
        let [canvas, _tools] = surface.split_left(NodeIndex::root(), 0.15, vec![Tab::Tools]);
        let [_, right] = surface.split_right(canvas, 0.8, vec![Tab::Layers]);
        surface.split_below(right, 0.5, vec![Tab::Properties, Tab::History]);

        Self {
            dock,
            viewport: Viewport::default(),
            adapter_info,
            state: None,
            new_doc: None,
            canvas_size: None,
            adjust_dialog: None,
            transform_dialog: None,
            image_size_dialog: None,
            pending: None,
            error: None,
            last_title: String::new(),
        }
    }

    fn is_dirty(&self) -> bool {
        self.state.as_ref().is_some_and(|s| s.editor.is_dirty())
    }

    fn request_new(&mut self) {
        if self.is_dirty() {
            self.pending = Some(PendingAction::New);
        } else {
            self.new_doc = Some(NewDocDialog::default());
        }
    }

    fn request_open(&mut self) {
        if self.is_dirty() {
            self.pending = Some(PendingAction::Open);
        } else {
            self.do_open();
        }
    }

    fn do_open(&mut self) {
        let Some(path) =
            rfd::FileDialog::new().add_filter("Atelier document", &["atl"]).pick_file()
        else {
            return;
        };
        self.open_from(path);
    }

    /// Dialog-free open path (also used by UI tests).
    fn open_from(&mut self, path: PathBuf) {
        match atelier_io::load_atl(&path) {
            Ok(doc) => {
                self.state = Some(EditorState {
                    editor: Editor::from_document(doc),
                    path: Some(path),
                    rename: None,
                    composite: None,
                    tool: ActiveTool::Move,
                    brush: BrushSettings::default(),
                    stroke: None,
                    dirty_patch: None,
                    select_drag: None,
                    ants: None,
                    wand_click: None,
                    vector_cache: None,
                    pending_shape: None,
                });
            }
            Err(e) => self.error = Some(e.to_string()),
        }
    }

    fn save(&mut self, save_as: bool) {
        let Some(st) = &self.state else { return };
        let path = if save_as || st.path.is_none() {
            rfd::FileDialog::new()
                .add_filter("Atelier document", &["atl"])
                .set_file_name("untitled.atl")
                .save_file()
        } else {
            st.path.clone()
        };
        let Some(path) = path else { return };
        self.save_to(path);
    }

    /// Dialog-free save path (also used by UI tests).
    fn save_to(&mut self, path: PathBuf) {
        let Some(st) = &mut self.state else { return };
        match atelier_io::save_atl(&st.editor.doc, &path) {
            Ok(()) => {
                st.path = Some(path);
                st.editor.mark_saved();
            }
            Err(e) => self.error = Some(e.to_string()),
        }
    }

    /// Bake a scale/rotate into the selected raster layer (spec 0010, D-13).
    fn apply_transform(&mut self, scale_x: f32, scale_y: f32, rotate_deg: f32) {
        use atelier_core::NodeKind;
        let Some(st) = &mut self.state else { return };
        let Some(id) = st.editor.selection else { return };
        let tiles = match &st.editor.doc.node(id).map(|n| (&n.kind, &n.props)) {
            Some((NodeKind::Raster(c), p)) if p.visible && !p.locked => c.tiles.clone(),
            _ => return,
        };
        let new_tiles = atelier_raster::transform_layer(
            &tiles,
            scale_x / 100.0,
            scale_y / 100.0,
            rotate_deg.to_radians(),
        );
        let offset = match &st.editor.doc.node(id).expect("checked").kind {
            NodeKind::Raster(c) => c.offset,
            _ => return,
        };
        let cmd = atelier_core::command::ReplaceLayerTiles::new(
            &st.editor.doc,
            id,
            new_tiles,
            offset,
            "Transform Layer",
        );
        st.editor.apply(Box::new(cmd));
    }

    /// Crop the canvas to the current selection's bounds.
    fn crop_to_selection(&mut self) {
        let Some(st) = &mut self.state else { return };
        let Some(rect) = st.editor.doc.selection.as_deref().and_then(|m| m.pixel_bounds()) else {
            return;
        };
        let cmd = atelier_core::command::CropCanvas::new(&st.editor.doc, rect);
        st.editor.apply(Box::new(cmd));
        // The selection's coordinates no longer match the cropped canvas.
        let deselect =
            atelier_core::command::SetSelection::new(&st.editor.doc, None, "Deselect");
        st.editor.apply(Box::new(deselect));
    }

    /// Resample every raster layer + set the document size (Image Size).
    fn apply_resample(&mut self, new_size: [u32; 2]) {
        use atelier_core::NodeKind;
        let Some(st) = &mut self.state else { return };
        let old = st.editor.doc.size;
        if new_size == old || old[0] == 0 || old[1] == 0 {
            return;
        }
        // Uniform scale from width ratio (height follows for non-uniform too).
        let sx = new_size[0] as f32 / old[0] as f32;
        let sy = new_size[1] as f32 / old[1] as f32;
        let scale = (sx + sy) * 0.5; // single factor (bilinear); near-uniform expected
        let mut baked = Vec::new();
        for (id, _) in st.editor.doc.iter_tree() {
            if let Some(NodeKind::Raster(c)) = st.editor.doc.node(id).map(|n| &n.kind) {
                let (tiles, offset) = atelier_raster::resample_layer(&c.tiles, c.offset, scale);
                baked.push((id, (tiles, offset)));
            }
        }
        let cmd = atelier_core::command::ResizeImage::new(&st.editor.doc, new_size, baked);
        st.editor.apply(Box::new(cmd));
    }

    /// Magic-wand select at a doc pixel, combining per modifiers.
    fn magic_wand_at(&mut self, doc: [i32; 2], shift: bool, alt: bool) {
        use atelier_core::{CombineOp, NodeKind};
        let Some(st) = &mut self.state else { return };
        let Some(id) = st.editor.selection else { return };
        let (tiles, offset) = match &st.editor.doc.node(id).map(|n| &n.kind) {
            Some(NodeKind::Raster(c)) => (c.tiles.clone(), c.offset),
            _ => return,
        };
        let size = st.editor.doc.size;
        let shape = atelier_raster::selection::magic_wand(
            &tiles,
            offset,
            doc,
            st.brush.wand_tolerance,
            size,
        );
        let op = match (shift, alt) {
            (true, true) => CombineOp::Intersect,
            (true, false) => CombineOp::Add,
            (false, true) => CombineOp::Subtract,
            (false, false) => CombineOp::Replace,
        };
        let combined = match (&st.editor.doc.selection, op) {
            (Some(cur), op) if op != CombineOp::Replace => {
                let mut m = (**cur).clone();
                m.combine(&shape, op);
                m
            }
            _ => shape,
        };
        let new = (!combined.is_empty()).then(|| std::sync::Arc::new(combined));
        if new.is_none() && st.editor.doc.selection.is_none() {
            return;
        }
        let cmd = atelier_core::command::SetSelection::new(&st.editor.doc, new, "Magic Wand");
        st.editor.apply(Box::new(cmd));
    }

    /// Replace the selection with a transformed version of itself (Select menu).
    fn set_selection<F: FnOnce(&atelier_core::Mask, [u32; 2]) -> Option<atelier_core::Mask>>(
        &mut self,
        label: &str,
        f: F,
    ) {
        let Some(st) = &mut self.state else { return };
        let size = st.editor.doc.size;
        let cur = st.editor.doc.selection.clone();
        let new = match &cur {
            Some(m) => f(m, size),
            None => f(&atelier_core::Mask::new(), size),
        };
        let arc = new.filter(|m| !m.is_empty()).map(std::sync::Arc::new);
        if arc.is_none() && cur.is_none() {
            return;
        }
        let cmd = atelier_core::command::SetSelection::new(&st.editor.doc, arc, label);
        st.editor.apply(Box::new(cmd));
    }

    /// Insert a non-destructive adjustment layer above the selection.
    fn add_adjustment_layer(&mut self, adj: atelier_raster::Adjustment) {
        use atelier_core::{LayerProps, Node, NodeKind};
        let Some(st) = &mut self.state else { return };
        let doc = &st.editor.doc;
        let (parent, index) = match st.editor.selection.and_then(|s| doc.node(s).map(|n| (s, n))) {
            Some((sel, n)) => {
                let parent = n.parent.unwrap_or(doc.root());
                let index =
                    doc.children(parent).iter().position(|&c| c == sel).unwrap_or(0);
                (parent, index)
            }
            None => (doc.root(), 0),
        };
        let node = Node::new(LayerProps::named(adj.label()), NodeKind::Adjustment(adj));
        let cmd = atelier_core::command::AddNode::new(&mut st.editor.doc, node, parent, index);
        let id = cmd.id;
        st.editor.apply(Box::new(cmd));
        st.editor.selection = Some(id);
    }

    /// Insert a filled vector shape layer from a doc-space bounding box
    /// (spec 0014). `ellipse` chooses ellipse vs rectangle.
    fn add_shape_layer(&mut self, ellipse: bool, min: [f32; 2], max: [f32; 2]) {
        use atelier_core::atelier_vector::{Path, Shape};
        use atelier_core::{LayerProps, Node, NodeKind, VectorContent};
        let Some(st) = &mut self.state else { return };
        let (w, h) = (max[0] - min[0], max[1] - min[1]);
        if w < 1.0 || h < 1.0 {
            return;
        }
        let path = if ellipse {
            Path::ellipse(min[0] + w * 0.5, min[1] + h * 0.5, w * 0.5, h * 0.5)
        } else {
            Path::rect(min[0], min[1], w, h)
        };
        let content =
            VectorContent { shapes: vec![Shape::filled(path, st.brush.vector_fill)] };
        let name = if ellipse { "Ellipse" } else { "Rectangle" };

        let doc = &st.editor.doc;
        let (parent, index) = match st.editor.selection.and_then(|s| doc.node(s).map(|n| (s, n))) {
            Some((sel, n)) => {
                let parent = n.parent.unwrap_or(doc.root());
                let index = doc.children(parent).iter().position(|&c| c == sel).unwrap_or(0);
                (parent, index)
            }
            None => (doc.root(), 0),
        };
        let cmd = atelier_core::command::AddNode::new(
            &mut st.editor.doc,
            Node::new(LayerProps::named(name), NodeKind::Vector(content)),
            parent,
            index,
        );
        let id = cmd.id;
        st.editor.apply(Box::new(cmd));
        st.editor.selection = Some(id);
    }

    /// Apply a destructive adjustment to the selected raster layer, within the
    /// active selection (whole layer if none). One undoable PaintTiles entry.
    fn apply_adjustment(&mut self, adj: atelier_raster::Adjustment) {
        use atelier_core::NodeKind;
        let Some(st) = &mut self.state else { return };
        let Some(id) = st.editor.selection else { return };
        // Target must be a visible, unlocked raster layer.
        let offset = match st.editor.doc.node(id).map(|n| (&n.kind, &n.props)) {
            Some((NodeKind::Raster(c), props)) if props.visible && !props.locked => c.offset,
            _ => return,
        };
        let mask = st.editor.doc.selection.clone();
        let bounds = mask.as_deref().and_then(|m| m.bounds());

        let coords = {
            let NodeKind::Raster(c) = &st.editor.doc.node(id).expect("checked").kind else {
                return;
            };
            atelier_raster::target_tiles(&c.tiles, bounds, offset)
        };
        if coords.is_empty() {
            return;
        }

        // Capture before, mutate clones, reinsert.
        let mut before = Vec::with_capacity(coords.len());
        {
            let NodeKind::Raster(c) = &mut st.editor.doc.node_mut(id).expect("checked").kind
            else {
                return;
            };
            for (tx, ty) in coords {
                let original = c.tiles.tile_at((tx, ty)).cloned();
                before.push(((tx, ty), original.clone()));
                if let Some(mut tile) = original {
                    atelier_raster::apply_tile(
                        &mut tile,
                        adj,
                        tx,
                        ty,
                        offset,
                        mask.as_deref(),
                    );
                    c.tiles.insert_tile((tx, ty), tile);
                }
            }
        }
        let cmd = atelier_core::command::PaintTiles::from_capture(
            &st.editor.doc,
            id,
            adj.label(),
            before,
        );
        st.editor.history.push_committed(Box::new(cmd));
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        use egui::{Key, KeyboardShortcut, Modifiers};
        const CMD: Modifiers = Modifiers::COMMAND;
        let redo_shift = KeyboardShortcut::new(CMD.plus(Modifiers::SHIFT), Key::Z);
        let redo_y = KeyboardShortcut::new(CMD, Key::Y);
        let undo = KeyboardShortcut::new(CMD, Key::Z);
        let save_as = KeyboardShortcut::new(CMD.plus(Modifiers::SHIFT), Key::S);
        let save = KeyboardShortcut::new(CMD, Key::S);
        let new = KeyboardShortcut::new(CMD, Key::N);
        let open = KeyboardShortcut::new(CMD, Key::O);

        if ctx.input_mut(|i| i.consume_shortcut(&redo_shift) || i.consume_shortcut(&redo_y)) {
            if let Some(st) = &mut self.state {
                st.editor.history.redo(&mut st.editor.doc);
            }
        } else if ctx.input_mut(|i| i.consume_shortcut(&undo)) {
            if let Some(st) = &mut self.state {
                st.editor.history.undo(&mut st.editor.doc);
            }
        }
        if ctx.input_mut(|i| i.consume_shortcut(&save_as)) {
            self.save(true);
        } else if ctx.input_mut(|i| i.consume_shortcut(&save)) {
            self.save(false);
        }
        if ctx.input_mut(|i| i.consume_shortcut(&new)) {
            self.request_new();
        }
        if ctx.input_mut(|i| i.consume_shortcut(&open)) {
            self.request_open();
        }

        // Selection/adjust shortcuts must yield to focused text fields (e.g.
        // Ctrl+A select-all-text during a layer rename).
        let editing_text = ctx.wants_keyboard_input();

        // Invert image (Ctrl+I) vs invert selection (Ctrl+Shift+I).
        let invert_sel = KeyboardShortcut::new(CMD.plus(Modifiers::SHIFT), Key::I);
        let invert = KeyboardShortcut::new(CMD, Key::I);
        let select_all = KeyboardShortcut::new(CMD, Key::A);
        if !editing_text {
            if ctx.input_mut(|i| i.consume_shortcut(&invert_sel)) {
                self.set_selection("Invert Selection", |m, size| Some(m.inverted(size)));
            } else if ctx.input_mut(|i| i.consume_shortcut(&invert)) {
                self.apply_adjustment(atelier_raster::Adjustment::Invert);
            }
            if ctx.input_mut(|i| i.consume_shortcut(&select_all)) {
                self.set_selection("Select All", |_, size| {
                    Some(atelier_core::Mask::select_all(size))
                });
            }
        }

        // Deselect (Ctrl+D).
        let deselect = KeyboardShortcut::new(CMD, Key::D);
        if !editing_text && ctx.input_mut(|i| i.consume_shortcut(&deselect)) {
            if let Some(st) = &mut self.state {
                if st.editor.doc.selection.is_some() {
                    let cmd = atelier_core::command::SetSelection::new(
                        &st.editor.doc,
                        None,
                        "Deselect",
                    );
                    st.editor.apply(Box::new(cmd));
                }
            }
        }

        // Tool keys (plain letters — only when no text field wants them).
        if !ctx.wants_keyboard_input() {
            if let Some(st) = &mut self.state {
                ctx.input(|i| {
                    if i.key_pressed(Key::V) {
                        st.tool = ActiveTool::Move;
                    }
                    if i.key_pressed(Key::B) {
                        st.tool = ActiveTool::Brush;
                    }
                    if i.key_pressed(Key::E) {
                        st.tool = ActiveTool::Eraser;
                    }
                    if i.key_pressed(Key::M) {
                        st.tool = ActiveTool::SelectRect;
                    }
                    if i.key_pressed(Key::L) {
                        st.tool = ActiveTool::Lasso;
                    }
                    if i.key_pressed(Key::U) {
                        st.tool = ActiveTool::ShapeRect;
                    }
                    if i.key_pressed(Key::W) {
                        st.tool = ActiveTool::MagicWand;
                    }
                });
            }
        }
    }

    fn menu_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New…\t(Ctrl+N)").clicked() {
                        self.request_new();
                        ui.close_menu();
                    }
                    if ui.button("Open…\t(Ctrl+O)").clicked() {
                        self.request_open();
                        ui.close_menu();
                    }
                    ui.separator();
                    let has_doc = self.state.is_some();
                    if ui.add_enabled(has_doc, egui::Button::new("Save\t(Ctrl+S)")).clicked() {
                        self.save(false);
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(has_doc, egui::Button::new("Save As…\t(Ctrl+Shift+S)"))
                        .clicked()
                    {
                        self.save(true);
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Exit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button("Image", |ui| {
                    let size = self.state.as_ref().map(|s| s.editor.doc.size);
                    if ui
                        .add_enabled(size.is_some(), egui::Button::new("Canvas Size…"))
                        .clicked()
                    {
                        self.canvas_size = size;
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(size.is_some(), egui::Button::new("Image Size…"))
                        .clicked()
                    {
                        self.image_size_dialog = size;
                        ui.close_menu();
                    }
                    let has_sel = self
                        .state
                        .as_ref()
                        .is_some_and(|s| s.editor.doc.selection.is_some());
                    if ui
                        .add_enabled(has_sel, egui::Button::new("Crop to Selection"))
                        .clicked()
                    {
                        self.crop_to_selection();
                        ui.close_menu();
                    }
                });
                ui.menu_button("Layer", |ui| {
                    use atelier_raster::Adjustment;
                    let has = self.state.is_some();
                    if ui.add_enabled(has, egui::Button::new("Transform…")).clicked() {
                        self.transform_dialog = Some([100.0, 100.0, 0.0]);
                        ui.close_menu();
                    }
                    ui.separator();
                    ui.menu_button("New Adjustment Layer", |ui| {
                        let opts = [
                            ("Invert", Adjustment::Invert),
                            (
                                "Brightness/Contrast",
                                Adjustment::BrightnessContrast { brightness: 0.0, contrast: 0.0 },
                            ),
                            ("Levels", Adjustment::Levels { black: 0.0, white: 1.0, gamma: 1.0 }),
                            (
                                "Hue/Saturation",
                                Adjustment::HueSaturation { hue: 0.0, sat: 0.0, light: 0.0 },
                            ),
                        ];
                        for (name, adj) in opts {
                            if ui.add_enabled(has, egui::Button::new(name)).clicked() {
                                self.add_adjustment_layer(adj);
                                ui.close_menu();
                            }
                        }
                    });
                });
                ui.menu_button("Select", |ui| {
                    let has = self.state.is_some();
                    let has_sel =
                        self.state.as_ref().is_some_and(|s| s.editor.doc.selection.is_some());
                    if ui.add_enabled(has, egui::Button::new("All\t(Ctrl+A)")).clicked() {
                        self.set_selection("Select All", |_, size| {
                            Some(atelier_core::Mask::select_all(size))
                        });
                        ui.close_menu();
                    }
                    if ui.add_enabled(has_sel, egui::Button::new("Deselect\t(Ctrl+D)")).clicked() {
                        self.set_selection("Deselect", |_, _| None);
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(has_sel, egui::Button::new("Invert\t(Ctrl+Shift+I)"))
                        .clicked()
                    {
                        self.set_selection("Invert Selection", |m, size| Some(m.inverted(size)));
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.add_enabled(has_sel, egui::Button::new("Grow")).clicked() {
                        self.set_selection("Grow Selection", |m, _| {
                            Some(atelier_raster::selection::grow(m, 2))
                        });
                        ui.close_menu();
                    }
                    if ui.add_enabled(has_sel, egui::Button::new("Shrink")).clicked() {
                        self.set_selection("Shrink Selection", |m, _| {
                            Some(atelier_raster::selection::shrink(m, 2))
                        });
                        ui.close_menu();
                    }
                    if ui.add_enabled(has_sel, egui::Button::new("Feather")).clicked() {
                        self.set_selection("Feather Selection", |m, _| {
                            Some(atelier_raster::selection::feather(m, 3))
                        });
                        ui.close_menu();
                    }
                });
                ui.menu_button("Adjust", |ui| {
                    use atelier_raster::Adjustment;
                    let has = self.state.is_some();
                    if ui.add_enabled(has, egui::Button::new("Invert\t(Ctrl+I)")).clicked() {
                        self.apply_adjustment(Adjustment::Invert);
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.add_enabled(has, egui::Button::new("Brightness/Contrast…")).clicked() {
                        self.adjust_dialog =
                            Some(Adjustment::BrightnessContrast { brightness: 0.0, contrast: 0.0 });
                        ui.close_menu();
                    }
                    if ui.add_enabled(has, egui::Button::new("Levels…")).clicked() {
                        self.adjust_dialog =
                            Some(Adjustment::Levels { black: 0.0, white: 1.0, gamma: 1.0 });
                        ui.close_menu();
                    }
                    if ui.add_enabled(has, egui::Button::new("Hue/Saturation…")).clicked() {
                        self.adjust_dialog =
                            Some(Adjustment::HueSaturation { hue: 0.0, sat: 0.0, light: 0.0 });
                        ui.close_menu();
                    }
                });
                ui.menu_button("Edit", |ui| {
                    let (can_undo, can_redo) = self
                        .state
                        .as_ref()
                        .map(|s| (s.editor.history.can_undo(), s.editor.history.can_redo()))
                        .unwrap_or((false, false));
                    if ui.add_enabled(can_undo, egui::Button::new("Undo\t(Ctrl+Z)")).clicked() {
                        if let Some(st) = &mut self.state {
                            st.editor.history.undo(&mut st.editor.doc);
                        }
                        ui.close_menu();
                    }
                    if ui.add_enabled(can_redo, egui::Button::new("Redo\t(Ctrl+Shift+Z)")).clicked()
                    {
                        if let Some(st) = &mut self.state {
                            st.editor.history.redo(&mut st.editor.doc);
                        }
                        ui.close_menu();
                    }
                });
            });
        });
    }

    fn modal_windows(&mut self, ctx: &egui::Context) {
        // New-document dialog.
        let mut create = false;
        let mut cancel = false;
        if let Some(dlg) = &mut self.new_doc {
            egui::Window::new("New Document")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    egui::Grid::new("new_doc_grid").num_columns(2).show(ui, |ui| {
                        ui.label("Width");
                        ui.add(egui::DragValue::new(&mut dlg.width).range(1..=32768).suffix(" px"));
                        ui.end_row();
                        ui.label("Height");
                        ui.add(
                            egui::DragValue::new(&mut dlg.height).range(1..=32768).suffix(" px"),
                        );
                        ui.end_row();
                    });
                    ui.label("Project focus (workspace preset; both layer kinds always work):");
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut dlg.focus, ProjectFocus::Raster, "Raster (photo)");
                        ui.radio_value(&mut dlg.focus, ProjectFocus::Vector, "Vector (illustration)");
                    });
                    ui.separator();
                    ui.horizontal(|ui| {
                        create = ui.button("Create").clicked();
                        cancel = ui.button("Cancel").clicked();
                    });
                });
        }
        if create {
            let dlg = self.new_doc.take().expect("dialog open");
            self.state = Some(EditorState {
                editor: Editor::new([dlg.width, dlg.height], dlg.focus),
                path: None,
                rename: None,
                composite: None,
                tool: ActiveTool::Move,
                brush: BrushSettings::default(),
                stroke: None,
                    dirty_patch: None,
                    select_drag: None,
                    ants: None,
                    wand_click: None,
                    vector_cache: None,
                    pending_shape: None,
            });
            self.viewport = Viewport::default();
        } else if cancel {
            self.new_doc = None;
        }

        // Canvas Size dialog.
        let mut resize = false;
        let mut resize_cancel = false;
        if let Some(size) = &mut self.canvas_size {
            egui::Window::new("Canvas Size")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Width");
                        ui.add(egui::DragValue::new(&mut size[0]).range(1..=32768).suffix(" px"));
                        ui.label("Height");
                        ui.add(egui::DragValue::new(&mut size[1]).range(1..=32768).suffix(" px"));
                    });
                    ui.horizontal(|ui| {
                        resize = ui.button("Resize").clicked();
                        resize_cancel = ui.button("Cancel").clicked();
                    });
                });
        }
        if resize {
            let size = self.canvas_size.take().expect("dialog open");
            if let Some(st) = &mut self.state {
                let cmd = atelier_core::command::CanvasResize::new(&st.editor.doc, size);
                st.editor.apply(Box::new(cmd));
            }
        } else if resize_cancel {
            self.canvas_size = None;
        }

        // Transform dialog (numeric scale/rotate).
        let mut transform_apply: Option<[f32; 3]> = None;
        let mut transform_cancel = false;
        if let Some(t) = &mut self.transform_dialog {
            egui::Window::new("Transform Layer")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.add(egui::Slider::new(&mut t[0], 1.0..=400.0).text("Scale X %"));
                    ui.add(egui::Slider::new(&mut t[1], 1.0..=400.0).text("Scale Y %"));
                    ui.add(egui::Slider::new(&mut t[2], -180.0..=180.0).text("Rotate °"));
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("Apply").clicked() {
                            transform_apply = Some(*t);
                        }
                        if ui.button("Cancel").clicked() {
                            transform_cancel = true;
                        }
                    });
                });
        }
        if let Some(t) = transform_apply {
            self.transform_dialog = None;
            self.apply_transform(t[0], t[1], t[2]);
        } else if transform_cancel {
            self.transform_dialog = None;
        }

        // Image Size dialog (resample).
        let mut resample_apply: Option<[u32; 2]> = None;
        let mut resample_cancel = false;
        if let Some(sz) = &mut self.image_size_dialog {
            egui::Window::new("Image Size")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Width");
                        ui.add(egui::DragValue::new(&mut sz[0]).range(1..=32768).suffix(" px"));
                        ui.label("Height");
                        ui.add(egui::DragValue::new(&mut sz[1]).range(1..=32768).suffix(" px"));
                    });
                    ui.horizontal(|ui| {
                        resample_apply = ui.button("Resample").clicked().then_some(*sz);
                        resample_cancel = ui.button("Cancel").clicked();
                    });
                });
        }
        if let Some(sz) = resample_apply {
            self.image_size_dialog = None;
            self.apply_resample(sz);
        } else if resample_cancel {
            self.image_size_dialog = None;
        }

        // Adjustment dialog (parametric).
        let mut adjust_apply: Option<atelier_raster::Adjustment> = None;
        let mut adjust_cancel = false;
        if let Some(adj) = &mut self.adjust_dialog {
            use atelier_raster::Adjustment;
            egui::Window::new(adj.label())
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    match adj {
                        Adjustment::BrightnessContrast { brightness, contrast } => {
                            ui.add(egui::Slider::new(brightness, -1.0..=1.0).text("Brightness"));
                            ui.add(egui::Slider::new(contrast, -1.0..=1.0).text("Contrast"));
                        }
                        Adjustment::Levels { black, white, gamma } => {
                            ui.add(egui::Slider::new(black, 0.0..=1.0).text("Black point"));
                            ui.add(egui::Slider::new(white, 0.0..=1.0).text("White point"));
                            ui.add(egui::Slider::new(gamma, 0.1..=5.0).text("Gamma"));
                        }
                        Adjustment::HueSaturation { hue, sat, light } => {
                            ui.add(egui::Slider::new(hue, -180.0..=180.0).text("Hue"));
                            ui.add(egui::Slider::new(sat, -1.0..=1.0).text("Saturation"));
                            ui.add(egui::Slider::new(light, -1.0..=1.0).text("Lightness"));
                        }
                        Adjustment::Invert => {}
                    }
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("Apply").clicked() {
                            adjust_apply = Some(*adj);
                        }
                        if ui.button("Cancel").clicked() {
                            adjust_cancel = true;
                        }
                    });
                });
        }
        if let Some(adj) = adjust_apply {
            self.adjust_dialog = None;
            self.apply_adjustment(adj);
        } else if adjust_cancel {
            self.adjust_dialog = None;
        }

        // Unsaved-changes confirmation.
        let mut decided: Option<bool> = None;
        if self.pending.is_some() {
            egui::Window::new("Unsaved changes")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label("The current document has unsaved changes. Discard them?");
                    ui.horizontal(|ui| {
                        if ui.button("Discard").clicked() {
                            decided = Some(true);
                        }
                        if ui.button("Cancel").clicked() {
                            decided = Some(false);
                        }
                    });
                });
        }
        if let Some(discard) = decided {
            let action = self.pending.take().expect("pending set");
            if discard {
                match action {
                    PendingAction::New => self.new_doc = Some(NewDocDialog::default()),
                    PendingAction::Open => self.do_open(),
                }
            }
        }

        // Error popup.
        let mut dismiss = false;
        if let Some(msg) = &self.error {
            egui::Window::new("Error")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(msg);
                    dismiss = ui.button("OK").clicked();
                });
        }
        if dismiss {
            self.error = None;
        }
    }

    fn sync_title(&mut self, ctx: &egui::Context) {
        let title = match &self.state {
            Some(st) => {
                let name = st
                    .path
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "Untitled".into());
                let dirty = if st.editor.is_dirty() { "*" } else { "" };
                format!("Atelier — {name}{dirty}")
            }
            None => "Atelier".into(),
        };
        if title != self.last_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.last_title = title;
        }
    }
}

impl eframe::App for AtelierApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ui(ctx);
    }
}

impl AtelierApp {
    /// Full per-frame UI, independent of eframe (drivable by kittest).
    fn ui(&mut self, ctx: &egui::Context) {
        // Drop selections orphaned by undo/redo before any panel reads them.
        if let Some(st) = &mut self.state {
            if st.editor.selection.is_some_and(|id| st.editor.doc.node(id).is_none()) {
                st.editor.selection = None;
            }
            if st.rename.as_ref().is_some_and(|(id, _)| st.editor.doc.node(*id).is_none()) {
                st.rename = None;
            }
        }

        self.handle_shortcuts(ctx);
        self.menu_bar(ctx);

        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.adapter_info);
                ui.separator();
                ui.label(format!("{:.0}%", self.viewport.zoom * 100.0));
                if let Some(st) = &self.state {
                    ui.separator();
                    ui.label(format!("{} × {} px", st.editor.doc.size[0], st.editor.doc.size[1]));
                }
            });
        });

        self.modal_windows(ctx);

        let mut tabs = TabContents { viewport: &mut self.viewport, state: &mut self.state };
        DockArea::new(&mut self.dock)
            .style(egui_dock::Style::from_egui(ctx.style().as_ref()))
            .show(ctx, &mut tabs);

        // Drain a queued magic-wand click (canvas can't call the app helper).
        if let Some((doc, shift, alt)) = self.state.as_mut().and_then(|s| s.wand_click.take()) {
            self.magic_wand_at(doc, shift, alt);
        }
        // Drain a queued shape-tool drag into a new vector layer (spec 0014).
        if let Some((ellipse, min, max)) = self.state.as_mut().and_then(|s| s.pending_shape.take())
        {
            self.add_shape_layer(ellipse, min, max);
        }

        self.sync_title(ctx);
    }
}

struct TabContents<'a> {
    viewport: &'a mut Viewport,
    state: &'a mut Option<EditorState>,
}

impl egui_dock::TabViewer for TabContents<'_> {
    type Tab = Tab;

    fn title(&mut self, tab: &mut Tab) -> egui::WidgetText {
        match tab {
            Tab::Canvas => "Canvas".into(),
            Tab::Tools => "Tools".into(),
            Tab::Layers => "Layers".into(),
            Tab::Properties => "Properties".into(),
            Tab::History => "History".into(),
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Tab) {
        match tab {
            Tab::Canvas => {
                canvas::canvas_ui(ui, self.viewport, self.state.as_mut());
            }
            Tab::Tools => match self.state {
                Some(st) => panels::tools_ui(ui, st),
                None => {
                    ui.weak("No document");
                }
            },
            Tab::Layers => match self.state {
                Some(st) => panels::layers_ui(ui, st),
                None => {
                    ui.weak("No document. File → New… (Ctrl+N)");
                }
            },
            Tab::Properties => match self.state {
                Some(st) => panels::properties_ui(ui, st),
                None => {
                    ui.weak("No document");
                }
            },
            Tab::History => match self.state {
                Some(st) => panels::history_ui(ui, st),
                None => {
                    ui.weak("No document");
                }
            },
        }
    }
}

/// Headless UI walkthrough of the spec 0001/0002 checklists, driving the real
/// widget tree via egui_kittest (no OS input, no GPU, no file dialogs).
#[cfg(test)]
mod ui_tests {
    use super::*;
    use atelier_core::{BlendMode, NodeKind};
    use egui_kittest::kittest::Queryable;
    use egui_kittest::Harness;

    fn harness() -> Harness<'static, AtelierApp> {
        let app = AtelierApp::with_adapter_info("test-adapter".into());
        let mut h = Harness::builder()
            .with_size(egui::vec2(1400.0, 900.0))
            .build_state(|ctx, app: &mut AtelierApp| app.ui(ctx), app);
        h.run();
        h
    }

    /// Single chokepoint for raw input events (easy to adapt if the API moves).
    fn send(h: &mut Harness<'static, AtelierApp>, event: egui::Event) {
        h.input_mut().events.push(event);
        h.run();
    }

    fn send_key(h: &mut Harness<'static, AtelierApp>, key: egui::Key, modifiers: egui::Modifiers) {
        // Shortcuts read modifiers from the event; canvas/zoom paths read the
        // sticky `RawInput::modifiers` state — set both.
        h.input_mut().modifiers = modifiers;
        send(
            h,
            egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers },
        );
        send(
            h,
            egui::Event::Key { key, physical_key: None, pressed: false, repeat: false, modifiers },
        );
        h.input_mut().modifiers = egui::Modifiers::NONE;
    }

    fn click_label(h: &mut Harness<'static, AtelierApp>, label: &str) {
        h.get_by_label(label).click();
        h.run();
    }

    /// Click through New Document dialog → Create. Small canvas so the
    /// per-frame recomposite stays cheap in tests.
    fn create_doc(h: &mut Harness<'static, AtelierApp>) {
        h.state_mut().new_doc =
            Some(NewDocDialog { width: 64, height: 64, focus: ProjectFocus::Raster });
        h.run();
        click_label(h, "Create");
        assert!(h.state().state.is_some(), "document created via dialog");
    }

    fn doc_labels(h: &Harness<'static, AtelierApp>) -> Vec<(String, usize)> {
        let st = h.state().state.as_ref().expect("doc open");
        st.editor
            .doc
            .iter_tree()
            .iter()
            .map(|&(id, depth)| (st.editor.doc.node(id).unwrap().props.name.clone(), depth))
            .collect()
    }

    #[test]
    fn full_layer_walkthrough_with_undo_redo_and_history_jump() {
        let mut h = harness();
        create_doc(&mut h);

        // Add two layers and a group via the Layers panel buttons.
        click_label(&mut h, "+ Layer");
        click_label(&mut h, "+ Layer");
        click_label(&mut h, "+ Group");
        assert_eq!(
            doc_labels(&h),
            vec![
                ("Group 3".into(), 0),
                ("Layer 2".into(), 0),
                ("Layer 1".into(), 0)
            ]
        );

        // Select "Layer 2" and nest it into the group above (row label click).
        click_label(&mut h, "Layer 2");
        click_label(&mut h, "Into Group");
        assert_eq!(doc_labels(&h)[1], ("Layer 2".into(), 1), "nested under group");

        // Back out, then reorder below "Layer 1".
        click_label(&mut h, "Out");
        assert_eq!(doc_labels(&h)[1], ("Layer 2".into(), 0));
        click_label(&mut h, "Down");
        assert_eq!(
            doc_labels(&h),
            vec![
                ("Group 3".into(), 0),
                ("Layer 1".into(), 0),
                ("Layer 2".into(), 0)
            ]
        );

        // Undo once via the keyboard path (Ctrl+Z handled by the app shell).
        send_key(&mut h, egui::Key::Z, egui::Modifiers::COMMAND);
        assert_eq!(
            doc_labels(&h)[1],
            ("Layer 2".into(), 0),
            "reorder undone, Layer 2 back above Layer 1"
        );
        assert_eq!(doc_labels(&h)[2], ("Layer 1".into(), 0));

        // History panel: activate its dock tab (egui_dock titles aren't
        // accesskit nodes), then jump to document-open state via UI click.
        let loc = h.state_mut().dock.find_tab(&Tab::History).expect("History tab exists");
        h.state_mut().dock.set_active_tab(loc);
        h.run();
        click_label(&mut h, "(document opened)");
        assert!(doc_labels(&h).is_empty(), "history jump undid everything");

        // Jump all the way forward again (API; redo labels are duplicated in UI).
        {
            let st = h.state_mut().state.as_mut().unwrap();
            st.editor.history.jump_to(&mut st.editor.doc, 6);
        }
        h.run();
        assert_eq!(doc_labels(&h).len(), 3, "redo restored all edits");
    }

    #[test]
    fn rename_via_double_click_and_typing() {
        let mut h = harness();
        create_doc(&mut h);
        click_label(&mut h, "+ Layer");

        // Double-click with real pointer events at the row label's center
        // (accesskit click actions don't produce double-click timing).
        let node_id = h.state().state.as_ref().unwrap().editor.selection.unwrap();
        let bb = h.get_by_label("Layer 1").bounding_box().expect("row label has bounds");
        let pos = egui::pos2(
            (bb.x0 + bb.x1) as f32 / 2.0,
            (bb.y0 + bb.y1) as f32 / 2.0,
        );
        for pressed in [true, false, true, false] {
            h.input_mut().events.push(egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            });
        }
        h.run();
        h.run();
        assert!(
            h.state().state.as_ref().unwrap().rename.is_some(),
            "double-click entered rename mode"
        );

        // Select-all, type replacement, commit with Enter — the TextEdit path.
        send_key(&mut h, egui::Key::A, egui::Modifiers::COMMAND);
        send(&mut h, egui::Event::Text("Background".into()));
        send_key(&mut h, egui::Key::Enter, egui::Modifiers::NONE);

        let st = h.state().state.as_ref().unwrap();
        assert_eq!(st.editor.doc.node(node_id).unwrap().props.name, "Background");
        assert!(st.editor.history.can_undo(), "rename recorded as a command");
    }

    #[test]
    fn blend_mode_change_via_combo_is_undoable() {
        let mut h = harness();
        create_doc(&mut h);
        click_label(&mut h, "+ Layer");
        let id = h.state().state.as_ref().unwrap().editor.selection.unwrap();

        // The combo's selected text is not an accessible label; open by role.
        h.get_by_role(accesskit::Role::ComboBox).click();
        h.run();
        click_label(&mut h, "Multiply"); // pick mode
        {
            let st = h.state().state.as_ref().unwrap();
            assert_eq!(st.editor.doc.node(id).unwrap().props.blend, BlendMode::Multiply);
        }
        send_key(&mut h, egui::Key::Z, egui::Modifiers::COMMAND);
        let st = h.state().state.as_ref().unwrap();
        assert_eq!(st.editor.doc.node(id).unwrap().props.blend, BlendMode::Normal);
    }

    #[test]
    fn delete_selected_layer_via_ui_then_undo() {
        let mut h = harness();
        create_doc(&mut h);
        click_label(&mut h, "+ Layer");
        assert_eq!(doc_labels(&h).len(), 1);

        click_label(&mut h, "Delete");
        assert_eq!(doc_labels(&h).len(), 0, "layer deleted");
        assert!(h.state().state.as_ref().unwrap().editor.selection.is_none());

        send_key(&mut h, egui::Key::Z, egui::Modifiers::COMMAND);
        assert_eq!(doc_labels(&h).len(), 1, "delete undone");
    }

    #[test]
    fn save_then_reopen_restores_identical_tree() {
        let path =
            std::env::temp_dir().join(format!("atelier-ui-test-{}.atl", std::process::id()));

        let mut h = harness();
        create_doc(&mut h);
        click_label(&mut h, "+ Layer");
        click_label(&mut h, "+ Group");

        let saved_doc = h.state().state.as_ref().unwrap().editor.doc.clone();
        h.state_mut().save_to(path.clone());
        assert!(!h.state().state.as_ref().unwrap().editor.is_dirty(), "saved");

        // Fresh app instance (simulates restart), open the file.
        let mut h2 = harness();
        h2.state_mut().open_from(path.clone());
        h2.run();
        std::fs::remove_file(&path).ok();

        let reopened = &h2.state().state.as_ref().unwrap().editor.doc;
        assert_eq!(*reopened, saved_doc, "tree identical after restart+open");
        assert_eq!(doc_labels(&h2).len(), 2);
    }

    #[test]
    fn canvas_keyboard_zoom_pan_and_ctrl_wheel() {
        let mut h = harness();
        create_doc(&mut h);

        // Hover the canvas center so the canvas response receives input.
        let canvas_center = egui::pos2(550.0, 450.0);
        send(&mut h, egui::Event::PointerMoved(canvas_center));

        let z0 = h.state().viewport.zoom;
        send_key(&mut h, egui::Key::Equals, egui::Modifiers::COMMAND);
        let z1 = h.state().viewport.zoom;
        assert!(z1 > z0, "Ctrl+= zoomed in ({z0} -> {z1})");

        let pan0 = h.state().viewport.pan;
        send_key(&mut h, egui::Key::ArrowRight, egui::Modifiers::NONE);
        let pan1 = h.state().viewport.pan;
        assert_ne!(pan0, pan1, "arrow key panned");

        // Ctrl+wheel zoom (the mouse-wheel path; egui folds it into zoom_delta,
        // which reads the sticky modifier state).
        h.input_mut().modifiers = egui::Modifiers::COMMAND;
        send(
            &mut h,
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                delta: egui::vec2(0.0, 2.0),
                modifiers: egui::Modifiers::COMMAND,
            },
        );
        h.input_mut().modifiers = egui::Modifiers::NONE;
        let z2 = h.state().viewport.zoom;
        assert!(z2 > z1, "ctrl+wheel zoomed in ({z1} -> {z2})");

        // Ctrl+0 resets zoom to 100%.
        send_key(&mut h, egui::Key::Num0, egui::Modifiers::COMMAND);
        assert!((h.state().viewport.zoom - 1.0).abs() < 1e-4, "Ctrl+0 reset");
    }

    /// Raw pointer drag on the canvas (press → move → release across frames).
    fn pointer_drag(h: &mut Harness<'static, AtelierApp>, from: egui::Pos2, to: egui::Pos2) {
        h.input_mut().events.push(egui::Event::PointerMoved(from));
        h.input_mut().events.push(egui::Event::PointerButton {
            pos: from,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        });
        h.run();
        h.input_mut().events.push(egui::Event::PointerMoved(to));
        h.run();
        h.input_mut().events.push(egui::Event::PointerButton {
            pos: to,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        });
        h.run();
        h.run();
    }

    fn selected_raster<'a>(h: &'a Harness<'static, AtelierApp>) -> &'a atelier_core::RasterContent {
        let st = h.state().state.as_ref().unwrap();
        let id = st.editor.selection.unwrap();
        match &st.editor.doc.node(id).unwrap().kind {
            NodeKind::Raster(c) => c,
            _ => panic!("raster selected"),
        }
    }

    const CANVAS_A: egui::Pos2 = egui::pos2(600.0, 400.0);
    const CANVAS_B: egui::Pos2 = egui::pos2(650.0, 430.0);

    #[test]
    fn brush_paints_then_undo_clears() {
        let mut h = harness();
        create_doc(&mut h);
        click_label(&mut h, "+ Layer");
        // Start from an empty layer so the assert is unambiguous.
        {
            let st = h.state_mut().state.as_mut().unwrap();
            let id = st.editor.selection.unwrap();
            if let NodeKind::Raster(c) = &mut st.editor.doc.node_mut(id).unwrap().kind {
                c.tiles = atelier_core::TileMap::new();
            }
        }
        click_label(&mut h, "Brush (B)");
        let before_len = h.state().state.as_ref().unwrap().editor.history.applied_len();

        pointer_drag(&mut h, CANVAS_A, CANVAS_B);

        let content = selected_raster(&h);
        assert!(!content.tiles.is_empty(), "stroke painted pixels");
        let st = h.state().state.as_ref().unwrap();
        assert_eq!(
            st.editor.history.applied_len(),
            before_len + 1,
            "one history entry per stroke"
        );

        send_key(&mut h, egui::Key::Z, egui::Modifiers::COMMAND);
        let content = selected_raster(&h);
        assert!(content.tiles.is_empty(), "undo removed the stroke");

        send_key(&mut h, egui::Key::Y, egui::Modifiers::COMMAND);
        let content = selected_raster(&h);
        assert!(!content.tiles.is_empty(), "redo restored the stroke");
    }

    #[test]
    fn move_tool_drags_layer_offset_one_undo_step() {
        let mut h = harness();
        create_doc(&mut h);
        click_label(&mut h, "+ Layer");
        click_label(&mut h, "Move (V)");
        assert_eq!(selected_raster(&h).offset, [0, 0]);
        let before_len = h.state().state.as_ref().unwrap().editor.history.applied_len();

        pointer_drag(&mut h, CANVAS_A, CANVAS_B);

        let off = selected_raster(&h).offset;
        assert_ne!(off, [0, 0], "drag moved the layer");
        let st = h.state().state.as_ref().unwrap();
        assert_eq!(st.editor.history.applied_len(), before_len + 1, "drag merged to one step");

        send_key(&mut h, egui::Key::Z, egui::Modifiers::COMMAND);
        assert_eq!(selected_raster(&h).offset, [0, 0], "single undo restores");
    }

    #[test]
    fn eraser_stroke_reduces_alpha_via_ui() {
        let mut h = harness();
        create_doc(&mut h);
        click_label(&mut h, "+ Layer");
        click_label(&mut h, "Brush (B)");
        pointer_drag(&mut h, CANVAS_A, CANVAS_B);
        // Find a painted pixel to compare after erasing the same path.
        let painted: Vec<_> = {
            let c = selected_raster(&h);
            c.tiles.tiles().map(|(coord, _)| *coord).collect()
        };
        assert!(!painted.is_empty());

        click_label(&mut h, "Eraser (E)");
        // Erase repeatedly along the same path.
        for _ in 0..3 {
            pointer_drag(&mut h, CANVAS_A, CANVAS_B);
        }
        let st = h.state().state.as_ref().unwrap();
        let labels: Vec<String> = st.editor.history.undo_labels().collect();
        assert!(labels.iter().any(|l| l == "Eraser Stroke"), "eraser recorded: {labels:?}");
    }

    /// Spec 0006: live strokes patch the texture region instead of bumping the
    /// revision every frame; the commit is the single revision bump.
    #[test]
    fn live_stroke_patches_without_revision_churn() {
        let mut h = harness();
        create_doc(&mut h);
        click_label(&mut h, "+ Layer");
        click_label(&mut h, "Brush (B)");
        let rev_before = h.state().state.as_ref().unwrap().editor.history.revision();

        // Press and move WITHOUT releasing: live stroke in progress.
        h.input_mut().events.push(egui::Event::PointerMoved(CANVAS_A));
        h.input_mut().events.push(egui::Event::PointerButton {
            pos: CANVAS_A,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        });
        h.run();
        h.input_mut().events.push(egui::Event::PointerMoved(CANVAS_B));
        h.run();
        {
            let st = h.state().state.as_ref().unwrap();
            assert!(st.stroke.is_some(), "stroke active");
            assert_eq!(
                st.editor.history.revision(),
                rev_before,
                "no revision churn during live stroke"
            );
            assert!(st.dirty_patch.is_none(), "canvas consumed the patch each frame");
        }
        // Release: one committed entry, one revision bump.
        h.input_mut().events.push(egui::Event::PointerButton {
            pos: CANVAS_B,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        });
        h.run();
        h.run();
        let st = h.state().state.as_ref().unwrap();
        assert_eq!(st.editor.history.revision(), rev_before + 1, "commit bumps once");
        assert!(!selected_raster(&h).tiles.is_empty(), "pixels landed");
    }

    /// Pan/zoom must never recomposite (Phase 2 gate: 60 fps is texture-redraw).
    #[test]
    fn pan_and_zoom_leave_composite_cache_untouched() {
        let mut h = harness();
        create_doc(&mut h);
        click_label(&mut h, "+ Layer");
        h.run();
        let rev = h.state().state.as_ref().unwrap().composite.as_ref().unwrap().0;

        send(&mut h, egui::Event::PointerMoved(CANVAS_A));
        send_key(&mut h, egui::Key::ArrowRight, egui::Modifiers::NONE);
        send_key(&mut h, egui::Key::Equals, egui::Modifiers::COMMAND);
        send_key(&mut h, egui::Key::ArrowDown, egui::Modifiers::NONE);

        let st = h.state().state.as_ref().unwrap();
        assert_ne!(h.state().viewport.zoom, 1.0, "zoom changed");
        assert_eq!(
            st.composite.as_ref().unwrap().0,
            rev,
            "viewport changes never recomposite"
        );
    }

    #[test]
    fn magic_wand_select_all_and_invert() {
        let mut h = harness();
        create_doc(&mut h); // 64×64
        click_label(&mut h, "+ Layer");
        let id = h.state().state.as_ref().unwrap().editor.selection.unwrap();
        {
            // Left half red, right half blue.
            let st = h.state_mut().state.as_mut().unwrap();
            if let NodeKind::Raster(c) = &mut st.editor.doc.node_mut(id).unwrap().kind {
                let mut t = atelier_core::TileMap::new();
                t.fill_rect(0, 0, 32, 64, [255, 0, 0, 255]);
                t.fill_rect(32, 0, 64, 64, [0, 0, 255, 255]);
                c.tiles = t;
            }
        }

        // Magic wand on the red half.
        h.state_mut().magic_wand_at([5, 5], false, false);
        h.run();
        {
            let sel = h.state().state.as_ref().unwrap().editor.doc.selection.clone().unwrap();
            assert_eq!(sel.get(5, 5), 255, "red selected");
            assert_eq!(sel.get(50, 5), 0, "blue not selected");
        }
        send_key(&mut h, egui::Key::Z, egui::Modifiers::COMMAND);
        assert!(h.state().state.as_ref().unwrap().editor.doc.selection.is_none(), "wand undone");

        // Select All then Invert.
        send_key(&mut h, egui::Key::A, egui::Modifiers::COMMAND);
        {
            let sel = h.state().state.as_ref().unwrap().editor.doc.selection.clone().unwrap();
            assert_eq!(sel.get(0, 0), 255);
            assert_eq!(sel.get(63, 63), 255);
        }
        h.state_mut().set_selection("Invert Selection", |m, size| Some(m.inverted(size)));
        h.run();
        // Inverting a full selection clears it.
        assert!(
            h.state().state.as_ref().unwrap().editor.doc.selection.is_none(),
            "invert of select-all is empty"
        );
    }

    #[test]
    fn rect_select_combine_and_deselect_via_ui() {
        let mut h = harness();
        create_doc(&mut h);
        click_label(&mut h, "Select Rect (M)");
        let len0 = h.state().state.as_ref().unwrap().editor.history.applied_len();

        pointer_drag(&mut h, CANVAS_A, CANVAS_B);
        {
            let st = h.state().state.as_ref().unwrap();
            assert!(st.editor.doc.selection.is_some(), "marquee created a selection");
            assert_eq!(st.editor.history.applied_len(), len0 + 1, "one undoable step");
        }

        // Shift = add: second marquee, selection persists, second history entry.
        h.input_mut().modifiers = egui::Modifiers::SHIFT;
        pointer_drag(&mut h, egui::pos2(700.0, 300.0), egui::pos2(740.0, 330.0));
        h.input_mut().modifiers = egui::Modifiers::NONE;
        {
            let st = h.state().state.as_ref().unwrap();
            assert!(st.editor.doc.selection.is_some());
            assert_eq!(st.editor.history.applied_len(), len0 + 2);
            let labels: Vec<String> = st.editor.history.undo_labels().collect();
            assert_eq!(
                labels.iter().filter(|l| *l == "Rectangular Select").count(),
                2,
                "{labels:?}"
            );
        }

        // Ctrl+D deselects, undo restores.
        send_key(&mut h, egui::Key::D, egui::Modifiers::COMMAND);
        assert!(h.state().state.as_ref().unwrap().editor.doc.selection.is_none());
        send_key(&mut h, egui::Key::Z, egui::Modifiers::COMMAND);
        assert!(h.state().state.as_ref().unwrap().editor.doc.selection.is_some());
    }

    /// Spec 0014: a shape-tool drag inserts one vector layer; undo removes it.
    #[test]
    fn shape_tool_drag_inserts_vector_layer_and_undoes() {
        for (ellipse, tool) in
            [(false, ActiveTool::ShapeRect), (true, ActiveTool::ShapeEllipse)]
        {
            let mut h = harness();
            create_doc(&mut h);
            h.state_mut().state.as_mut().unwrap().tool = tool;
            h.run();
            let n0 = h.state().state.as_ref().unwrap().editor.doc.node_count();

            pointer_drag(&mut h, CANVAS_A, CANVAS_B);
            h.run();

            let st = h.state().state.as_ref().unwrap();
            assert_eq!(
                st.editor.doc.node_count(),
                n0 + 1,
                "shape drag added one layer (ellipse={ellipse})"
            );
            let id = st.editor.selection.expect("new shape selected");
            match &st.editor.doc.node(id).unwrap().kind {
                NodeKind::Vector(c) => assert_eq!(c.shapes.len(), 1, "one shape"),
                k => panic!("expected vector layer, got {}", k.kind_name()),
            }

            send_key(&mut h, egui::Key::Z, egui::Modifiers::COMMAND);
            assert_eq!(
                h.state().state.as_ref().unwrap().editor.doc.node_count(),
                n0,
                "undo removed the shape layer"
            );
        }
    }

    /// Spec 0008: Invert via menu mutates the selected layer's pixels + undoable.
    /// Spec 0013: a vector layer tessellates into a cached, non-empty mesh that
    /// invalidates on revision change.
    #[test]
    fn vector_layer_tessellates_and_caches() {
        use atelier_core::atelier_vector::{Path, Shape};
        use atelier_core::VectorContent;
        let mut h = harness();
        create_doc(&mut h);
        {
            let st = h.state_mut().state.as_mut().unwrap();
            let root = st.editor.doc.root();
            let content = VectorContent {
                shapes: vec![Shape::filled(Path::rect(4.0, 4.0, 20.0, 20.0), [0.0, 0.7, 0.9, 1.0])],
            };
            let cmd = atelier_core::command::AddNode::new(
                &mut st.editor.doc,
                atelier_core::Node::new(
                    atelier_core::LayerProps::named("vec"),
                    atelier_core::NodeKind::Vector(content),
                ),
                root,
                0,
            );
            st.editor.apply(Box::new(cmd));
        }
        h.run();

        let (rev, layers) = h.state().state.as_ref().unwrap().vector_cache.clone().unwrap();
        assert_eq!(layers.len(), 1, "one vector layer cached");
        assert!(!layers[0].1.is_empty(), "tessellated to triangles");

        click_label(&mut h, "+ Layer");
        h.run();
        let rev2 = h.state().state.as_ref().unwrap().vector_cache.as_ref().unwrap().0;
        assert_ne!(rev, rev2, "cache invalidated on revision change");
    }

    /// Spec 0010: numeric transform bakes the layer; undo restores exactly.
    #[test]
    fn transform_layer_scales_and_undoes() {
        let mut h = harness();
        create_doc(&mut h);
        click_label(&mut h, "+ Layer");
        let id = h.state().state.as_ref().unwrap().editor.selection.unwrap();
        {
            let st = h.state_mut().state.as_mut().unwrap();
            if let NodeKind::Raster(c) = &mut st.editor.doc.node_mut(id).unwrap().kind {
                let mut t = atelier_core::TileMap::new();
                t.fill_rect(0, 0, 20, 20, [255, 0, 0, 255]);
                c.tiles = t;
            }
        }
        let content_w = |h: &Harness<'static, AtelierApp>| -> i32 {
            let st = h.state().state.as_ref().unwrap();
            let NodeKind::Raster(c) = &st.editor.doc.node(id).unwrap().kind else { panic!() };
            let [x0, y0, x1, y1] = c.tiles.bounds().unwrap();
            let (mut lo, mut hi) = (i32::MAX, i32::MIN);
            for y in y0..y1 {
                for x in x0..x1 {
                    if c.tiles.pixel(x, y)[3] > 0 {
                        lo = lo.min(x);
                        hi = hi.max(x);
                    }
                }
            }
            hi - lo + 1
        };
        let before = content_w(&h);
        h.state_mut().apply_transform(200.0, 200.0, 0.0);
        h.run();
        let after = content_w(&h);
        assert!(after > before + 10, "2x scale widened content {before}->{after}");

        send_key(&mut h, egui::Key::Z, egui::Modifiers::COMMAND);
        assert_eq!(content_w(&h), before, "undo restored original tiles");
    }

    /// Crop to selection resizes the doc and shifts offsets; undo restores.
    #[test]
    fn crop_to_selection_resizes_and_undoes() {
        let mut h = harness();
        create_doc(&mut h); // 64×64
        click_label(&mut h, "+ Layer");
        {
            let st = h.state_mut().state.as_mut().unwrap();
            let mut m = atelier_core::Mask::new();
            for y in 10..40 {
                for x in 10..30 {
                    m.set(x, y, 255);
                }
            }
            let cmd = atelier_core::command::SetSelection::new(
                &st.editor.doc,
                Some(std::sync::Arc::new(m)),
                "sel",
            );
            st.editor.apply(Box::new(cmd));
        }
        h.state_mut().crop_to_selection();
        h.run();
        let size = h.state().state.as_ref().unwrap().editor.doc.size;
        assert!(size[0] < 64 && size[0] > 0, "cropped width {}", size[0]);
        // crop + deselect = 2 entries; undo both.
        send_key(&mut h, egui::Key::Z, egui::Modifiers::COMMAND);
        send_key(&mut h, egui::Key::Z, egui::Modifiers::COMMAND);
        assert_eq!(h.state().state.as_ref().unwrap().editor.doc.size, [64, 64], "undo restored size");
    }

    #[test]
    fn adjustment_layer_via_menu_recomposites_and_undoes() {
        let mut h = harness();
        create_doc(&mut h);
        // A raster layer filled with a known color below the adjustment.
        click_label(&mut h, "+ Layer");
        let id = h.state().state.as_ref().unwrap().editor.selection.unwrap();
        {
            let st = h.state_mut().state.as_mut().unwrap();
            if let NodeKind::Raster(c) = &mut st.editor.doc.node_mut(id).unwrap().kind {
                let mut t = atelier_core::TileMap::new();
                t.fill_rect(0, 0, 64, 64, [10, 20, 30, 255]);
                c.tiles = t;
            }
        }
        h.run();
        let composite_px = |h: &mut Harness<'static, AtelierApp>| -> [u8; 4] {
            let doc = &h.state().state.as_ref().unwrap().editor.doc;
            let rgba = atelier_raster::composite_rgba8(doc, 64, 64);
            [rgba[0], rgba[1], rgba[2], rgba[3]]
        };
        assert_eq!(composite_px(&mut h), [10, 20, 30, 255]);

        let n0 = h.state().state.as_ref().unwrap().editor.doc.node_count();
        h.state_mut().add_adjustment_layer(atelier_raster::Adjustment::Invert);
        h.run();
        assert_eq!(
            h.state().state.as_ref().unwrap().editor.doc.node_count(),
            n0 + 1,
            "adjustment layer added"
        );
        assert_eq!(composite_px(&mut h), [245, 235, 225, 255], "composite inverted below");

        send_key(&mut h, egui::Key::Z, egui::Modifiers::COMMAND);
        assert_eq!(composite_px(&mut h), [10, 20, 30, 255], "undo removed adjustment");
    }

    #[test]
    fn invert_adjustment_changes_pixels_and_undoes() {
        let mut h = harness();
        create_doc(&mut h);
        click_label(&mut h, "+ Layer");
        let id = h.state().state.as_ref().unwrap().editor.selection.unwrap();
        {
            let st = h.state_mut().state.as_mut().unwrap();
            if let NodeKind::Raster(c) = &mut st.editor.doc.node_mut(id).unwrap().kind {
                let mut tiles = atelier_core::TileMap::new();
                tiles.fill_rect(0, 0, 64, 64, [10, 20, 30, 255]);
                c.tiles = tiles;
            }
        }
        let sample = |h: &Harness<'static, AtelierApp>| -> [u8; 4] {
            let st = h.state().state.as_ref().unwrap();
            let NodeKind::Raster(c) = &st.editor.doc.node(id).unwrap().kind else {
                panic!()
            };
            c.tiles.pixel(8, 8)
        };
        let before = sample(&h);
        assert_ne!(before[3], 0, "layer has opaque pixels");

        h.state_mut().apply_adjustment(atelier_raster::Adjustment::Invert);
        h.run();
        let after = sample(&h);
        assert_eq!(after[0], 255 - before[0], "red inverted");
        assert_eq!(after[3], before[3], "alpha preserved");

        send_key(&mut h, egui::Key::Z, egui::Modifiers::COMMAND);
        assert_eq!(sample(&h), before, "undo restored pixels");
    }

    /// Adjustment restricted to a selection leaves outside pixels untouched.
    #[test]
    fn adjustment_respects_selection_bounds() {
        let mut h = harness();
        create_doc(&mut h);
        // Fill the whole 64x64 layer with a known opaque color.
        click_label(&mut h, "+ Layer");
        let id = h.state().state.as_ref().unwrap().editor.selection.unwrap();
        {
            let st = h.state_mut().state.as_mut().unwrap();
            if let NodeKind::Raster(c) = &mut st.editor.doc.node_mut(id).unwrap().kind {
                let mut tiles = atelier_core::TileMap::new();
                tiles.fill_rect(0, 0, 64, 64, [10, 20, 30, 255]);
                c.tiles = tiles;
            }
        }
        // Select only the left 20 px column via the model.
        {
            let st = h.state_mut().state.as_mut().unwrap();
            let mut m = atelier_core::Mask::new();
            for y in 0..64 {
                for x in 0..20 {
                    m.set(x, y, 255);
                }
            }
            let cmd = atelier_core::command::SetSelection::new(
                &st.editor.doc,
                Some(std::sync::Arc::new(m)),
                "test sel",
            );
            st.editor.apply(Box::new(cmd));
        }

        h.state_mut().apply_adjustment(atelier_raster::Adjustment::Invert);
        h.run();
        let st = h.state().state.as_ref().unwrap();
        let NodeKind::Raster(c) = &st.editor.doc.node(id).unwrap().kind else { panic!() };
        assert_eq!(c.tiles.pixel(5, 5), [245, 235, 225, 255], "inside selection inverted");
        assert_eq!(c.tiles.pixel(40, 5), [10, 20, 30, 255], "outside selection untouched");
    }

    #[test]
    fn canvas_resize_dialog_applies_and_undoes() {
        let mut h = harness();
        create_doc(&mut h);
        h.state_mut().canvas_size = Some([128, 32]);
        h.run();
        click_label(&mut h, "Resize");
        assert_eq!(h.state().state.as_ref().unwrap().editor.doc.size, [128, 32]);
        send_key(&mut h, egui::Key::Z, egui::Modifiers::COMMAND);
        assert_eq!(h.state().state.as_ref().unwrap().editor.doc.size, [64, 64]);
    }

    #[test]
    fn composite_cache_follows_history_revision() {
        let mut h = harness();
        create_doc(&mut h);
        h.run();
        let rev_of = |h: &Harness<'static, AtelierApp>| {
            h.state().state.as_ref().unwrap().composite.as_ref().map(|(r, _)| *r)
        };
        let rev0 = rev_of(&h);
        assert!(rev0.is_some(), "canvas composited the empty doc");

        click_label(&mut h, "+ Layer");
        h.run();
        let rev1 = rev_of(&h);
        assert_ne!(rev0, rev1, "edit recomposited");

        send_key(&mut h, egui::Key::Z, egui::Modifiers::COMMAND);
        h.run();
        let rev2 = rev_of(&h);
        assert_ne!(rev1, rev2, "undo recomposited");

        h.run();
        assert_eq!(rev2, rev_of(&h), "no edit → cache stable");
    }

    #[test]
    fn unsaved_changes_guard_appears_on_new_when_dirty() {
        let mut h = harness();
        create_doc(&mut h);
        click_label(&mut h, "+ Layer");
        assert!(h.state().state.as_ref().unwrap().editor.is_dirty());

        h.state_mut().request_new();
        h.run();
        assert!(h.state().pending.is_some(), "guard pending");
        click_label(&mut h, "Discard");
        assert!(h.state().pending.is_none());
        assert!(h.state().new_doc.is_some(), "discard proceeds to New dialog");
    }
}

