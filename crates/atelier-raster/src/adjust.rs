//! Destructive per-pixel image adjustments (spec 0008, RAS-6 core set).
//! Operate on straight-alpha RGBA8; alpha is preserved. Each adjustment is a
//! `fn([u8;4]) -> [u8;4]` color map; `apply_tile` runs one over a tile with an
//! optional selection-coverage clip (partial coverage = partial blend).

use atelier_core::{Mask, Tile, TileMap, TILE_SIZE};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Adjustment {
    Invert,
    /// brightness in [-1,1], contrast in [-1,1].
    BrightnessContrast { brightness: f32, contrast: f32 },
    /// black/white points in [0,1], gamma > 0.
    Levels { black: f32, white: f32, gamma: f32 },
    /// hue degrees [-180,180], saturation/lightness [-1,1].
    HueSaturation { hue: f32, sat: f32, light: f32 },
}

impl Adjustment {
    pub fn label(self) -> &'static str {
        match self {
            Adjustment::Invert => "Invert",
            Adjustment::BrightnessContrast { .. } => "Brightness/Contrast",
            Adjustment::Levels { .. } => "Levels",
            Adjustment::HueSaturation { .. } => "Hue/Saturation",
        }
    }

    /// Map one straight-alpha pixel (alpha untouched).
    pub fn map_pixel(self, px: [u8; 4]) -> [u8; 4] {
        let a = px[3];
        let mut c = [px[0] as f32 / 255.0, px[1] as f32 / 255.0, px[2] as f32 / 255.0];
        match self {
            Adjustment::Invert => {
                for ch in &mut c {
                    *ch = 1.0 - *ch;
                }
            }
            Adjustment::BrightnessContrast { brightness, contrast } => {
                let k = contrast + 1.0; // -1..1 -> 0..2 slope
                for ch in &mut c {
                    *ch = (k * (*ch - 0.5) + 0.5 + brightness).clamp(0.0, 1.0);
                }
            }
            Adjustment::Levels { black, white, gamma } => {
                let span = (white - black).max(1e-3);
                let inv_g = 1.0 / gamma.max(1e-3);
                for ch in &mut c {
                    let n = ((*ch - black) / span).clamp(0.0, 1.0);
                    *ch = n.powf(inv_g);
                }
            }
            Adjustment::HueSaturation { hue, sat, light } => {
                let (mut h, mut s, mut l) = rgb_to_hsl(c);
                h = (h + hue / 360.0).rem_euclid(1.0);
                s = (s * (1.0 + sat)).clamp(0.0, 1.0);
                l = (l + light * 0.5).clamp(0.0, 1.0);
                c = hsl_to_rgb(h, s, l);
            }
        }
        [
            (c[0] * 255.0 + 0.5) as u8,
            (c[1] * 255.0 + 0.5) as u8,
            (c[2] * 255.0 + 0.5) as u8,
            a,
        ]
    }
}

fn rgb_to_hsl(c: [f32; 3]) -> (f32, f32, f32) {
    let max = c[0].max(c[1]).max(c[2]);
    let min = c[0].min(c[1]).min(c[2]);
    let l = (max + min) * 0.5;
    if (max - min).abs() < 1e-6 {
        return (0.0, 0.0, l);
    }
    let d = max - min;
    let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };
    let h = if max == c[0] {
        ((c[1] - c[2]) / d).rem_euclid(6.0)
    } else if max == c[1] {
        (c[2] - c[0]) / d + 2.0
    } else {
        (c[0] - c[1]) / d + 4.0
    };
    (h / 6.0, s, l)
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> [f32; 3] {
    if s < 1e-6 {
        return [l, l, l];
    }
    let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
    let p = 2.0 * l - q;
    [hue(p, q, h + 1.0 / 3.0), hue(p, q, h), hue(p, q, h - 1.0 / 3.0)]
}

fn hue(p: f32, q: f32, t: f32) -> f32 {
    let t = t.rem_euclid(1.0);
    if t < 1.0 / 6.0 {
        p + (q - p) * 6.0 * t
    } else if t < 0.5 {
        q
    } else if t < 2.0 / 3.0 {
        p + (q - p) * (2.0 / 3.0 - t) * 6.0
    } else {
        p
    }
}

/// Apply `adj` to one tile at doc-tile `(tx,ty)`, layer drawn at `offset`,
/// optionally clipped by `mask` (selection coverage). `coverage/255` blends the
/// adjusted color toward the original so partial selections apply partially.
pub fn apply_tile(
    tile: &mut Tile,
    adj: Adjustment,
    tx: i32,
    ty: i32,
    offset: [i32; 2],
    mask: Option<&Mask>,
) {
    let t = TILE_SIZE as i32;
    for y in 0..TILE_SIZE {
        for x in 0..TILE_SIZE {
            let orig = tile.pixel(x, y);
            if orig[3] == 0 {
                continue;
            }
            let cov = match mask {
                None => 255u8,
                Some(m) => {
                    let dx = tx * t + x as i32 + offset[0];
                    let dy = ty * t + y as i32 + offset[1];
                    m.get(dx, dy)
                }
            };
            if cov == 0 {
                continue;
            }
            let adjusted = adj.map_pixel(orig);
            let out = if cov == 255 {
                adjusted
            } else {
                let f = cov as f32 / 255.0;
                let mut o = [0u8; 4];
                for i in 0..3 {
                    o[i] = (orig[i] as f32 * (1.0 - f) + adjusted[i] as f32 * f + 0.5) as u8;
                }
                o[3] = orig[3];
                o
            };
            tile.set_pixel(x, y, out);
        }
    }
}

/// Tiles to apply over: those intersecting `bounds` (doc px) if given, else all.
pub fn target_tiles(tiles: &TileMap, bounds: Option<[i32; 4]>, offset: [i32; 2]) -> Vec<(i32, i32)> {
    let t = TILE_SIZE as i32;
    tiles
        .tiles()
        .map(|(c, _)| *c)
        .filter(|&(tx, ty)| match bounds {
            None => true,
            Some([bx0, by0, bx1, by1]) => {
                // Tile's doc-space span (drawn at offset).
                let x0 = tx * t + offset[0];
                let y0 = ty * t + offset[1];
                x0 < bx1 && x0 + t > bx0 && y0 < by1 && y0 + t > by0
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invert_is_involution() {
        let p = [10, 200, 130, 255];
        let once = Adjustment::Invert.map_pixel(p);
        assert_eq!(once, [245, 55, 125, 255]);
        assert_eq!(Adjustment::Invert.map_pixel(once), p, "invert twice = original");
    }

    #[test]
    fn brightness_contrast_known_values() {
        // contrast 0, brightness +0.5 lifts mid-gray.
        let p = [128, 128, 128, 255];
        let b = Adjustment::BrightnessContrast { brightness: 0.5, contrast: 0.0 }.map_pixel(p);
        assert!(b[0] > 200, "brightened: {b:?}");
        // full negative contrast collapses toward 0.5.
        let c = Adjustment::BrightnessContrast { brightness: 0.0, contrast: -1.0 }.map_pixel([0, 0, 0, 255]);
        assert!((c[0] as i32 - 128).abs() <= 1, "flattened to mid: {c:?}");
    }

    #[test]
    fn levels_clamps_and_maps_endpoints() {
        let adj = Adjustment::Levels { black: 0.25, white: 0.75, gamma: 1.0 };
        assert_eq!(adj.map_pixel([0, 0, 0, 255])[0], 0, "below black → 0");
        assert_eq!(adj.map_pixel([255, 255, 255, 255])[0], 255, "above white → 255");
        let mid = adj.map_pixel([128, 128, 128, 255])[0];
        assert!((mid as i32 - 128).abs() <= 3, "mid maps near mid: {mid}");
    }

    #[test]
    fn hue_saturation_zero_is_identity() {
        let p = [200, 100, 50, 255];
        let out = Adjustment::HueSaturation { hue: 0.0, sat: 0.0, light: 0.0 }.map_pixel(p);
        for i in 0..3 {
            assert!((out[i] as i32 - p[i] as i32).abs() <= 2, "channel {i}: {out:?} vs {p:?}");
        }
        // -1 saturation → gray (r≈g≈b).
        let gray = Adjustment::HueSaturation { hue: 0.0, sat: -1.0, light: 0.0 }.map_pixel(p);
        assert!((gray[0] as i32 - gray[1] as i32).abs() <= 2 && (gray[1] as i32 - gray[2] as i32).abs() <= 2);
    }

    #[test]
    fn apply_tile_clips_to_mask() {
        let mut tile = Tile::default();
        for y in 0..TILE_SIZE {
            for x in 0..TILE_SIZE {
                tile.set_pixel(x, y, [10, 20, 30, 255]);
            }
        }
        let mut mask = Mask::new();
        // Select only the left half of this tile (doc == layer, offset 0).
        for y in 0..TILE_SIZE as i32 {
            for x in 0..128 {
                mask.set(x, y, 255);
            }
        }
        apply_tile(&mut tile, Adjustment::Invert, 0, 0, [0, 0], Some(&mask));
        assert_eq!(tile.pixel(10, 10), [245, 235, 225, 255], "inside selection inverted");
        assert_eq!(tile.pixel(200, 10), [10, 20, 30, 255], "outside selection untouched");
    }
}
