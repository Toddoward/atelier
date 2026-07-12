//! CPU reference compositor — the blend-mode source of truth (D-9, R-04).
//!
//! Flattens a document layer tree to straight-alpha RGBA. Works in f32;
//! standard compositing (W3C "Compositing and Blending Level 1"):
//!
//! ```text
//! ao = as + ab·(1−as)
//! Co = ( as·(1−ab)·Cs + as·ab·B(Cb,Cs) + (1−as)·ab·Cb ) / ao        (ao>0)
//! ```
//!
//! Groups: non-pass-through groups composite their children into an isolated
//! transparent buffer, then blend that buffer onto the backdrop with the
//! group's mode/opacity. `PassThrough` groups at opacity 1 composite children
//! directly onto the backdrop (Photoshop semantics); `PassThrough` at opacity
//! < 1 falls back to isolated `Normal` (documented simplification, spec 0003).

use crate::blend::{blend_rgb, dissolve_keeps};
use atelier_core::{BlendMode, Document, NodeId, NodeKind};

/// f32 straight-alpha RGBA framebuffer covering a doc-space rect.
struct Buffer {
    w: usize,
    h: usize,
    /// Doc coordinates of pixel (0,0) — region compositing (spec 0006).
    origin: [i32; 2],
    /// `[r, g, b, a]` per pixel, row-major.
    px: Vec<[f32; 4]>,
}

impl Buffer {
    fn transparent(w: usize, h: usize, origin: [i32; 2]) -> Self {
        Self { w, h, origin, px: vec![[0.0; 4]; w * h] }
    }
}

/// Per-pixel source fetch for one layer, in DOC coordinates.
trait Source {
    fn sample(&self, x: i32, y: i32) -> [f32; 4];
}

struct TileSource<'a> {
    tiles: &'a atelier_core::TileMap,
    offset: [i32; 2],
    /// Optional doc-space layer mask multiplying alpha (spec 0047).
    mask: Option<&'a atelier_core::Mask>,
}

impl Source for TileSource<'_> {
    fn sample(&self, x: i32, y: i32) -> [f32; 4] {
        let [r, g, b, a] = self.tiles.pixel(x - self.offset[0], y - self.offset[1]);
        let mut a = a as f32 / 255.0;
        if let Some(m) = self.mask {
            a *= m.get(x, y) as f32 / 255.0;
        }
        [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a]
    }
}

struct BufferSource<'a>(&'a Buffer);

impl Source for BufferSource<'_> {
    fn sample(&self, x: i32, y: i32) -> [f32; 4] {
        let (bx, by) = (x - self.0.origin[0], y - self.0.origin[1]);
        if bx < 0 || by < 0 || bx >= self.0.w as i32 || by >= self.0.h as i32 {
            return [0.0; 4];
        }
        self.0.px[by as usize * self.0.w + bx as usize]
    }
}

/// Samples an origin-`[0,0]` buffer placed at `offset` under an affine inverse
/// (rotate + scale about the embedded document's centre, nearest-neighbour).
/// `inv` is the 2×2 inverse of `M = R(θ)·S`, applied to the pixel centre relative
/// to `center`: embedded `e = center + inv·(p + 0.5 − offset − center)` (spec
/// 0055/0056). Centre pivot keeps the object in place when scaled/rotated.
struct AffineSource<'a> {
    buf: &'a Buffer,
    offset: [i32; 2],
    center: [f32; 2],
    inv: [[f32; 2]; 2],
}

impl AffineSource<'_> {
    /// Build the inverse of `M = R(θ)·S`; `M⁻¹ = [[c/sx, s/sx], [−s/sy, c/sy]]`.
    fn inverse(scale: [f32; 2], rotation: f32) -> Option<[[f32; 2]; 2]> {
        let (sx, sy) = (scale[0], scale[1]);
        if sx <= 0.0 || sy <= 0.0 {
            return None;
        }
        let (c, s) = (rotation.cos(), rotation.sin());
        Some([[c / sx, s / sx], [-s / sy, c / sy]])
    }
}

impl Source for AffineSource<'_> {
    fn sample(&self, x: i32, y: i32) -> [f32; 4] {
        let dx = (x - self.offset[0]) as f32 + 0.5 - self.center[0];
        let dy = (y - self.offset[1]) as f32 + 0.5 - self.center[1];
        let ex = self.inv[0][0] * dx + self.inv[0][1] * dy + self.center[0];
        let ey = self.inv[1][0] * dx + self.inv[1][1] * dy + self.center[1];
        if ex < 0.0 || ey < 0.0 {
            return [0.0; 4];
        }
        let (bx, by) = (ex.floor() as usize, ey.floor() as usize);
        if bx >= self.buf.w || by >= self.buf.h {
            return [0.0; 4];
        }
        self.buf.px[by * self.buf.w + bx]
    }
}

/// Composite one source over `backdrop` with `mode` and `opacity`.
fn blend_onto(backdrop: &mut Buffer, src: &dyn Source, mode: BlendMode, opacity: f32) {
    for y in 0..backdrop.h {
        for x in 0..backdrop.w {
            let i = y * backdrop.w + x;
            // Absolute doc coordinates: sampling and the Dissolve hash must be
            // region-invariant.
            let dx = backdrop.origin[0] + x as i32;
            let dy = backdrop.origin[1] + y as i32;
            let s = src.sample(dx, dy);
            let (mut s_rgb, mut s_a) = ([s[0], s[1], s[2]], s[3] * opacity);
            let mut mode = mode;

            if mode == BlendMode::Dissolve {
                // Dissolve: alpha becomes a per-pixel all-or-nothing gate.
                if s[3] > 0.0 && dissolve_keeps(dx, dy, s_a) {
                    s_a = 1.0;
                } else {
                    s_a = 0.0;
                }
                mode = BlendMode::Normal;
            }
            if s_a <= 0.0 {
                continue;
            }

            let b = backdrop.px[i];
            let (b_rgb, b_a) = ([b[0], b[1], b[2]], b[3]);

            // PassThrough only reaches here via the opacity<1 fallback.
            if mode == BlendMode::PassThrough {
                mode = BlendMode::Normal;
            }
            let blended = if mode == BlendMode::Normal {
                s_rgb
            } else {
                blend_rgb(mode, b_rgb, s_rgb)
            };

            let a_out = s_a + b_a * (1.0 - s_a);
            for c in 0..3 {
                s_rgb[c] = (s_a * (1.0 - b_a) * s_rgb[c]
                    + s_a * b_a * blended[c]
                    + (1.0 - s_a) * b_a * b_rgb[c])
                    / a_out;
            }
            backdrop.px[i] = [s_rgb[0], s_rgb[1], s_rgb[2], a_out];
        }
    }
}

/// Re-tone every pixel of the backdrop in place (adjustment layer). Works in
/// u8 space (canvas is 8-bit) for parity with the destructive path; alpha kept.
fn adjust_backdrop(backdrop: &mut Buffer, adj: atelier_core::Adjustment, opacity: f32) {
    for p in &mut backdrop.px {
        let q = crate::quantize_rgba8;
        let u = [q(p[0]), q(p[1]), q(p[2]), q(p[3])];
        let m = adj.map_pixel_amount(u, opacity);
        p[0] = m[0] as f32 / 255.0;
        p[1] = m[1] as f32 / 255.0;
        p[2] = m[2] as f32 / 255.0;
        // alpha (p[3]) untouched
    }
}

/// Blend one content layer's pixels (Raster/Vector/Smart) onto `backdrop` with
/// the given mode/opacity. Shared by the direct path (layer's own blend) and the
/// isolated clip path (Normal at own opacity) — spec 0057.
fn blend_content(kind: &NodeKind, backdrop: &mut Buffer, mode: BlendMode, opacity: f32) {
    match kind {
        NodeKind::Raster(content) => {
            let src = TileSource {
                tiles: &content.tiles,
                offset: content.offset,
                mask: content.mask.as_ref(),
            };
            blend_onto(backdrop, &src, mode, opacity);
        }
        NodeKind::Vector(content) => {
            // Rasterize the vector layer (doc space, up to the region's far edge)
            // and blend it in tree order so vectors interleave with rasters
            // (spec 0051). The doc-space tiles are sampled/clipped to the region.
            let w = (backdrop.origin[0] + backdrop.w as i32).max(0) as u32;
            let h = (backdrop.origin[1] + backdrop.h as i32).max(0) as u32;
            let vt = crate::rasterize_vector(content, w, h);
            let src = TileSource { tiles: &vt, offset: [0, 0], mask: None };
            blend_onto(backdrop, &src, mode, opacity);
        }
        NodeKind::Smart(content) => {
            // Composite the embedded document at its native resolution into its
            // own buffer, then blend through an affine nearest-neighbour source
            // (centre pivot) placed at `offset` (spec 0052/0055/0056). Recursion
            // via composite_children supports nested groups/clips/vectors/smarts;
            // resampling the embedded source each frame keeps it non-destructive.
            let [dw, dh] = content.doc.size;
            let mut inner = Buffer::transparent(dw as usize, dh as usize, [0, 0]);
            let root = content.doc.root();
            composite_children(&content.doc, root, &mut inner);
            if let Some(inv) = AffineSource::inverse(content.scale, content.rotation) {
                let center = [dw as f32 / 2.0, dh as f32 / 2.0];
                let src = AffineSource { buf: &inner, offset: content.offset, center, inv };
                blend_onto(backdrop, &src, mode, opacity);
            }
        }
        // Group/Adjustment handled by composite_node; text/fill: later phases.
        _ => {}
    }
}

/// Composite one node directly onto the backdrop (no clip handling).
fn composite_node(doc: &Document, id: NodeId, backdrop: &mut Buffer) {
    let Some(node) = doc.node(id) else { return };
    let props = &node.props;
    match &node.kind {
        NodeKind::Adjustment(adj) => {
            adjust_backdrop(backdrop, *adj, props.opacity);
        }
        NodeKind::Group { .. } => {
            if props.blend == BlendMode::PassThrough && props.opacity >= 1.0 {
                composite_children(doc, id, backdrop);
            } else {
                let mut isolated = Buffer::transparent(backdrop.w, backdrop.h, backdrop.origin);
                composite_children(doc, id, &mut isolated);
                blend_onto(backdrop, &BufferSource(&isolated), props.blend, props.opacity);
            }
        }
        kind => blend_content(kind, backdrop, props.blend, props.opacity),
    }
}

/// Render a content layer (Raster/Vector/Smart) into its own transparent buffer
/// (own opacity, Normal) — the isolated form the clip path composites from.
fn render_layer_isolated(doc: &Document, id: NodeId, w: usize, h: usize, origin: [i32; 2]) -> Buffer {
    let mut buf = Buffer::transparent(w, h, origin);
    if let Some(node) = doc.node(id) {
        blend_content(&node.kind, &mut buf, BlendMode::Normal, node.props.opacity);
    }
    buf
}

/// Children are stored top-first (panel order); painter's algorithm wants
/// bottom-first. Clipping masks (DOC-4): a run of `clip` content layers above a
/// content base (Raster/Vector/Smart) is masked by the base's alpha (spec
/// 0046; generalized beyond rasters in spec 0057).
fn composite_children(doc: &Document, parent: NodeId, backdrop: &mut Buffer) {
    let kids: Vec<NodeId> = doc.children(parent).iter().rev().copied().collect();
    let visible = |id: NodeId| doc.node(id).is_some_and(|n| n.props.visible);
    // Layers with pixels of their own — valid clip bases and clip members.
    let is_content = |id: NodeId| {
        matches!(
            doc.node(id).map(|n| &n.kind),
            Some(NodeKind::Raster(_) | NodeKind::Vector(_) | NodeKind::Smart(_))
        )
    };
    let mut i = 0;
    while i < kids.len() {
        let id = kids[i];
        if !visible(id) {
            i += 1;
            continue;
        }
        let clipped = doc.node(id).expect("present").props.clip;
        // A content base (clip=false) may carry a run of clip content layers.
        if is_content(id) && !clipped {
            let mut clips = Vec::new();
            let mut j = i + 1;
            while j < kids.len() {
                if !visible(kids[j]) {
                    j += 1;
                    continue;
                }
                if doc.node(kids[j]).expect("present").props.clip && is_content(kids[j]) {
                    clips.push(kids[j]);
                    j += 1;
                } else {
                    break;
                }
            }
            if clips.is_empty() {
                composite_node(doc, id, backdrop);
                i += 1;
            } else {
                let (w, h, origin) = (backdrop.w, backdrop.h, backdrop.origin);
                let mut base = render_layer_isolated(doc, id, w, h, origin);
                let base_alpha: Vec<f32> = base.px.iter().map(|p| p[3]).collect();
                for c in clips {
                    let mut cb = render_layer_isolated(doc, c, w, h, origin);
                    for (k, p) in cb.px.iter_mut().enumerate() {
                        p[3] *= base_alpha[k]; // clip to base coverage
                    }
                    let mode = doc.node(c).expect("present").props.blend;
                    blend_onto(&mut base, &BufferSource(&cb), mode, 1.0);
                }
                let base_mode = doc.node(id).expect("present").props.blend;
                blend_onto(backdrop, &BufferSource(&base), base_mode, 1.0);
                i = j;
            }
        } else {
            composite_node(doc, id, backdrop);
            i += 1;
        }
    }
}

/// Flatten the whole document to straight-alpha RGBA8, `width × height` from
/// document origin.
pub fn composite_rgba8(doc: &Document, width: u32, height: u32) -> Vec<u8> {
    composite_region_rgba8(doc, 0, 0, width, height)
}

/// Composite only the doc-space rect `[x0, y0, x0+w, y0+h)` — identical pixels
/// to the corresponding slice of the full composite (spec 0006).
pub fn composite_region_rgba8(doc: &Document, x0: i32, y0: i32, w: u32, h: u32) -> Vec<u8> {
    let mut backdrop = Buffer::transparent(w as usize, h as usize, [x0, y0]);
    composite_children(doc, doc.root(), &mut backdrop);
    let mut out = Vec::with_capacity(backdrop.px.len() * 4);
    for p in &backdrop.px {
        for c in p {
            out.push(crate::quantize_rgba8(*c));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use atelier_core::command::AddNode;
    use atelier_core::{
        Command, LayerProps, Node, NodeKind, PlaceholderArt, ProjectFocus, RasterContent,
    };

    fn solid_layer(name: &str, rect: [f32; 4], color: [f32; 4]) -> Node {
        Node::new(
            LayerProps::named(name),
            NodeKind::Raster(RasterContent::from_placeholder(PlaceholderArt {
                bounds: rect,
                color,
            })),
        )
    }

    fn add(doc: &mut Document, node: Node, parent: NodeId, index: usize) -> NodeId {
        let mut cmd = AddNode::new(doc, node, parent, index);
        cmd.apply(doc);
        cmd.id
    }

    fn px(buf: &[u8], w: u32, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * w + x) * 4) as usize;
        [buf[i], buf[i + 1], buf[i + 2], buf[i + 3]]
    }

    #[test]
    fn single_layer_over_transparent_is_source() {
        let mut doc = Document::new([8, 8], ProjectFocus::Raster);
        let root = doc.root();
        add(&mut doc, solid_layer("red", [0.0, 0.0, 4.0, 4.0], [1.0, 0.0, 0.0, 1.0]), root, 0);
        let out = composite_rgba8(&doc, 8, 8);
        assert_eq!(px(&out, 8, 0, 0), [255, 0, 0, 255]);
        assert_eq!(px(&out, 8, 3, 3), [255, 0, 0, 255]);
        assert_eq!(px(&out, 8, 4, 4), [0, 0, 0, 0], "outside the filled rect");
    }

    #[test]
    fn vector_layer_interleaves_in_z_order() {
        use atelier_core::atelier_vector::{Path, Shape};
        use atelier_core::VectorContent;
        let mut doc = Document::new([8, 8], ProjectFocus::Vector);
        let root = doc.root();
        // Bottom: blue raster covering the whole canvas.
        add(&mut doc, solid_layer("blue", [0.0, 0.0, 8.0, 8.0], [0.0, 0.0, 1.0, 1.0]), root, 0);
        // Middle: green vector rect over the whole canvas, above the blue raster.
        let vec_node = Node::new(
            LayerProps::named("vec"),
            NodeKind::Vector(VectorContent {
                shapes: vec![Shape::filled(Path::rect(0.0, 0.0, 8.0, 8.0), [0.0, 1.0, 0.0, 1.0])],
            }),
        );
        add(&mut doc, vec_node, root, 0);
        // Top: red raster covering only the top-left quadrant, above the vector.
        add(&mut doc, solid_layer("red", [0.0, 0.0, 4.0, 4.0], [1.0, 0.0, 0.0, 1.0]), root, 0);

        let out = composite_rgba8(&doc, 8, 8);
        assert_eq!(px(&out, 8, 2, 2), [255, 0, 0, 255], "raster wins above the vector");
        assert_eq!(px(&out, 8, 6, 6), [0, 255, 0, 255], "vector wins above the raster");
    }

    #[test]
    fn smart_object_composites_embedded_doc_at_offset() {
        use atelier_core::SmartContent;
        // Embedded doc: a red 4x4 square at its own (0,0).
        let mut inner = Document::new([8, 8], ProjectFocus::Raster);
        let iroot = inner.root();
        add(&mut inner, solid_layer("r", [0.0, 0.0, 4.0, 4.0], [1.0, 0.0, 0.0, 1.0]), iroot, 0);
        // Outer doc places the smart object at offset (2,2).
        let mut doc = Document::new([8, 8], ProjectFocus::Raster);
        let root = doc.root();
        let smart = Node::new(
            LayerProps::named("smart"),
            NodeKind::Smart(SmartContent { doc: Box::new(inner), offset: [2, 2], scale: [1.0, 1.0], rotation: 0.0 }),
        );
        add(&mut doc, smart, root, 0);

        let out = composite_rgba8(&doc, 8, 8);
        assert_eq!(px(&out, 8, 0, 0), [0, 0, 0, 0], "embedded content shifted off (0,0)");
        assert_eq!(px(&out, 8, 3, 3), [255, 0, 0, 255], "red square shifted to (2,2)..(6,6)");
        assert_eq!(px(&out, 8, 5, 5), [255, 0, 0, 255], "still inside the shifted square");
        assert_eq!(px(&out, 8, 6, 6), [0, 0, 0, 0], "past the shifted square");
    }

    #[test]
    fn smart_object_opacity_applies_once() {
        use atelier_core::SmartContent;
        let mut inner = Document::new([4, 4], ProjectFocus::Raster);
        let iroot = inner.root();
        // Embedded layer at full opacity; the smart node carries the 0.5.
        add(&mut inner, solid_layer("w", [0.0, 0.0, 4.0, 4.0], [1.0, 1.0, 1.0, 1.0]), iroot, 0);
        let mut doc = Document::new([4, 4], ProjectFocus::Raster);
        let root = doc.root();
        let id = add(
            &mut doc,
            Node::new(
                LayerProps::named("smart"),
                NodeKind::Smart(SmartContent { doc: Box::new(inner), offset: [0, 0], scale: [1.0, 1.0], rotation: 0.0 }),
            ),
            root,
            0,
        );
        doc.node_mut(id).unwrap().props.opacity = 0.5;
        let out = composite_rgba8(&doc, 4, 4);
        let got = px(&out, 4, 1, 1);
        assert_eq!(got[3], 128, "smart opacity applied exactly once");
        assert_eq!(got[0], 255, "straight color survives");
    }

    #[test]
    fn smart_object_scales_non_destructively() {
        use atelier_core::SmartContent;
        // Embedded 4x4 fully red, scaled 2× about its centre (spec 0055/0056).
        let mut inner = Document::new([4, 4], ProjectFocus::Raster);
        let iroot = inner.root();
        add(&mut inner, solid_layer("r", [0.0, 0.0, 4.0, 4.0], [1.0, 0.0, 0.0, 1.0]), iroot, 0);
        let mut doc = Document::new([8, 8], ProjectFocus::Raster);
        let root = doc.root();
        add(
            &mut doc,
            Node::new(
                LayerProps::named("smart"),
                NodeKind::Smart(SmartContent {
                    doc: Box::new(inner),
                    offset: [0, 0],
                    scale: [2.0, 2.0],
                    rotation: 0.0,
                }),
            ),
            root,
            0,
        );
        let out = composite_rgba8(&doc, 8, 8);
        // 2× about the centre: the 4×4 doc grows to cover parent ~[-2,6).
        assert_eq!(px(&out, 8, 5, 5), [255, 0, 0, 255], "scaled 2× reaches (5,5)");
        assert_eq!(px(&out, 8, 6, 6), [0, 0, 0, 0], "past the 2×-scaled doc");
    }

    #[test]
    fn smart_object_rotates_about_center() {
        use atelier_core::SmartContent;
        // Embedded 4x4 with a single red marker at (3,0).
        let mut inner = Document::new([4, 4], ProjectFocus::Raster);
        let iroot = inner.root();
        add(&mut inner, solid_layer("m", [3.0, 0.0, 1.0, 1.0], [1.0, 0.0, 0.0, 1.0]), iroot, 0);
        let mut doc = Document::new([8, 8], ProjectFocus::Raster);
        let root = doc.root();
        add(
            &mut doc,
            Node::new(
                LayerProps::named("smart"),
                NodeKind::Smart(SmartContent {
                    doc: Box::new(inner),
                    offset: [0, 0],
                    scale: [1.0, 1.0],
                    rotation: std::f32::consts::FRAC_PI_2,
                }),
            ),
            root,
            0,
        );
        let out = composite_rgba8(&doc, 8, 8);
        // 90° about centre (2,2): embedded (3,0) lands at parent (3,3).
        assert_eq!(px(&out, 8, 3, 3), [255, 0, 0, 255], "marker rotated to (3,3)");
        assert_eq!(px(&out, 8, 3, 0), [0, 0, 0, 0], "marker left its original cell");
    }

    /// Spec 0057: clip runs work across layer kinds, not just raster-over-raster.
    #[test]
    fn clipping_mask_works_across_layer_kinds() {
        use atelier_core::atelier_vector::{Path, Shape};
        use atelier_core::VectorContent;
        let vec_layer = |x: f32, y: f32, w: f32, h: f32, color: [f32; 4]| {
            Node::new(
                LayerProps::named("v"),
                NodeKind::Vector(VectorContent {
                    shapes: vec![Shape::filled(Path::rect(x, y, w, h), color)],
                }),
            )
        };

        // Case 1: clipped RASTER over a VECTOR base (left half of the canvas).
        let mut doc = Document::new([8, 8], ProjectFocus::Raster);
        let root = doc.root();
        add(&mut doc, vec_layer(0.0, 0.0, 4.0, 8.0, [0.0, 1.0, 0.0, 1.0]), root, 0);
        let red = add(&mut doc, solid_layer("red", [0.0, 0.0, 8.0, 8.0], [1.0, 0.0, 0.0, 1.0]), root, 0);
        doc.node_mut(red).unwrap().props.clip = true;
        let out = composite_rgba8(&doc, 8, 8);
        assert_eq!(px(&out, 8, 1, 1), [255, 0, 0, 255], "red clips onto vector base");
        assert_eq!(px(&out, 8, 6, 1), [0, 0, 0, 0], "clipped where vector base is empty");

        // Case 2: clipped VECTOR over a RASTER base (left half of the canvas).
        let mut doc2 = Document::new([8, 8], ProjectFocus::Raster);
        let root2 = doc2.root();
        add(&mut doc2, solid_layer("base", [0.0, 0.0, 4.0, 8.0], [0.0, 0.0, 1.0, 1.0]), root2, 0);
        let vid = add(&mut doc2, vec_layer(0.0, 0.0, 8.0, 8.0, [1.0, 1.0, 0.0, 1.0]), root2, 0);
        doc2.node_mut(vid).unwrap().props.clip = true;
        let out2 = composite_rgba8(&doc2, 8, 8);
        assert_eq!(px(&out2, 8, 1, 1), [255, 255, 0, 255], "vector clips onto raster base");
        assert_eq!(px(&out2, 8, 6, 1), [0, 0, 0, 0], "clipped where raster base is empty");
    }

    #[test]
    fn multiply_of_known_colors() {
        let mut doc = Document::new([4, 4], ProjectFocus::Raster);
        let root = doc.root();
        // Backdrop 50% gray, source 50% gray multiplied → 0.5·0.5 = 0.25.
        add(&mut doc, solid_layer("base", [0.0, 0.0, 4.0, 4.0], [0.5, 0.5, 0.5, 1.0]), root, 0);
        let top_id = add(
            &mut doc,
            solid_layer("mul", [0.0, 0.0, 4.0, 4.0], [0.5, 0.5, 0.5, 1.0]),
            root,
            0,
        );
        doc.node_mut(top_id).unwrap().props.blend = BlendMode::Multiply;
        let out = composite_rgba8(&doc, 4, 4);
        // 0.5 stored as 128/255≈0.50196; 0.50196² = 0.25196 → 64.25 → 64
        let got = px(&out, 4, 1, 1);
        assert!((got[0] as i32 - 64).abs() <= 1, "multiply gave {got:?}");
        assert_eq!(got[3], 255);
    }

    #[test]
    fn opacity_scales_coverage() {
        let mut doc = Document::new([2, 2], ProjectFocus::Raster);
        let root = doc.root();
        let id = add(
            &mut doc,
            solid_layer("half", [0.0, 0.0, 2.0, 2.0], [1.0, 1.0, 1.0, 1.0]),
            root,
            0,
        );
        doc.node_mut(id).unwrap().props.opacity = 0.5;
        let out = composite_rgba8(&doc, 2, 2);
        let got = px(&out, 2, 0, 0);
        assert_eq!(got[3], 128, "alpha = opacity over transparent");
        assert_eq!(got[0], 255, "straight color survives");
    }

    #[test]
    fn group_isolation_differs_from_pass_through() {
        // Backdrop red; group contains a white layer at 50% group opacity.
        // Pass-through(op 1) == direct; isolated Normal at 0.5 averages with red.
        let build = |group_blend: BlendMode, group_op: f32| {
            let mut doc = Document::new([2, 2], ProjectFocus::Raster);
            let root = doc.root();
            add(
                &mut doc,
                solid_layer("red", [0.0, 0.0, 2.0, 2.0], [1.0, 0.0, 0.0, 1.0]),
                root,
                0,
            );
            let g = add(&mut doc, Node::group("g"), root, 0);
            doc.node_mut(g).unwrap().props.blend = group_blend;
            doc.node_mut(g).unwrap().props.opacity = group_op;
            add(
                &mut doc,
                solid_layer("white", [0.0, 0.0, 2.0, 2.0], [1.0, 1.0, 1.0, 1.0]),
                g,
                0,
            );
            composite_rgba8(&doc, 2, 2)
        };
        let isolated = build(BlendMode::Normal, 0.5);
        // white at 0.5 over red → (0.5·1 + 0.5·1·…) co = 0.5*white + 0.5*red
        assert_eq!(px(&isolated, 2, 0, 0), [255, 128, 128, 255]);

        let pass = build(BlendMode::PassThrough, 1.0);
        assert_eq!(px(&pass, 2, 0, 0), [255, 255, 255, 255], "pass-through direct");
    }

    #[test]
    fn hidden_layers_and_empty_doc_composite_clean() {
        let mut doc = Document::new([2, 2], ProjectFocus::Raster);
        let root = doc.root();
        let id = add(
            &mut doc,
            solid_layer("x", [0.0, 0.0, 2.0, 2.0], [1.0, 0.0, 0.0, 1.0]),
            root,
            0,
        );
        doc.node_mut(id).unwrap().props.visible = false;
        let out = composite_rgba8(&doc, 2, 2);
        assert_eq!(px(&out, 2, 0, 0), [0, 0, 0, 0]);
    }

    #[test]
    fn every_mode_composites_finite_output() {
        for mode in BlendMode::ALL {
            if mode == BlendMode::PassThrough {
                continue; // group-only
            }
            let mut doc = Document::new([3, 3], ProjectFocus::Raster);
            let root = doc.root();
            add(
                &mut doc,
                solid_layer("base", [0.0, 0.0, 3.0, 3.0], [0.7, 0.3, 0.1, 0.8]),
                root,
                0,
            );
            let id = add(
                &mut doc,
                solid_layer("top", [0.0, 0.0, 3.0, 3.0], [0.2, 0.9, 0.5, 0.6]),
                root,
                0,
            );
            doc.node_mut(id).unwrap().props.blend = mode;
            doc.node_mut(id).unwrap().props.opacity = 0.7;
            let out = composite_rgba8(&doc, 3, 3);
            assert_eq!(out.len(), 36, "{mode:?}");
            // u8 output is inherently in range; just ensure deterministic repeat.
            assert_eq!(out, composite_rgba8(&doc, 3, 3), "{mode:?} deterministic");
        }
    }

    /// Region composite must equal the slice of the full composite — including
    /// Dissolve (absolute-coord hash) and offset layers.
    #[test]
    fn clipping_mask_limits_layer_to_base_alpha() {
        use atelier_core::LayerProps;
        let mut doc = Document::new([4, 4], ProjectFocus::Raster);
        let root = doc.root();
        // Base: opaque green only in the left half [0,0,2,4].
        add(&mut doc, solid_layer("base", [0.0, 0.0, 2.0, 4.0], [0.0, 1.0, 0.0, 1.0]), root, 0);
        // Clip layer: red filling the whole canvas, clipped to base.
        let clip = Node::new(
            LayerProps::named("clip"),
            NodeKind::Raster(RasterContent::from_placeholder(PlaceholderArt {
                bounds: [0.0, 0.0, 4.0, 4.0],
                color: [1.0, 0.0, 0.0, 1.0],
            })),
        );
        let cid = add(&mut doc, clip, root, 0); // top
        doc.node_mut(cid).unwrap().props.clip = true;

        let out = composite_rgba8(&doc, 4, 4);
        // Left half: clip red shows (base present) → red.
        assert_eq!(px(&out, 4, 0, 0), [255, 0, 0, 255], "clip visible over base");
        // Right half: base absent → clip hidden → fully transparent.
        assert_eq!(px(&out, 4, 3, 0), [0, 0, 0, 0], "clip hidden where base is transparent");
    }

    #[test]
    fn adjustment_layer_retones_only_below() {
        use atelier_core::Adjustment;
        let mut doc = Document::new([4, 4], ProjectFocus::Raster);
        let root = doc.root();
        // bottom (red) — below the adjust; top (green) — above it.
        add(&mut doc, solid_layer("below", [0.0, 0.0, 4.0, 4.0], [1.0, 0.0, 0.0, 1.0]), root, 0);
        let adj = add(
            &mut doc,
            Node::new(
                atelier_core::LayerProps::named("inv"),
                NodeKind::Adjustment(Adjustment::Invert),
            ),
            root,
            0,
        );
        let _ = adj;
        // 'top' painted only on the right half so we can see both regions.
        add(&mut doc, solid_layer("top", [2.0, 0.0, 2.0, 4.0], [0.0, 1.0, 0.0, 1.0]), root, 0);

        let out = composite_rgba8(&doc, 4, 4);
        // Left col: only 'below' red, inverted by the adjust → cyan.
        assert_eq!(px(&out, 4, 0, 0), [0, 255, 255, 255], "below inverted");
        // Right col: 'top' green sits above the adjust → untouched.
        assert_eq!(px(&out, 4, 3, 0), [0, 255, 0, 255], "above adjust untouched");
    }

    #[test]
    fn adjustment_layer_respects_visibility_and_opacity() {
        use atelier_core::Adjustment;
        let build = |visible: bool, opacity: f32| {
            let mut doc = Document::new([2, 2], ProjectFocus::Raster);
            let root = doc.root();
            add(&mut doc, solid_layer("b", [0.0, 0.0, 2.0, 2.0], [0.0, 0.0, 0.0, 1.0]), root, 0);
            let a = add(
                &mut doc,
                Node::new(
                    atelier_core::LayerProps::named("inv"),
                    NodeKind::Adjustment(Adjustment::Invert),
                ),
                root,
                0,
            );
            doc.node_mut(a).unwrap().props.visible = visible;
            doc.node_mut(a).unwrap().props.opacity = opacity;
            composite_rgba8(&doc, 2, 2)
        };
        assert_eq!(px(&build(false, 1.0), 2, 0, 0), [0, 0, 0, 255], "hidden adjust = no-op");
        assert_eq!(px(&build(true, 1.0), 2, 0, 0), [255, 255, 255, 255], "full invert");
        let half = build(true, 0.5);
        assert!((px(&half, 2, 0, 0)[0] as i32 - 128).abs() <= 1, "opacity lerps");
    }

    #[test]
    fn region_equals_slice_of_full() {
        let mut doc = Document::new([96, 96], ProjectFocus::Raster);
        let root = doc.root();
        add(&mut doc, solid_layer("base", [0.0, 0.0, 96.0, 96.0], [0.3, 0.5, 0.7, 0.9]), root, 0);
        let moved = add(
            &mut doc,
            solid_layer("moved", [0.0, 0.0, 40.0, 40.0], [0.9, 0.2, 0.1, 0.8]),
            root,
            0,
        );
        if let NodeKind::Raster(c) = &mut doc.node_mut(moved).unwrap().kind {
            c.offset = [17, -5];
        }
        let dis = add(
            &mut doc,
            solid_layer("dis", [10.0, 10.0, 60.0, 60.0], [0.1, 0.9, 0.3, 1.0]),
            root,
            0,
        );
        doc.node_mut(dis).unwrap().props.blend = BlendMode::Dissolve;
        doc.node_mut(dis).unwrap().props.opacity = 0.5;
        let g = add(&mut doc, Node::group("g"), root, 0);
        doc.node_mut(g).unwrap().props.opacity = 0.6; // isolated
        doc.node_mut(g).unwrap().props.blend = BlendMode::Normal;
        add(&mut doc, solid_layer("in", [30.0, 30.0, 50.0, 50.0], [1.0, 1.0, 0.2, 1.0]), g, 0);

        let full = composite_rgba8(&doc, 96, 96);
        let (rx, ry, rw, rh) = (23, 11, 41, 37);
        let region = composite_region_rgba8(&doc, rx, ry, rw, rh);
        for y in 0..rh as usize {
            for x in 0..rw as usize {
                let r = &region[(y * rw as usize + x) * 4..][..4];
                let fy = y + ry as usize;
                let fx = x + rx as usize;
                let f = &full[(fy * 96 + fx) * 4..][..4];
                assert_eq!(r, f, "mismatch at region ({x},{y})");
            }
        }
    }

    /// Perf evidence for the Phase 2 gate — run manually:
    /// `cargo test -p atelier-raster --release -- --ignored --nocapture`
    #[test]
    #[ignore = "perf measurement; run locally in release"]
    fn perf_numbers_for_phase2_gate() {
        let mut doc = Document::new([4096, 4096], ProjectFocus::Raster);
        let root = doc.root();
        for i in 0..50 {
            let x = (i % 8) as f32 * 450.0;
            let y = (i / 8) as f32 * 550.0;
            let id = add(
                &mut doc,
                solid_layer(&format!("l{i}"), [x, y, 512.0, 512.0], [0.5, 0.3, 0.8, 0.9]),
                root,
                0,
            );
            doc.node_mut(id).unwrap().props.blend =
                BlendMode::ALL[2 + (i % 26) as usize];
            doc.node_mut(id).unwrap().props.opacity = 0.8;
        }
        let t = std::time::Instant::now();
        let _ = composite_region_rgba8(&doc, 1000, 1000, 256, 256);
        println!("256x256 region over 50 layers: {:?}", t.elapsed());

        let t = std::time::Instant::now();
        let _ = composite_rgba8(&doc, 4096, 4096);
        println!("full 4096x4096 x 50 layers: {:?}", t.elapsed());
    }

    #[test]
    fn dissolve_is_all_or_nothing_per_pixel() {
        let mut doc = Document::new([16, 16], ProjectFocus::Raster);
        let root = doc.root();
        let id = add(
            &mut doc,
            solid_layer("d", [0.0, 0.0, 16.0, 16.0], [0.0, 1.0, 0.0, 1.0]),
            root,
            0,
        );
        doc.node_mut(id).unwrap().props.blend = BlendMode::Dissolve;
        doc.node_mut(id).unwrap().props.opacity = 0.5;
        let out = composite_rgba8(&doc, 16, 16);
        let mut kept = 0;
        for y in 0..16 {
            for x in 0..16 {
                let p = px(&out, 16, x, y);
                assert!(p[3] == 0 || p[3] == 255, "dissolve must not produce partial alpha");
                if p[3] == 255 {
                    assert_eq!(p[1], 255);
                    kept += 1;
                }
            }
        }
        assert!((50..200).contains(&kept), "~half survive at 50%: {kept}/256");
    }
}
