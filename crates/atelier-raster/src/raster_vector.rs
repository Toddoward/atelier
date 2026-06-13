//! Rasterize a vector layer's filled shapes into a tile map (INT-2, spec 0023;
//! anti-aliased per spec 0025). Tessellates each filled shape and accumulates
//! 4×4 subsample coverage per pixel, then writes straight-alpha src-over.

use atelier_core::atelier_vector::{tessellate, Vertex, VectorContent};
use atelier_core::TileMap;
use std::collections::HashMap;

const SS: i32 = 4; // subsamples per axis
const SS2: u32 = (SS * SS) as u32;

#[inline]
fn edge(a: [f32; 2], b: [f32; 2], p: [f32; 2]) -> f32 {
    (p[0] - a[0]) * (b[1] - a[1]) - (p[1] - a[1]) * (b[0] - a[0])
}

/// Add this triangle's subsample hits into the per-pixel coverage accumulator.
fn accumulate(cov: &mut HashMap<(i32, i32), u32>, a: Vertex, b: Vertex, c: Vertex, w: i32, h: i32) {
    let (pa, pb, pc) = (a.pos, b.pos, c.pos);
    let min_x = (pa[0].min(pb[0]).min(pc[0]).floor() as i32).max(0);
    let min_y = (pa[1].min(pb[1]).min(pc[1]).floor() as i32).max(0);
    let max_x = (pa[0].max(pb[0]).max(pc[0]).ceil() as i32).min(w);
    let max_y = (pa[1].max(pb[1]).max(pc[1]).ceil() as i32).min(h);
    for y in min_y..max_y {
        for x in min_x..max_x {
            let mut hits = 0u32;
            for sy in 0..SS {
                for sx in 0..SS {
                    let p = [
                        x as f32 + (sx as f32 + 0.5) / SS as f32,
                        y as f32 + (sy as f32 + 0.5) / SS as f32,
                    ];
                    let (w0, w1, w2) = (edge(pb, pc, p), edge(pc, pa, p), edge(pa, pb, p));
                    if (w0 <= 0.0 && w1 <= 0.0 && w2 <= 0.0)
                        || (w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0)
                    {
                        hits += 1;
                    }
                }
            }
            if hits > 0 {
                *cov.entry((x, y)).or_insert(0) += hits;
            }
        }
    }
}

/// Straight-alpha source-over of `src` (already coverage-scaled alpha) onto `dst`.
fn src_over(dst: [u8; 4], src: [f32; 4]) -> [u8; 4] {
    let sa = src[3];
    if sa <= 0.0 {
        return dst;
    }
    let da = dst[3] as f32 / 255.0;
    let ao = sa + da * (1.0 - sa);
    if ao <= 0.0 {
        return [0; 4];
    }
    let q = crate::quantize_rgba8;
    let mut out = [0u8; 4];
    for i in 0..3 {
        let dc = dst[i] as f32 / 255.0;
        out[i] = q((src[i] * sa + dc * da * (1.0 - sa)) / ao);
    }
    out[3] = q(ao);
    out
}

/// Rasterize `content`'s filled shapes into a `w×h` tile map (doc origin), AA'd.
pub fn rasterize_vector(content: &VectorContent, w: u32, h: u32) -> TileMap {
    let mut tiles = TileMap::new();
    let (wi, hi) = (w as i32, h as i32);
    for shape in &content.shapes {
        let Some(fill) = shape.fill else { continue }; // strokes not rasterized yet
        let mesh = tessellate(shape);
        let mut cov: HashMap<(i32, i32), u32> = HashMap::new();
        for tri in mesh.indices.chunks_exact(3) {
            let a = mesh.vertices[tri[0] as usize];
            let b = mesh.vertices[tri[1] as usize];
            let c = mesh.vertices[tri[2] as usize];
            accumulate(&mut cov, a, b, c, wi, hi);
        }
        for ((x, y), hits) in cov {
            let coverage = hits.min(SS2) as f32 / SS2 as f32;
            let src = [fill[0], fill[1], fill[2], fill[3] * coverage];
            let out = src_over(tiles.pixel(x, y), src);
            tiles.set_pixel(x, y, out);
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
        assert_eq!(tiles.pixel(12, 12), [255, 0, 0, 255], "inside the rect, full coverage");
        assert_eq!(tiles.pixel(40, 40), [0, 0, 0, 0], "outside untouched");
    }

    #[test]
    fn unfilled_shape_produces_nothing() {
        let mut s = Shape::filled(Path::rect(0.0, 0.0, 10.0, 10.0), [1.0; 4]);
        s.fill = None;
        let content = VectorContent { shapes: vec![s] };
        assert!(rasterize_vector(&content, 32, 32).is_empty());
    }

    #[test]
    fn edges_are_antialiased() {
        // A half-pixel-offset rect leaves its left column partially covered.
        let content = VectorContent {
            shapes: vec![Shape::filled(Path::rect(4.5, 4.0, 20.0, 20.0), [1.0, 1.0, 1.0, 1.0])],
        };
        let tiles = rasterize_vector(&content, 64, 64);
        let a = tiles.pixel(4, 10)[3];
        assert!(a > 0 && a < 255, "left edge column is partially covered: {a}");
        assert_eq!(tiles.pixel(12, 10)[3], 255, "interior fully covered");
    }
}
