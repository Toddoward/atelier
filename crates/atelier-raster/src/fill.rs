//! Solid-color fill of a region, optionally clipped by a selection mask
//! (RAS-9 fill, spec 0036). Straight-alpha source-over; coverage scales the
//! source alpha so partial selections fill partially.

use atelier_core::{Mask, TileMap};

fn src_over(dst: [u8; 4], color: [f32; 4], cov: f32) -> [u8; 4] {
    let sa = color[3] * cov;
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
        out[i] = q((color[i] * sa + dc * da * (1.0 - sa)) / ao);
    }
    out[3] = q(ao);
    out
}

/// Fill the doc-space rect `region = [x0,y0,x1,y1)` on `tiles` (drawn at
/// `offset`) with straight-alpha `color`, clipped by `mask` coverage if given.
pub fn fill_region(
    tiles: &mut TileMap,
    color: [f32; 4],
    offset: [i32; 2],
    region: [i32; 4],
    mask: Option<&Mask>,
) {
    for dy in region[1]..region[3] {
        for dx in region[0]..region[2] {
            let cov = mask.map_or(255u8, |m| m.get(dx, dy));
            if cov == 0 {
                continue;
            }
            let (lx, ly) = (dx - offset[0], dy - offset[1]);
            let out = src_over(tiles.pixel(lx, ly), color, cov as f32 / 255.0);
            tiles.set_pixel(lx, ly, out);
        }
    }
}

/// Fill `region` with a two-stop linear gradient from `c0` at `p0` to `c1` at
/// `p1` (doc space), clipped by `mask`, respecting the layer `offset`.
/// Straight-alpha source-over; `t` is the projection onto the p0→p1 axis.
#[allow(clippy::too_many_arguments)]
pub fn gradient_region(
    tiles: &mut TileMap,
    c0: [f32; 4],
    c1: [f32; 4],
    p0: [f32; 2],
    p1: [f32; 2],
    offset: [i32; 2],
    region: [i32; 4],
    mask: Option<&Mask>,
) {
    let (dx, dy) = (p1[0] - p0[0], p1[1] - p0[1]);
    let len2 = dx * dx + dy * dy;
    for y in region[1]..region[3] {
        for x in region[0]..region[2] {
            let cov = mask.map_or(255u8, |m| m.get(x, y));
            if cov == 0 {
                continue;
            }
            let t = if len2 <= 1e-6 {
                0.0
            } else {
                (((x as f32 + 0.5 - p0[0]) * dx + (y as f32 + 0.5 - p0[1]) * dy) / len2)
                    .clamp(0.0, 1.0)
            };
            let col = [
                c0[0] + (c1[0] - c0[0]) * t,
                c0[1] + (c1[1] - c0[1]) * t,
                c0[2] + (c1[2] - c0[2]) * t,
                c0[3] + (c1[3] - c0[3]) * t,
            ];
            let (lx, ly) = (x - offset[0], y - offset[1]);
            let out = src_over(tiles.pixel(lx, ly), col, cov as f32 / 255.0);
            tiles.set_pixel(lx, ly, out);
        }
    }
}

/// Like [`gradient_region`] but radial: `c0` at `center` (= p0) fading to `c1`
/// at radius `|p1 − p0|`.
#[allow(clippy::too_many_arguments)]
pub fn gradient_region_radial(
    tiles: &mut TileMap,
    c0: [f32; 4],
    c1: [f32; 4],
    center: [f32; 2],
    edge: [f32; 2],
    offset: [i32; 2],
    region: [i32; 4],
    mask: Option<&Mask>,
) {
    let radius = {
        let (dx, dy) = (edge[0] - center[0], edge[1] - center[1]);
        (dx * dx + dy * dy).sqrt().max(1e-3)
    };
    for y in region[1]..region[3] {
        for x in region[0]..region[2] {
            let cov = mask.map_or(255u8, |m| m.get(x, y));
            if cov == 0 {
                continue;
            }
            let (dx, dy) = (x as f32 + 0.5 - center[0], y as f32 + 0.5 - center[1]);
            let t = ((dx * dx + dy * dy).sqrt() / radius).clamp(0.0, 1.0);
            let col = [
                c0[0] + (c1[0] - c0[0]) * t,
                c0[1] + (c1[1] - c0[1]) * t,
                c0[2] + (c1[2] - c0[2]) * t,
                c0[3] + (c1[3] - c0[3]) * t,
            ];
            let (lx, ly) = (x - offset[0], y - offset[1]);
            let out = src_over(tiles.pixel(lx, ly), col, cov as f32 / 255.0);
            tiles.set_pixel(lx, ly, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn radial_gradient_is_brightest_at_center() {
        let mut t = TileMap::new();
        gradient_region_radial(
            &mut t,
            [1.0, 0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0, 0.0],
            [5.0, 5.0],
            [15.0, 5.0], // radius 10
            [0, 0],
            [0, 0, 20, 20],
            None,
        );
        let center = t.pixel(5, 5)[3];
        let mid = t.pixel(10, 5)[3];
        let edge = t.pixel(14, 5)[3];
        assert!(center > 230 && mid < center && edge < mid, "{center} {mid} {edge}");
    }

    #[test]
    fn fills_region_unclipped() {
        let mut t = TileMap::new();
        fill_region(&mut t, [1.0, 0.0, 0.0, 1.0], [0, 0], [0, 0, 4, 4], None);
        assert_eq!(t.pixel(2, 2), [255, 0, 0, 255]);
        assert_eq!(t.pixel(5, 5), [0, 0, 0, 0], "outside region untouched");
    }

    #[test]
    fn fill_respects_mask_and_offset() {
        let mut t = TileMap::new();
        let mut m = Mask::new();
        // Select the left two doc columns only.
        for y in 0..4 {
            for x in 0..2 {
                m.set(x, y, 255);
            }
        }
        // Layer drawn at offset (10,0): doc pixel (0,0) → layer (-10,0).
        fill_region(&mut t, [0.0, 0.0, 1.0, 1.0], [10, 0], [0, 0, 4, 4], Some(&m));
        assert_eq!(t.pixel(-10, 0), [0, 0, 255, 255], "filled inside selection (layer space)");
        assert_eq!(t.pixel(-8, 0), [0, 0, 0, 0], "outside selection not filled");
    }

    #[test]
    fn gradient_interpolates_along_axis() {
        let mut t = TileMap::new();
        // Red opaque at x=0 → red transparent at x=10, horizontal.
        gradient_region(
            &mut t,
            [1.0, 0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 0.0],
            [10.0, 0.0],
            [0, 0],
            [0, 0, 10, 1],
            None,
        );
        let a0 = t.pixel(0, 0)[3];
        let a9 = t.pixel(9, 0)[3];
        assert!(a0 > 230, "start ~opaque: {a0}");
        assert!(a9 < 60, "end ~transparent: {a9}");
        assert!(a0 > t.pixel(5, 0)[3] && t.pixel(5, 0)[3] > a9, "monotonic falloff");
    }

    #[test]
    fn partial_coverage_blends() {
        let mut t = TileMap::new();
        let mut m = Mask::new();
        m.set(0, 0, 128); // half-selected
        fill_region(&mut t, [1.0, 1.0, 1.0, 1.0], [0, 0], [0, 0, 1, 1], Some(&m));
        let p = t.pixel(0, 0);
        assert!((p[3] as i32 - 128).abs() <= 1, "half coverage → ~50% alpha: {p:?}");
    }
}
