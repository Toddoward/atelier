//! Selection shape rasterizers and boundary extraction (spec 0007).
//! All coordinates are doc-space pixels; rects are half-open.

use atelier_core::Mask;

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
    fn combine_with_shapes_and_boundary_of_rect() {
        let mut sel = rect_mask(0.0, 0.0, 10.0, 10.0);
        sel.combine(&rect_mask(20.0, 0.0, 30.0, 10.0), CombineOp::Add);
        assert_eq!(sel.get(25, 5), 255);

        let segs = boundary_segments(&rect_mask(1.0, 1.0, 4.0, 3.0));
        // 3×2 px rect → perimeter edges: 2*(3+2) = 10 unit segments.
        assert_eq!(segs.len(), 10);
    }
}
