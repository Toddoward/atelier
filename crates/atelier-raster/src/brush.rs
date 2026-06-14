//! Round brush / eraser stamping into a TileMap (spec 0005, RAS-2 core).
//! Coordinates are layer space (caller subtracts the layer offset).

use atelier_core::{Mask, TileCoord, TileMap, TILE_SIZE};

/// Selection clip: a coverage mask in doc space and the painted layer's offset
/// (layer pixel `p` maps to doc pixel `p + offset`).
pub type BrushClip<'a> = (&'a Mask, [i32; 2]);

#[derive(Debug, Clone, Copy)]
pub struct BrushParams {
    pub radius: f32,
    /// 0 = soft (falloff from center), 1 = hard edge.
    pub hardness: f32,
    /// Straight-alpha paint color (ignored by the eraser).
    pub color: [f32; 4],
    pub erase: bool,
}

impl Default for BrushParams {
    fn default() -> Self {
        Self { radius: 16.0, hardness: 0.8, color: [0.0, 0.0, 0.0, 1.0], erase: false }
    }
}

fn stamp_centers(from: [f32; 2], to: [f32; 2], radius: f32) -> Vec<[f32; 2]> {
    let spacing = (radius / 3.0).max(0.75);
    let (dx, dy) = (to[0] - from[0], to[1] - from[1]);
    let dist = (dx * dx + dy * dy).sqrt();
    let steps = (dist / spacing).ceil() as usize;
    (0..=steps)
        .map(|i| {
            let t = if steps == 0 { 0.0 } else { i as f32 / steps as f32 };
            [from[0] + dx * t, from[1] + dy * t]
        })
        .collect()
}

/// Tiles a stroke segment can touch — call BEFORE `stamp_segment` to capture
/// undo state for exactly these coords.
pub fn segment_tiles(from: [f32; 2], to: [f32; 2], radius: f32) -> Vec<TileCoord> {
    let t = TILE_SIZE as i32;
    let r = radius.ceil() as i32 + 1;
    let x0 = (from[0].min(to[0]).floor() as i32 - r).div_euclid(t);
    let y0 = (from[1].min(to[1]).floor() as i32 - r).div_euclid(t);
    let x1 = (from[0].max(to[0]).ceil() as i32 + r).div_euclid(t);
    let y1 = (from[1].max(to[1]).ceil() as i32 + r).div_euclid(t);
    let mut out = Vec::new();
    for ty in y0..=y1 {
        for tx in x0..=x1 {
            out.push((tx, ty));
        }
    }
    out
}

/// Smoothstep coverage: 1 inside `hardness·r`, fading to 0 at `r`.
fn coverage(dist: f32, radius: f32, hardness: f32) -> f32 {
    let inner = radius * hardness.clamp(0.0, 0.99);
    if dist <= inner {
        return 1.0;
    }
    if dist >= radius {
        return 0.0;
    }
    let t = (dist - inner) / (radius - inner);
    1.0 - t * t * (3.0 - 2.0 * t)
}

/// Stamp a stroke segment (inclusive endpoints).
pub fn stamp_segment(tiles: &mut TileMap, from: [f32; 2], to: [f32; 2], p: &BrushParams) {
    stamp_segment_clipped(tiles, from, to, p, None);
}

/// Stamp a stroke segment, optionally clipped by a selection mask.
pub fn stamp_segment_clipped(
    tiles: &mut TileMap,
    from: [f32; 2],
    to: [f32; 2],
    p: &BrushParams,
    clip: Option<BrushClip<'_>>,
) {
    for c in stamp_centers(from, to, p.radius) {
        stamp(tiles, c, p, clip);
    }
}

/// Paint into a layer mask (doc space): brush reveals (raises coverage toward
/// 255), eraser hides (scales coverage down). Spec 0050.
pub fn stamp_mask_segment(
    mask: &mut Mask,
    from: [f32; 2],
    to: [f32; 2],
    radius: f32,
    hardness: f32,
    erase: bool,
) {
    for center in stamp_centers(from, to, radius) {
        let (x0, x1) = ((center[0] - radius).floor() as i32, (center[0] + radius).ceil() as i32);
        let (y0, y1) = ((center[1] - radius).floor() as i32, (center[1] + radius).ceil() as i32);
        for y in y0..=y1 {
            for x in x0..=x1 {
                let (dx, dy) = (x as f32 + 0.5 - center[0], y as f32 + 0.5 - center[1]);
                let cov = coverage((dx * dx + dy * dy).sqrt(), radius, hardness);
                if cov <= 0.0 {
                    continue;
                }
                let cur = mask.get(x, y);
                let nv = if erase {
                    (cur as f32 * (1.0 - cov)) as u8
                } else {
                    cur.max((cov * 255.0) as u8)
                };
                mask.set(x, y, nv);
            }
        }
    }
}

fn stamp(tiles: &mut TileMap, center: [f32; 2], p: &BrushParams, clip: Option<BrushClip<'_>>) {
    let r = p.radius;
    let x0 = (center[0] - r).floor() as i32;
    let x1 = (center[0] + r).ceil() as i32;
    let y0 = (center[1] - r).floor() as i32;
    let y1 = (center[1] + r).ceil() as i32;
    for y in y0..=y1 {
        for x in x0..=x1 {
            let (dx, dy) = (x as f32 + 0.5 - center[0], y as f32 + 0.5 - center[1]);
            let mut cov = coverage((dx * dx + dy * dy).sqrt(), r, p.hardness);
            if cov <= 0.0 {
                continue;
            }
            // Selection clip: scale coverage by mask at the doc pixel.
            if let Some((mask, offset)) = clip {
                let m = mask.get(x + offset[0], y + offset[1]) as f32 / 255.0;
                cov *= m;
                if cov <= 0.0 {
                    continue;
                }
            }
            let dst = tiles.pixel(x, y);
            let out = if p.erase {
                let mut d = dst;
                d[3] = crate::quantize_rgba8((dst[3] as f32 / 255.0) * (1.0 - cov));
                d
            } else {
                src_over(dst, p.color, cov)
            };
            tiles.set_pixel(x, y, out);
        }
    }
}

/// Straight-alpha source-over with coverage-scaled source alpha.
fn src_over(dst: [u8; 4], color: [f32; 4], cov: f32) -> [u8; 4] {
    let sa = color[3] * cov;
    let da = dst[3] as f32 / 255.0;
    let a_out = sa + da * (1.0 - sa);
    if a_out <= 0.0 {
        return [0; 4];
    }
    let mut out = [0u8; 4];
    for c in 0..3 {
        let dc = dst[c] as f32 / 255.0;
        out[c] = crate::quantize_rgba8((sa * color[c] + (1.0 - sa) * da * dc) / a_out);
    }
    out[3] = crate::quantize_rgba8(a_out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stamp_paints_solid_center_and_soft_edge() {
        let mut tiles = TileMap::new();
        let p = BrushParams { radius: 8.0, hardness: 0.5, color: [1.0, 0.0, 0.0, 1.0], erase: false };
        stamp_segment(&mut tiles, [16.0, 16.0], [16.0, 16.0], &p);
        assert_eq!(tiles.pixel(16, 16), [255, 0, 0, 255], "center opaque");
        let edge = tiles.pixel(16 + 6, 16);
        assert!(edge[3] > 0 && edge[3] < 255, "falloff zone partial: {edge:?}");
        assert_eq!(tiles.pixel(40, 16), [0; 4], "outside untouched");
    }

    #[test]
    fn segment_covers_line_and_preflight_tiles_contain_it() {
        let mut tiles = TileMap::new();
        let p = BrushParams { radius: 4.0, ..Default::default() };
        let (from, to) = ([10.0, 10.0], [300.0, 10.0]);
        let pre = segment_tiles(from, to, p.radius);
        stamp_segment(&mut tiles, from, to, &p);
        assert!(tiles.pixel(150, 10)[3] > 0, "midpoint painted");
        for (coord, _) in tiles.tiles() {
            assert!(pre.contains(coord), "preflight missed touched tile {coord:?}");
        }
    }

    #[test]
    fn eraser_clears_alpha() {
        let mut tiles = TileMap::new();
        tiles.fill_rect(0, 0, 32, 32, [0, 255, 0, 255]);
        let p = BrushParams { radius: 6.0, hardness: 0.9, erase: true, ..Default::default() };
        stamp_segment(&mut tiles, [16.0, 16.0], [16.0, 16.0], &p);
        assert_eq!(tiles.pixel(16, 16)[3], 0, "erased center");
        assert_eq!(tiles.pixel(0, 0)[3], 255, "corner untouched");
    }

    #[test]
    fn hard_brush_has_no_fringe_beyond_radius() {
        let mut tiles = TileMap::new();
        let p = BrushParams { radius: 5.0, hardness: 0.99, color: [0.0, 0.0, 1.0, 1.0], erase: false };
        stamp_segment(&mut tiles, [8.0, 8.0], [8.0, 8.0], &p);
        assert_eq!(tiles.pixel(8 + 7, 8), [0; 4]);
        assert_eq!(tiles.pixel(8, 8), [0, 0, 255, 255]);
    }
}
