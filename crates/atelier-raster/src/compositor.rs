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

/// f32 straight-alpha RGBA framebuffer.
struct Buffer {
    w: usize,
    h: usize,
    /// `[r, g, b, a]` per pixel, row-major.
    px: Vec<[f32; 4]>,
}

impl Buffer {
    fn transparent(w: usize, h: usize) -> Self {
        Self { w, h, px: vec![[0.0; 4]; w * h] }
    }
}

/// Per-pixel source fetch for one layer: returns straight-alpha RGBA.
trait Source {
    fn sample(&self, x: i32, y: i32) -> [f32; 4];
}

struct TileSource<'a> {
    tiles: &'a atelier_core::TileMap,
    offset: [i32; 2],
}

impl Source for TileSource<'_> {
    fn sample(&self, x: i32, y: i32) -> [f32; 4] {
        let [r, g, b, a] = self.tiles.pixel(x - self.offset[0], y - self.offset[1]);
        [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a as f32 / 255.0]
    }
}

struct BufferSource<'a>(&'a Buffer);

impl Source for BufferSource<'_> {
    fn sample(&self, x: i32, y: i32) -> [f32; 4] {
        if x < 0 || y < 0 || x >= self.0.w as i32 || y >= self.0.h as i32 {
            return [0.0; 4];
        }
        self.0.px[y as usize * self.0.w + x as usize]
    }
}

/// Composite one source over `backdrop` with `mode` and `opacity`.
fn blend_onto(backdrop: &mut Buffer, src: &dyn Source, mode: BlendMode, opacity: f32) {
    for y in 0..backdrop.h {
        for x in 0..backdrop.w {
            let i = y * backdrop.w + x;
            let s = src.sample(x as i32, y as i32);
            let (mut s_rgb, mut s_a) = ([s[0], s[1], s[2]], s[3] * opacity);
            let mut mode = mode;

            if mode == BlendMode::Dissolve {
                // Dissolve: alpha becomes a per-pixel all-or-nothing gate.
                if s[3] > 0.0 && dissolve_keeps(x as i32, y as i32, s_a) {
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

/// Children are stored top-first (panel order); painter's algorithm wants
/// bottom-first, so iterate reversed.
fn composite_children(doc: &Document, parent: NodeId, backdrop: &mut Buffer) {
    for &id in doc.children(parent).iter().rev() {
        let Some(node) = doc.node(id) else { continue };
        if !node.props.visible {
            continue;
        }
        let props = &node.props;
        match &node.kind {
            NodeKind::Raster(content) => {
                let src = TileSource { tiles: &content.tiles, offset: content.offset };
                blend_onto(backdrop, &src, props.blend, props.opacity);
            }
            NodeKind::Group { .. } => {
                if props.blend == BlendMode::PassThrough && props.opacity >= 1.0 {
                    composite_children(doc, id, backdrop);
                } else {
                    let mut isolated = Buffer::transparent(backdrop.w, backdrop.h);
                    composite_children(doc, id, &mut isolated);
                    blend_onto(backdrop, &BufferSource(&isolated), props.blend, props.opacity);
                }
            }
            // Vector tessellation (spec 0004+), adjustment/text/smart/fill: later phases.
            _ => {}
        }
    }
}

/// Flatten the whole document to straight-alpha RGBA8, `width × height` from
/// document origin.
pub fn composite_rgba8(doc: &Document, width: u32, height: u32) -> Vec<u8> {
    let mut backdrop = Buffer::transparent(width as usize, height as usize);
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
