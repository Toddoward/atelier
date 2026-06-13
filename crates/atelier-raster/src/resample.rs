//! Bilinear resampling and destructive affine bakes for layer transform,
//! crop, and image resize (spec 0010, D-13). All in straight-alpha RGBA8.

use atelier_core::TileMap;

/// Bilinear sample of a tile map at fractional layer coords. Outside stored
/// content reads transparent. Premultiplies by alpha for correct edge blends,
/// then un-premultiplies the result.
pub fn sample_bilinear(tiles: &TileMap, x: f32, y: f32) -> [u8; 4] {
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let fx = x - x0 as f32;
    let fy = y - y0 as f32;

    let mut acc = [0.0f32; 4]; // premultiplied rgb + alpha
    for (dx, wx) in [(0, 1.0 - fx), (1, fx)] {
        for (dy, wy) in [(0, 1.0 - fy), (1, fy)] {
            let w = wx * wy;
            if w <= 0.0 {
                continue;
            }
            let p = tiles.pixel(x0 + dx, y0 + dy);
            let a = p[3] as f32 / 255.0;
            acc[0] += p[0] as f32 / 255.0 * a * w;
            acc[1] += p[1] as f32 / 255.0 * a * w;
            acc[2] += p[2] as f32 / 255.0 * a * w;
            acc[3] += a * w;
        }
    }
    if acc[3] <= 1e-6 {
        return [0, 0, 0, 0];
    }
    let q = crate::quantize_rgba8;
    [q(acc[0] / acc[3]), q(acc[1] / acc[3]), q(acc[2] / acc[3]), q(acc[3])]
}

/// 2×2 linear map plus translation, mapping source layer coords → dest.
#[derive(Clone, Copy)]
struct Affine {
    m: [f32; 4], // [m00, m01, m10, m11]
    t: [f32; 2],
}

impl Affine {
    /// Map `M` about `pivot`: q = M·(p − pivot) + pivot.
    fn about(m: [f32; 4], pivot: [f32; 2]) -> Self {
        let t = [
            pivot[0] - (m[0] * pivot[0] + m[1] * pivot[1]),
            pivot[1] - (m[2] * pivot[0] + m[3] * pivot[1]),
        ];
        Self { m, t }
    }
    fn apply(&self, p: [f32; 2]) -> [f32; 2] {
        [
            self.m[0] * p[0] + self.m[1] * p[1] + self.t[0],
            self.m[2] * p[0] + self.m[3] * p[1] + self.t[1],
        ]
    }
    fn inverse(&self) -> Affine {
        let [a, b, c, d] = self.m;
        let det = a * d - b * c;
        let inv = if det.abs() < 1e-9 { 0.0 } else { 1.0 / det };
        let m = [d * inv, -b * inv, -c * inv, a * inv];
        // inverse translation: p = M⁻¹·(q − t)
        let t = [-(m[0] * self.t[0] + m[1] * self.t[1]), -(m[2] * self.t[0] + m[3] * self.t[1])];
        Affine { m, t }
    }
}

/// Bake a 2×2 map `m` about `pivot` (layer-space) into a fresh tile set.
/// Offset is unchanged by the caller — content stays at the same doc position.
fn bake(tiles: &TileMap, m: [f32; 4], pivot: [f32; 2]) -> TileMap {
    let mut out = TileMap::new();
    let Some([bx0, by0, bx1, by1]) = tiles.bounds() else {
        return out;
    };
    let fwd = Affine::about(m, pivot);
    // Dest bbox = transform of source bbox corners.
    let corners = [
        fwd.apply([bx0 as f32, by0 as f32]),
        fwd.apply([bx1 as f32, by0 as f32]),
        fwd.apply([bx0 as f32, by1 as f32]),
        fwd.apply([bx1 as f32, by1 as f32]),
    ];
    let dx0 = corners.iter().map(|c| c[0]).fold(f32::INFINITY, f32::min).floor() as i32;
    let dy0 = corners.iter().map(|c| c[1]).fold(f32::INFINITY, f32::min).floor() as i32;
    let dx1 = corners.iter().map(|c| c[0]).fold(f32::NEG_INFINITY, f32::max).ceil() as i32;
    let dy1 = corners.iter().map(|c| c[1]).fold(f32::NEG_INFINITY, f32::max).ceil() as i32;

    let inv = fwd.inverse();
    for dy in dy0..dy1 {
        for dx in dx0..dx1 {
            let s = inv.apply([dx as f32 + 0.5, dy as f32 + 0.5]);
            let px = sample_bilinear(tiles, s[0] - 0.5, s[1] - 0.5);
            if px[3] != 0 {
                out.set_pixel(dx, dy, px);
            }
        }
    }
    out
}

/// Scale (sx,sy) then rotate (radians) the layer about its content center.
/// Returns new tiles (offset unchanged by the caller).
pub fn transform_layer(tiles: &TileMap, sx: f32, sy: f32, rot: f32) -> TileMap {
    let Some([bx0, by0, bx1, by1]) = tiles.bounds() else {
        return TileMap::new();
    };
    let pivot = [(bx0 + bx1) as f32 * 0.5, (by0 + by1) as f32 * 0.5];
    let (c, s) = (rot.cos(), rot.sin());
    // M = R·S
    let m = [c * sx, -s * sy, s * sx, c * sy];
    bake(tiles, m, pivot)
}

/// Uniformly resample a layer about the layer origin (image resize). Returns
/// the new tiles and the scaled offset.
pub fn resample_layer(tiles: &TileMap, offset: [i32; 2], scale: f32) -> (TileMap, [i32; 2]) {
    let baked = bake(tiles, [scale, 0.0, 0.0, scale], [0.0, 0.0]);
    let new_offset =
        [(offset[0] as f32 * scale).round() as i32, (offset[1] as f32 * scale).round() as i32];
    (baked, new_offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filled(x0: i32, y0: i32, x1: i32, y1: i32, color: [u8; 4]) -> TileMap {
        let mut t = TileMap::new();
        t.fill_rect(x0, y0, x1, y1, color);
        t
    }

    #[test]
    fn bilinear_midpoint_averages() {
        let mut t = TileMap::new();
        t.set_pixel(0, 0, [0, 0, 0, 255]);
        t.set_pixel(1, 0, [255, 255, 255, 255]);
        // Halfway between the two opaque pixels.
        let mid = sample_bilinear(&t, 0.5, 0.0);
        assert!((mid[0] as i32 - 128).abs() <= 2, "{mid:?}");
        assert_eq!(mid[3], 255);
    }

    #[test]
    fn identity_transform_preserves_content() {
        let t = filled(10, 10, 40, 40, [200, 50, 25, 255]);
        let out = transform_layer(&t, 1.0, 1.0, 0.0);
        // Interior pixel survives unchanged.
        assert_eq!(out.pixel(25, 25), [200, 50, 25, 255]);
        assert_eq!(out.pixel(100, 100), [0, 0, 0, 0]);
    }

    #[test]
    fn scale_2x_grows_content_bbox() {
        let t = filled(0, 0, 20, 20, [255, 255, 255, 255]); // 20×20 about center (10,10)
        let out = transform_layer(&t, 2.0, 2.0, 0.0);
        let [x0, y0, x1, y1] = out.bounds().expect("content");
        // 20px wide → ~40px wide (tile-granular bounds, so check content span).
        let mut min_x = i32::MAX;
        let mut max_x = i32::MIN;
        for y in y0..y1 {
            for x in x0..x1 {
                if out.pixel(x, y)[3] > 0 {
                    min_x = min_x.min(x);
                    max_x = max_x.max(x);
                }
            }
        }
        let w = max_x - min_x + 1;
        assert!((38..=42).contains(&w), "scaled width {w}");
    }

    #[test]
    fn rotate_90_maps_a_horizontal_bar_to_vertical() {
        // Wide short bar about its center → tall narrow bar.
        let t = filled(0, 10, 40, 14, [0, 255, 0, 255]); // 40 wide, 4 tall
        let out = transform_layer(&t, 1.0, 1.0, std::f32::consts::FRAC_PI_2);
        let mut min_x = i32::MAX;
        let mut max_x = i32::MIN;
        let mut min_y = i32::MAX;
        let mut max_y = i32::MIN;
        for y in -10..40 {
            for x in -10..50 {
                if out.pixel(x, y)[3] > 0 {
                    min_x = min_x.min(x);
                    max_x = max_x.max(x);
                    min_y = min_y.min(y);
                    max_y = max_y.max(y);
                }
            }
        }
        let w = max_x - min_x + 1;
        let h = max_y - min_y + 1;
        assert!(h > w, "rotated bar is now taller than wide: {w}x{h}");
    }

    #[test]
    fn resample_half_scales_offset() {
        let t = filled(0, 0, 40, 40, [255, 0, 0, 255]);
        let (out, off) = resample_layer(&t, [100, 60], 0.5);
        assert_eq!(off, [50, 30], "offset scaled");
        // Content roughly halved.
        let [x0, y0, x1, y1] = out.bounds().expect("content");
        let mut max_x = i32::MIN;
        for y in y0..y1 {
            for x in x0..x1 {
                if out.pixel(x, y)[3] > 0 {
                    max_x = max_x.max(x);
                }
            }
        }
        assert!((18..=22).contains(&(max_x + 1)), "halved width ~20: {}", max_x + 1);
    }
}
