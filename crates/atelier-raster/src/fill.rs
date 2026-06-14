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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn partial_coverage_blends() {
        let mut t = TileMap::new();
        let mut m = Mask::new();
        m.set(0, 0, 128); // half-selected
        fill_region(&mut t, [1.0, 1.0, 1.0, 1.0], [0, 0], [0, 0, 1, 1], Some(&m));
        let p = t.pixel(0, 0);
        assert!((p[3] as i32 - 128).abs() <= 1, "half coverage → ~50% alpha: {p:?}");
    }
}
