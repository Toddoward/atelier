//! Per-pixel blend functions `B(cb, cs)` for every [`BlendMode`] (DOC-3).
//!
//! Source of truth for compositing math (D-9): formulas follow the W3C
//! "Compositing and Blending Level 1" spec (same model Photoshop uses for its
//! separable modes) plus the customary definitions for the PS-only modes
//! (Linear/Vivid/Pin Light, Hard Mix, Subtract, Divide, Darker/Lighter Color).
//! All channels are f32 in [0,1], straight alpha; blending is alpha-agnostic —
//! alpha handling lives in the compositor.
//!
//! `Dissolve` and `PassThrough` are not per-pixel color functions; the
//! compositor special-cases them and must not call `blend_rgb` with them.

use atelier_core::BlendMode;

#[inline]
fn lum(c: [f32; 3]) -> f32 {
    0.3 * c[0] + 0.59 * c[1] + 0.11 * c[2]
}

fn clip_color(c: [f32; 3]) -> [f32; 3] {
    let l = lum(c);
    let n = c[0].min(c[1]).min(c[2]);
    let x = c[0].max(c[1]).max(c[2]);
    let mut out = c;
    if n < 0.0 {
        for ch in &mut out {
            *ch = l + (*ch - l) * l / (l - n);
        }
    }
    if x > 1.0 {
        for ch in &mut out {
            *ch = l + (*ch - l) * (1.0 - l) / (x - l);
        }
    }
    out
}

fn set_lum(c: [f32; 3], l: f32) -> [f32; 3] {
    let d = l - lum(c);
    clip_color([c[0] + d, c[1] + d, c[2] + d])
}

fn sat(c: [f32; 3]) -> f32 {
    c[0].max(c[1]).max(c[2]) - c[0].min(c[1]).min(c[2])
}

/// W3C SetSat: scale the mid channel between min and max to match `s`.
fn set_sat(c: [f32; 3], s: f32) -> [f32; 3] {
    // Index ordering of (min, mid, max).
    let mut idx = [0usize, 1, 2];
    idx.sort_by(|&a, &b| c[a].partial_cmp(&c[b]).expect("finite"));
    let (i_min, i_mid, i_max) = (idx[0], idx[1], idx[2]);
    let mut out = [0.0; 3];
    if c[i_max] > c[i_min] {
        out[i_mid] = (c[i_mid] - c[i_min]) * s / (c[i_max] - c[i_min]);
        out[i_max] = s;
    }
    out
}

#[inline]
fn soft_light_d(x: f32) -> f32 {
    if x <= 0.25 {
        ((16.0 * x - 12.0) * x + 4.0) * x
    } else {
        x.sqrt()
    }
}

#[inline]
fn color_burn(cb: f32, cs: f32) -> f32 {
    if cb >= 1.0 {
        1.0
    } else if cs <= 0.0 {
        0.0
    } else {
        1.0 - ((1.0 - cb) / cs).min(1.0)
    }
}

#[inline]
fn color_dodge(cb: f32, cs: f32) -> f32 {
    if cb <= 0.0 {
        0.0
    } else if cs >= 1.0 {
        1.0
    } else {
        (cb / (1.0 - cs)).min(1.0)
    }
}

#[inline]
fn hard_light(cb: f32, cs: f32) -> f32 {
    if cs <= 0.5 {
        cb * 2.0 * cs
    } else {
        // screen(cb, 2cs-1)
        let cs2 = 2.0 * cs - 1.0;
        cb + cs2 - cb * cs2
    }
}

/// Separable per-channel blend.
#[inline]
fn separable(mode: BlendMode, cb: f32, cs: f32) -> f32 {
    match mode {
        BlendMode::Normal => cs,
        BlendMode::Darken => cb.min(cs),
        BlendMode::Multiply => cb * cs,
        BlendMode::ColorBurn => color_burn(cb, cs),
        BlendMode::LinearBurn => (cb + cs - 1.0).clamp(0.0, 1.0),
        BlendMode::Lighten => cb.max(cs),
        BlendMode::Screen => cb + cs - cb * cs,
        BlendMode::ColorDodge => color_dodge(cb, cs),
        BlendMode::LinearDodge => (cb + cs).min(1.0),
        BlendMode::Overlay => hard_light(cs, cb),
        BlendMode::SoftLight => {
            if cs <= 0.5 {
                cb - (1.0 - 2.0 * cs) * cb * (1.0 - cb)
            } else {
                cb + (2.0 * cs - 1.0) * (soft_light_d(cb) - cb)
            }
        }
        BlendMode::HardLight => hard_light(cb, cs),
        BlendMode::VividLight => {
            if cs <= 0.5 {
                color_burn(cb, 2.0 * cs)
            } else {
                color_dodge(cb, 2.0 * cs - 1.0)
            }
        }
        BlendMode::LinearLight => (cb + 2.0 * cs - 1.0).clamp(0.0, 1.0),
        BlendMode::PinLight => {
            if cs <= 0.5 {
                cb.min(2.0 * cs)
            } else {
                cb.max(2.0 * cs - 1.0)
            }
        }
        BlendMode::HardMix => {
            if cb + cs >= 1.0 {
                1.0
            } else {
                0.0
            }
        }
        BlendMode::Difference => (cb - cs).abs(),
        BlendMode::Exclusion => cb + cs - 2.0 * cb * cs,
        BlendMode::Subtract => (cb - cs).max(0.0),
        BlendMode::Divide => {
            if cs <= 0.0 {
                1.0
            } else {
                (cb / cs).min(1.0)
            }
        }
        _ => unreachable!("non-separable mode routed through blend_rgb"),
    }
}

/// Blend source color over backdrop color for `mode` (color only — no alpha).
///
/// Panics in debug for `Dissolve`/`PassThrough`, which have no per-pixel color
/// function (the compositor handles them structurally).
pub fn blend_rgb(mode: BlendMode, cb: [f32; 3], cs: [f32; 3]) -> [f32; 3] {
    match mode {
        BlendMode::Hue => set_lum(set_sat(cs, sat(cb)), lum(cb)),
        BlendMode::Saturation => set_lum(set_sat(cb, sat(cs)), lum(cb)),
        BlendMode::Color => set_lum(cs, lum(cb)),
        BlendMode::Luminosity => set_lum(cb, lum(cs)),
        BlendMode::DarkerColor => {
            if lum(cs) < lum(cb) {
                cs
            } else {
                cb
            }
        }
        BlendMode::LighterColor => {
            if lum(cs) > lum(cb) {
                cs
            } else {
                cb
            }
        }
        BlendMode::Dissolve | BlendMode::PassThrough => {
            debug_assert!(false, "{mode:?} has no per-pixel blend function");
            cs
        }
        _ => [
            separable(mode, cb[0], cs[0]),
            separable(mode, cb[1], cs[1]),
            separable(mode, cb[2], cs[2]),
        ],
    }
}

/// Deterministic per-pixel threshold for Dissolve (stable across runs and
/// platforms so golden tests don't flake).
pub fn dissolve_keeps(x: i32, y: i32, alpha: f32) -> bool {
    let mut h = (x as u32).wrapping_mul(0x9E37_79B9) ^ (y as u32).wrapping_mul(0x85EB_CA6B);
    h ^= h >> 16;
    h = h.wrapping_mul(0x45D9_F3B5);
    h ^= h >> 16;
    (h as f32 / u32::MAX as f32) < alpha
}

#[cfg(test)]
mod tests {
    use super::*;
    use atelier_core::BlendMode as M;

    fn close(a: [f32; 3], b: [f32; 3]) -> bool {
        a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-5)
    }

    /// Hand-computed values from the W3C compositing spec formulas.
    #[test]
    fn separable_modes_match_hand_computed_values() {
        let cb = [0.5, 0.25, 1.0];
        let cs = [0.5, 0.8, 0.0];
        assert!(close(blend_rgb(M::Normal, cb, cs), cs));
        assert!(close(blend_rgb(M::Multiply, cb, cs), [0.25, 0.2, 0.0]));
        assert!(close(blend_rgb(M::Screen, cb, cs), [0.75, 0.85, 1.0]));
        // Overlay = HardLight(cs, cb): cb=0.5→2·0.5·0.5=0.5; cb=0.25→2·0.25·0.8=0.4;
        // cb=1.0→screen(0.0, 1.0)=1.0
        assert!(close(blend_rgb(M::Overlay, cb, cs), [0.5, 0.4, 1.0]));
        assert!(close(blend_rgb(M::Darken, cb, cs), [0.5, 0.25, 0.0]));
        assert!(close(blend_rgb(M::Lighten, cb, cs), [0.5, 0.8, 1.0]));
        assert!(close(blend_rgb(M::Difference, cb, cs), [0.0, 0.55, 1.0]));
        assert!(close(blend_rgb(M::Exclusion, cb, cs), [0.5, 0.65, 1.0]));
        assert!(close(blend_rgb(M::LinearDodge, cb, cs), [1.0, 1.0, 1.0]));
        assert!(close(blend_rgb(M::LinearBurn, cb, cs), [0.0, 0.05, 0.0]));
        assert!(close(blend_rgb(M::Subtract, cb, cs), [0.0, 0.0, 1.0]));
        // ColorDodge: 0.5/(1-0.5)=1; 0.25/0.2=1.25→1; cs=0→cb=1.0
        assert!(close(blend_rgb(M::ColorDodge, cb, cs), [1.0, 1.0, 1.0]));
        // ColorBurn: 1-(0.5/0.5)=0; 1-min(1,0.75/0.8)=0.0625; cs=0,cb=1→1
        assert!(close(blend_rgb(M::ColorBurn, cb, cs), [0.0, 0.0625, 1.0]));
    }

    #[test]
    fn soft_light_spec_values() {
        // cs=0.5 is identity in W3C soft-light.
        let cb = [0.3, 0.5, 0.9];
        assert!(close(blend_rgb(M::SoftLight, cb, [0.5; 3]), cb));
        // cs=1, cb=0.25: D(0.25)=((16·0.25-12)·0.25+4)·0.25=0.5; 0.25+(1)·(0.5-0.25)=0.5
        assert!(close(blend_rgb(M::SoftLight, [0.25; 3], [1.0; 3]), [0.5; 3]));
    }

    #[test]
    fn nonseparable_modes_preserve_their_invariants() {
        let cb = [0.7, 0.2, 0.4];
        let cs = [0.1, 0.9, 0.3];
        // Luminosity: result takes lum from cs, Color keeps lum of cb.
        assert!((lum(blend_rgb(M::Luminosity, cb, cs)) - lum(cs)).abs() < 1e-4);
        assert!((lum(blend_rgb(M::Color, cb, cs)) - lum(cb)).abs() < 1e-4);
        assert!((lum(blend_rgb(M::Hue, cb, cs)) - lum(cb)).abs() < 1e-4);
        // DarkerColor picks the lower-luminosity input wholesale.
        let dk = blend_rgb(M::DarkerColor, cb, cs);
        assert!(dk == cb || dk == cs);
    }

    #[test]
    fn all_color_modes_stay_finite_and_in_range() {
        let grid = [0.0, 0.001, 0.25, 0.5, 0.75, 0.999, 1.0];
        for mode in M::ALL {
            if matches!(mode, M::Dissolve | M::PassThrough) {
                continue;
            }
            for &b in &grid {
                for &s in &grid {
                    let out = blend_rgb(mode, [b; 3], [s; 3]);
                    for ch in out {
                        assert!(
                            ch.is_finite() && (-1e-6..=1.0 + 1e-6).contains(&ch),
                            "{mode:?} B({b},{s}) -> {ch}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn dissolve_threshold_is_deterministic_and_alpha_monotone() {
        assert_eq!(dissolve_keeps(3, 7, 0.5), dissolve_keeps(3, 7, 0.5));
        assert!(!dissolve_keeps(3, 7, 0.0));
        assert!(dissolve_keeps(3, 7, 1.1)); // alpha 1 keeps everything
        // Roughly half of pixels survive at alpha 0.5.
        let kept = (0..100)
            .flat_map(|x| (0..100).map(move |y| (x, y)))
            .filter(|&(x, y)| dissolve_keeps(x, y, 0.5))
            .count();
        assert!((3000..7000).contains(&kept), "kept {kept} of 10000");
    }
}
