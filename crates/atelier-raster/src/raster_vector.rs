//! Rasterize a vector layer's filled shapes into a tile map (INT-2, spec 0023).
//! Tessellates each filled shape and scan-fills its triangles into 8-bit tiles.
//! No anti-aliasing this slice (coverage = inside/outside at pixel centers).

use atelier_core::atelier_vector::{tessellate, Vertex, VectorContent};
use atelier_core::TileMap;

fn to_u8(c: [f32; 4]) -> [u8; 4] {
    [
        (c[0] * 255.0 + 0.5).clamp(0.0, 255.0) as u8,
        (c[1] * 255.0 + 0.5).clamp(0.0, 255.0) as u8,
        (c[2] * 255.0 + 0.5).clamp(0.0, 255.0) as u8,
        (c[3] * 255.0 + 0.5).clamp(0.0, 255.0) as u8,
    ]
}

#[inline]
fn edge(a: [f32; 2], b: [f32; 2], p: [f32; 2]) -> f32 {
    (p[0] - a[0]) * (b[1] - a[1]) - (p[1] - a[1]) * (b[0] - a[0])
}

fn fill_triangle(tiles: &mut TileMap, a: Vertex, b: Vertex, c: Vertex, w: i32, h: i32) {
    let (pa, pb, pc) = (a.pos, b.pos, c.pos);
    let min_x = pa[0].min(pb[0]).min(pc[0]).floor().max(0.0) as i32;
    let min_y = pa[1].min(pb[1]).min(pc[1]).floor().max(0.0) as i32;
    let max_x = (pa[0].max(pb[0]).max(pc[0]).ceil() as i32).min(w);
    let max_y = (pa[1].max(pb[1]).max(pc[1]).ceil() as i32).min(h);
    let color = to_u8(a.color);
    for y in min_y..max_y {
        for x in min_x..max_x {
            let p = [x as f32 + 0.5, y as f32 + 0.5];
            let (w0, w1, w2) = (edge(pb, pc, p), edge(pc, pa, p), edge(pa, pb, p));
            // Inside if all edge functions share a sign (cover both windings).
            let all_neg = w0 <= 0.0 && w1 <= 0.0 && w2 <= 0.0;
            let all_pos = w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0;
            if all_neg || all_pos {
                tiles.set_pixel(x, y, color);
            }
        }
    }
}

/// Rasterize `content`'s filled shapes into a `w×h` tile map (doc origin).
pub fn rasterize_vector(content: &VectorContent, w: u32, h: u32) -> TileMap {
    let mut tiles = TileMap::new();
    for shape in &content.shapes {
        if shape.fill.is_none() {
            continue; // strokes not rasterized this slice
        }
        let mesh = tessellate(shape);
        for tri in mesh.indices.chunks_exact(3) {
            let a = mesh.vertices[tri[0] as usize];
            let b = mesh.vertices[tri[1] as usize];
            let c = mesh.vertices[tri[2] as usize];
            fill_triangle(&mut tiles, a, b, c, w as i32, h as i32);
        }
    }
    tiles
}

#[cfg(test)]
mod tests {
    use super::*;
    use atelier_core::atelier_vector::{Path, Shape};

    #[test]
    fn rasterizes_a_filled_rect() {
        let content = VectorContent {
            shapes: vec![Shape::filled(Path::rect(4.0, 4.0, 20.0, 20.0), [1.0, 0.0, 0.0, 1.0])],
        };
        let tiles = rasterize_vector(&content, 64, 64);
        assert!(!tiles.is_empty(), "produced pixels");
        assert_eq!(tiles.pixel(12, 12), [255, 0, 0, 255], "inside the rect");
        assert_eq!(tiles.pixel(40, 40), [0, 0, 0, 0], "outside untouched");
    }

    #[test]
    fn unfilled_shape_produces_nothing() {
        let mut s = Shape::filled(Path::rect(0.0, 0.0, 10.0, 10.0), [1.0; 4]);
        s.fill = None;
        let content = VectorContent { shapes: vec![s] };
        assert!(rasterize_vector(&content, 32, 32).is_empty());
    }
}
