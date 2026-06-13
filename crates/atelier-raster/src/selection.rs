//! Selection shape rasterizers, boundary extraction (spec 0007), and
//! magic-wand / morphology / feather operations (spec 0011).
//! All coordinates are doc-space pixels; rects are half-open.

use atelier_core::{Mask, TileMap};
use std::collections::VecDeque;

/// Flood-fill select connected doc pixels whose color is within `tolerance`
/// (max abs channel delta, 0..=255) of the seed pixel. Bounded to the canvas
/// `size`; samples the layer drawn at `offset`.
pub fn magic_wand(
    tiles: &TileMap,
    offset: [i32; 2],
    seed: [i32; 2],
    tolerance: u8,
    size: [u32; 2],
) -> Mask {
    let (w, h) = (size[0] as i32, size[1] as i32);
    let mut out = Mask::new();
    if seed[0] < 0 || seed[1] < 0 || seed[0] >= w || seed[1] >= h {
        return out;
    }
    let sample = |x: i32, y: i32| tiles.pixel(x - offset[0], y - offset[1]);
    let matches = |a: [u8; 4], b: [u8; 4]| {
        (0..4).all(|i| (a[i] as i32 - b[i] as i32).unsigned_abs() <= tolerance as u32)
    };
    let target = sample(seed[0], seed[1]);

    let mut visited = vec![false; (w * h) as usize];
    let mut q = VecDeque::new();
    let idx = |x: i32, y: i32| (y * w + x) as usize;
    visited[idx(seed[0], seed[1])] = true;
    q.push_back(seed);
    while let Some([x, y]) = q.pop_front() {
        out.set(x, y, 255);
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let (nx, ny) = (x + dx, y + dy);
            if nx < 0 || ny < 0 || nx >= w || ny >= h || visited[idx(nx, ny)] {
                continue;
            }
            visited[idx(nx, ny)] = true;
            if matches(sample(nx, ny), target) {
                q.push_back([nx, ny]);
            }
        }
    }
    out
}

/// Chebyshev dilate (grow) the selection by `r` pixels.
pub fn grow(mask: &Mask, r: i32) -> Mask {
    morph(mask, r, true)
}

/// Chebyshev erode (shrink) the selection by `r` pixels.
pub fn shrink(mask: &Mask, r: i32) -> Mask {
    morph(mask, r, false)
}

fn morph(mask: &Mask, r: i32, dilate: bool) -> Mask {
    let Some([x0, y0, x1, y1]) = mask.bounds() else { return Mask::new() };
    let mut out = Mask::new();
    // Dilate can extend beyond current bounds by r; erode stays within.
    let pad = if dilate { r } else { 0 };
    for y in (y0 - pad)..(y1 + pad) {
        for x in (x0 - pad)..(x1 + pad) {
            let mut acc = if dilate { 0u8 } else { 255u8 };
            for dy in -r..=r {
                for dx in -r..=r {
                    let v = mask.get(x + dx, y + dy);
                    acc = if dilate { acc.max(v) } else { acc.min(v) };
                }
            }
            if acc != 0 {
                out.set(x, y, acc);
            }
        }
    }
    out
}

/// Soften selection edges with two box-blur passes (separable) of radius `r`.
pub fn feather(mask: &Mask, r: i32) -> Mask {
    if r <= 0 {
        return mask.clone();
    }
    let Some([x0, y0, x1, y1]) = mask.bounds() else { return Mask::new() };
    let (px0, py0, px1, py1) = (x0 - r, y0 - r, x1 + r, y1 + r);
    let (w, h) = ((px1 - px0) as usize, (py1 - py0) as usize);
    let at = |buf: &[u16], x: usize, y: usize| buf[y * w + x] as u32;

    // Load into a local buffer.
    let mut a = vec![0u16; w * h];
    for y in 0..h {
        for x in 0..w {
            a[y * w + x] = mask.get(px0 + x as i32, py0 + y as i32) as u16;
        }
    }
    let win = (2 * r + 1) as u32;
    // Horizontal pass.
    let mut b = vec![0u16; w * h];
    for y in 0..h {
        for x in 0..w {
            let mut sum = 0u32;
            for dx in -r..=r {
                let sx = (x as i32 + dx).clamp(0, w as i32 - 1) as usize;
                sum += at(&a, sx, y);
            }
            b[y * w + x] = (sum / win) as u16;
        }
    }
    // Vertical pass.
    let mut out = Mask::new();
    for y in 0..h {
        for x in 0..w {
            let mut sum = 0u32;
            for dy in -r..=r {
                let sy = (y as i32 + dy).clamp(0, h as i32 - 1) as usize;
                sum += at(&b, x, sy);
            }
            let v = (sum / win) as u8;
            if v != 0 {
                out.set(px0 + x as i32, py0 + y as i32, v);
            }
        }
    }
    out
}

/// Axis-aligned rect with antialiased fractional edges.
pub fn rect_mask(x0: f32, y0: f32, x1: f32, y1: f32) -> Mask {
    let (x0, x1) = (x0.min(x1), x0.max(x1));
    let (y0, y1) = (y0.min(y1), y0.max(y1));
    let mut m = Mask::new();
    let (ix0, ix1) = (x0.floor() as i32, x1.ceil() as i32);
    let (iy0, iy1) = (y0.floor() as i32, y1.ceil() as i32);
    for y in iy0..iy1 {
        // Vertical coverage of this pixel row.
        let cy = overlap(y as f32, y as f32 + 1.0, y0, y1);
        for x in ix0..ix1 {
            let c = cy * overlap(x as f32, x as f32 + 1.0, x0, x1);
            if c > 0.0 {
                m.set(x, y, (c * 255.0 + 0.5) as u8);
            }
        }
    }
    m
}

fn overlap(a0: f32, a1: f32, b0: f32, b1: f32) -> f32 {
    (a1.min(b1) - a0.max(b0)).clamp(0.0, 1.0)
}

/// Ellipse inscribed in the given rect, 2×2 supersampled coverage.
pub fn ellipse_mask(x0: f32, y0: f32, x1: f32, y1: f32) -> Mask {
    let (x0, x1) = (x0.min(x1), x0.max(x1));
    let (y0, y1) = (y0.min(y1), y0.max(y1));
    let (cx, cy) = ((x0 + x1) * 0.5, (y0 + y1) * 0.5);
    let (rx, ry) = (((x1 - x0) * 0.5).max(0.01), ((y1 - y0) * 0.5).max(0.01));
    let mut m = Mask::new();
    for y in y0.floor() as i32..y1.ceil() as i32 {
        for x in x0.floor() as i32..x1.ceil() as i32 {
            let mut hits = 0;
            for (sx, sy) in [(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)] {
                let nx = (x as f32 + sx - cx) / rx;
                let ny = (y as f32 + sy - cy) / ry;
                if nx * nx + ny * ny <= 1.0 {
                    hits += 1;
                }
            }
            if hits > 0 {
                m.set(x, y, (hits * 255 / 4) as u8);
            }
        }
    }
    m
}

/// Even-odd scanline fill of a closed polygon (binary coverage — lasso).
pub fn polygon_mask(points: &[[f32; 2]]) -> Mask {
    let mut m = Mask::new();
    if points.len() < 3 {
        return m;
    }
    let y_min = points.iter().map(|p| p[1]).fold(f32::INFINITY, f32::min).floor() as i32;
    let y_max = points.iter().map(|p| p[1]).fold(f32::NEG_INFINITY, f32::max).ceil() as i32;

    for y in y_min..y_max {
        let sy = y as f32 + 0.5;
        let mut xs: Vec<f32> = Vec::new();
        for i in 0..points.len() {
            let a = points[i];
            let b = points[(i + 1) % points.len()];
            if (a[1] <= sy && b[1] > sy) || (b[1] <= sy && a[1] > sy) {
                xs.push(a[0] + (sy - a[1]) / (b[1] - a[1]) * (b[0] - a[0]));
            }
        }
        xs.sort_by(|p, q| p.partial_cmp(q).expect("finite"));
        for pair in xs.chunks_exact(2) {
            for x in pair[0].round() as i32..pair[1].round() as i32 {
                m.set(x, y, 255);
            }
        }
    }
    m
}

/// Marching-squares boundary at threshold 128: unit segments in doc space,
/// suitable for drawing selection "ants". One pass over the mask bounds.
pub fn boundary_segments(mask: &Mask) -> Vec<([f32; 2], [f32; 2])> {
    let Some([bx0, by0, bx1, by1]) = mask.bounds() else { return Vec::new() };
    let inside = |x: i32, y: i32| mask.get(x, y) >= 128;
    let mut out = Vec::new();
    // Cell (x,y) spans pixels (x-1,y-1)..(x,y); emit edges where inside-ness flips.
    for y in by0..=by1 {
        for x in bx0..=bx1 {
            let here = inside(x, y);
            // Left neighbor → vertical edge at x.
            if here != inside(x - 1, y) {
                out.push(([x as f32, y as f32], [x as f32, y as f32 + 1.0]));
            }
            // Top neighbor → horizontal edge at y.
            if here != inside(x, y - 1) {
                out.push(([x as f32, y as f32], [x as f32 + 1.0, y as f32]));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use atelier_core::CombineOp;

    #[test]
    fn rect_mask_full_interior_and_aa_edges() {
        let m = rect_mask(2.0, 2.0, 10.5, 8.0);
        assert_eq!(m.get(5, 5), 255, "interior");
        assert_eq!(m.get(1, 5), 0, "outside");
        assert_eq!(m.get(10, 5), 128, "half-covered right column");
    }

    #[test]
    fn ellipse_center_full_corners_empty() {
        let m = ellipse_mask(0.0, 0.0, 20.0, 10.0);
        assert_eq!(m.get(10, 5), 255, "center");
        assert_eq!(m.get(0, 0), 0, "corner outside ellipse");
        assert_eq!(m.get(19, 9), 0);
        // Edge pixels partially covered somewhere.
        let edge = (0..20).map(|x| m.get(x, 1)).filter(|&v| v > 0 && v < 255).count();
        assert!(edge > 0, "AA edge exists");
    }

    #[test]
    fn polygon_triangle_fills_even_odd() {
        let m = polygon_mask(&[[0.0, 0.0], [20.0, 0.0], [0.0, 20.0]]);
        assert_eq!(m.get(3, 3), 255, "inside triangle");
        assert_eq!(m.get(15, 15), 0, "outside hypotenuse");
        assert!(polygon_mask(&[[0.0, 0.0], [1.0, 1.0]]).is_empty(), "degenerate");
    }

    #[test]
    fn magic_wand_selects_region_not_background() {
        // Left half red, right half blue; wand on the left selects only red.
        let mut tiles = TileMap::new();
        tiles.fill_rect(0, 0, 10, 10, [255, 0, 0, 255]);
        tiles.fill_rect(10, 0, 20, 10, [0, 0, 255, 255]);
        let sel = magic_wand(&tiles, [0, 0], [3, 3], 10, [20, 10]);
        assert_eq!(sel.get(3, 3), 255, "seed region selected");
        assert_eq!(sel.get(9, 5), 255, "rest of red selected");
        assert_eq!(sel.get(15, 5), 0, "blue not selected");
    }

    #[test]
    fn grow_then_shrink_approximates_original() {
        let m = rect_mask(20.0, 20.0, 40.0, 40.0);
        let grown = grow(&m, 2);
        assert_eq!(grown.get(18, 30), 255, "grew left by 2");
        let back = shrink(&grown, 2);
        // Interior identical; edges restored to ~original extent.
        assert_eq!(back.get(30, 30), 255);
        assert_eq!(back.get(18, 30), 0, "shrink undid the grow at the edge");
    }

    #[test]
    fn feather_softens_a_hard_edge() {
        let m = rect_mask(0.0, 0.0, 40.0, 40.0);
        let f = feather(&m, 3);
        // Somewhere near the right edge there is partial coverage.
        let partial = (0..40).any(|y| {
            (35..45).any(|x| {
                let v = f.get(x, y);
                v > 0 && v < 255
            })
        });
        assert!(partial, "feather produced a soft edge");
    }

    #[test]
    fn combine_with_shapes_and_boundary_of_rect() {
        let mut sel = rect_mask(0.0, 0.0, 10.0, 10.0);
        sel.combine(&rect_mask(20.0, 0.0, 30.0, 10.0), CombineOp::Add);
        assert_eq!(sel.get(25, 5), 255);

        let segs = boundary_segments(&rect_mask(1.0, 1.0, 4.0, 3.0));
        // 3×2 px rect → perimeter edges: 2*(3+2) = 10 unit segments.
        assert_eq!(segs.len(), 10);
    }
}
