//! Tonal/color adjustment parameters and the pure per-pixel map (spec 0008/0009).
//! Lives in core so `NodeKind::Adjustment` can hold it and both the destructive
//! path (`atelier-raster::adjust`) and adjustment layers (compositor) share it.
//!
//! These are tonal adjustments in sRGB-component space, not ICC conversions
//! (those stay in `atelier-color`, Phase 6).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
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

    /// Blend toward the mapped color by `amount` (0..=1) — adjustment-layer
    /// opacity. Alpha untouched.
    pub fn map_pixel_amount(self, px: [u8; 4], amount: f32) -> [u8; 4] {
        if amount >= 1.0 {
            return self.map_pixel(px);
        }
        let m = self.map_pixel(px);
        let f = amount.clamp(0.0, 1.0);
        [
            (px[0] as f32 * (1.0 - f) + m[0] as f32 * f + 0.5) as u8,
            (px[1] as f32 * (1.0 - f) + m[1] as f32 * f + 0.5) as u8,
            (px[2] as f32 * (1.0 - f) + m[2] as f32 * f + 0.5) as u8,
            px[3],
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
        let p = [128, 128, 128, 255];
        let b = Adjustment::BrightnessContrast { brightness: 0.5, contrast: 0.0 }.map_pixel(p);
        assert!(b[0] > 200, "brightened: {b:?}");
        let c = Adjustment::BrightnessContrast { brightness: 0.0, contrast: -1.0 }
            .map_pixel([0, 0, 0, 255]);
        assert!((c[0] as i32 - 128).abs() <= 1, "flattened to mid: {c:?}");
    }

    #[test]
    fn levels_clamps_and_maps_endpoints() {
        let adj = Adjustment::Levels { black: 0.25, white: 0.75, gamma: 1.0 };
        assert_eq!(adj.map_pixel([0, 0, 0, 255])[0], 0);
        assert_eq!(adj.map_pixel([255, 255, 255, 255])[0], 255);
        let mid = adj.map_pixel([128, 128, 128, 255])[0];
        assert!((mid as i32 - 128).abs() <= 3, "mid maps near mid: {mid}");
    }

    #[test]
    fn hue_saturation_zero_is_identity_and_desaturates() {
        let p = [200, 100, 50, 255];
        let out = Adjustment::HueSaturation { hue: 0.0, sat: 0.0, light: 0.0 }.map_pixel(p);
        for i in 0..3 {
            assert!((out[i] as i32 - p[i] as i32).abs() <= 2);
        }
        let gray = Adjustment::HueSaturation { hue: 0.0, sat: -1.0, light: 0.0 }.map_pixel(p);
        assert!(
            (gray[0] as i32 - gray[1] as i32).abs() <= 2
                && (gray[1] as i32 - gray[2] as i32).abs() <= 2
        );
    }

    #[test]
    fn map_pixel_amount_lerps() {
        let p = [0, 0, 0, 255];
        // Invert at amount 0.5 → mid-gray.
        let half = Adjustment::Invert.map_pixel_amount(p, 0.5);
        assert!((half[0] as i32 - 128).abs() <= 1, "{half:?}");
        assert_eq!(Adjustment::Invert.map_pixel_amount(p, 0.0), p, "amount 0 = identity");
    }
}
