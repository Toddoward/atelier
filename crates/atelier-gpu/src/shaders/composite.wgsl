// Tile compositor: composites one source (packed-RGBA8 tile or f32 buffer)
// onto a f32 stack buffer with the full blend-mode set. Formulas mirror
// atelier-raster::blend (W3C compositing) — the CPU path is the source of
// truth; golden tests enforce ≤1 LSB divergence after quantization.

struct Params {
    mode: u32,          // index into BlendMode::ALL
    opacity: f32,
    tile_origin: vec2i, // doc coords of this tile's (0,0) — for Dissolve
};

@group(0) @binding(0) var<storage, read_write> dst: array<vec4f>;
@group(0) @binding(2) var<uniform> params: Params;
// cs_tile source: packed RGBA8, one u32 per pixel (little-endian r,g,b,a).
@group(0) @binding(1) var<storage, read> src_tile: array<u32>;
// cs_buffer source: straight-alpha f32 (an isolated group buffer).
@group(0) @binding(3) var<storage, read> src_buf: array<vec4f>;

const TILE: u32 = 256u;

// ---- blend library ----------------------------------------------------------

fn lum(c: vec3f) -> f32 {
    return 0.3 * c.x + 0.59 * c.y + 0.11 * c.z;
}

fn clip_color(c_in: vec3f) -> vec3f {
    var c = c_in;
    let l = lum(c);
    let n = min(c.x, min(c.y, c.z));
    let x = max(c.x, max(c.y, c.z));
    if n < 0.0 {
        c = vec3f(l) + (c - vec3f(l)) * l / (l - n);
    }
    if x > 1.0 {
        c = vec3f(l) + (c - vec3f(l)) * (1.0 - l) / (x - l);
    }
    return c;
}

fn set_lum(c: vec3f, l: f32) -> vec3f {
    return clip_color(c + vec3f(l - lum(c)));
}

fn sat3(c: vec3f) -> f32 {
    return max(c.x, max(c.y, c.z)) - min(c.x, min(c.y, c.z));
}

fn set_sat(c: vec3f, s: f32) -> vec3f {
    // Stable 3-element bubble sort of indices (matches the CPU sort exactly).
    var idx = vec3u(0u, 1u, 2u);
    if c[idx.y] < c[idx.x] { let t = idx.x; idx.x = idx.y; idx.y = t; }
    if c[idx.z] < c[idx.y] { let t = idx.y; idx.y = idx.z; idx.z = t; }
    if c[idx.y] < c[idx.x] { let t = idx.x; idx.x = idx.y; idx.y = t; }
    var out = vec3f(0.0);
    if c[idx.z] > c[idx.x] {
        out[idx.y] = (c[idx.y] - c[idx.x]) * s / (c[idx.z] - c[idx.x]);
        out[idx.z] = s;
    }
    return out;
}

fn soft_light_d(x: f32) -> f32 {
    if x <= 0.25 {
        return ((16.0 * x - 12.0) * x + 4.0) * x;
    }
    return sqrt(x);
}

fn color_burn(cb: f32, cs: f32) -> f32 {
    if cb >= 1.0 { return 1.0; }
    if cs <= 0.0 { return 0.0; }
    return 1.0 - min((1.0 - cb) / cs, 1.0);
}

fn color_dodge(cb: f32, cs: f32) -> f32 {
    if cb <= 0.0 { return 0.0; }
    if cs >= 1.0 { return 1.0; }
    return min(cb / (1.0 - cs), 1.0);
}

fn hard_light_c(cb: f32, cs: f32) -> f32 {
    if cs <= 0.5 {
        return cb * 2.0 * cs;
    }
    let cs2 = 2.0 * cs - 1.0;
    return cb + cs2 - cb * cs2;
}

fn separable(mode: u32, cb: f32, cs: f32) -> f32 {
    switch mode {
        case 1u: { return cs; }                                  // Normal
        case 3u: { return min(cb, cs); }                         // Darken
        case 4u: { return cb * cs; }                             // Multiply
        case 5u: { return color_burn(cb, cs); }                  // ColorBurn
        case 6u: { return clamp(cb + cs - 1.0, 0.0, 1.0); }     // LinearBurn
        case 8u: { return max(cb, cs); }                         // Lighten
        case 9u: { return cb + cs - cb * cs; }                   // Screen
        case 10u: { return color_dodge(cb, cs); }                // ColorDodge
        case 11u: { return min(cb + cs, 1.0); }                  // LinearDodge
        case 13u: { return hard_light_c(cs, cb); }               // Overlay
        case 14u: {                                              // SoftLight
            if cs <= 0.5 {
                return cb - (1.0 - 2.0 * cs) * cb * (1.0 - cb);
            }
            return cb + (2.0 * cs - 1.0) * (soft_light_d(cb) - cb);
        }
        case 15u: { return hard_light_c(cb, cs); }               // HardLight
        case 16u: {                                              // VividLight
            if cs <= 0.5 {
                return color_burn(cb, 2.0 * cs);
            }
            return color_dodge(cb, 2.0 * cs - 1.0);
        }
        case 17u: { return clamp(cb + 2.0 * cs - 1.0, 0.0, 1.0); } // LinearLight
        case 18u: {                                              // PinLight
            if cs <= 0.5 {
                return min(cb, 2.0 * cs);
            }
            return max(cb, 2.0 * cs - 1.0);
        }
        case 19u: {                                              // HardMix
            if cb + cs >= 1.0 { return 1.0; }
            return 0.0;
        }
        case 20u: { return abs(cb - cs); }                       // Difference
        case 21u: { return cb + cs - 2.0 * cb * cs; }            // Exclusion
        case 22u: { return max(cb - cs, 0.0); }                  // Subtract
        case 23u: {                                              // Divide
            if cs <= 0.0 { return 1.0; }
            return min(cb / cs, 1.0);
        }
        default: { return cs; }
    }
}

fn blend_rgb(mode: u32, cb: vec3f, cs: vec3f) -> vec3f {
    switch mode {
        case 24u: { return set_lum(set_sat(cs, sat3(cb)), lum(cb)); }  // Hue
        case 25u: { return set_lum(set_sat(cb, sat3(cs)), lum(cb)); }  // Saturation
        case 26u: { return set_lum(cs, lum(cb)); }                     // Color
        case 27u: { return set_lum(cb, lum(cs)); }                     // Luminosity
        case 7u: {                                                     // DarkerColor
            if lum(cs) < lum(cb) { return cs; }
            return cb;
        }
        case 12u: {                                                    // LighterColor
            if lum(cs) > lum(cb) { return cs; }
            return cb;
        }
        default: {
            return vec3f(
                separable(mode, cb.x, cs.x),
                separable(mode, cb.y, cs.y),
                separable(mode, cb.z, cs.z),
            );
        }
    }
}

// Deterministic Dissolve threshold — bit-identical to the CPU hash.
fn dissolve_keeps(x: i32, y: i32, alpha: f32) -> bool {
    var h = u32(x) * 0x9E3779B9u ^ u32(y) * 0x85EBCA6Bu;
    h ^= h >> 16u;
    h = h * 0x45D9F3B5u;
    h ^= h >> 16u;
    return f32(h) / f32(0xFFFFFFFFu) < alpha;
}

// ---- compositing core -------------------------------------------------------

fn composite_px(i: u32, gx: u32, gy: u32, s_in: vec4f) {
    var mode = params.mode;
    var s_rgb = s_in.rgb;
    var s_a = s_in.a * params.opacity;

    if mode == 2u { // Dissolve → all-or-nothing Normal
        let dx = params.tile_origin.x + i32(gx);
        let dy = params.tile_origin.y + i32(gy);
        if s_in.a > 0.0 && dissolve_keeps(dx, dy, s_a) {
            s_a = 1.0;
        } else {
            s_a = 0.0;
        }
        mode = 1u;
    }
    if s_a <= 0.0 {
        return;
    }
    if mode == 0u { // PassThrough fallback (group at opacity < 1)
        mode = 1u;
    }

    let b = dst[i];
    let b_rgb = b.rgb;
    let b_a = b.a;

    var blended: vec3f;
    if mode == 1u {
        blended = s_rgb;
    } else {
        blended = blend_rgb(mode, b_rgb, s_rgb);
    }

    let a_out = s_a + b_a * (1.0 - s_a);
    let c_out = (s_a * (1.0 - b_a) * s_rgb + s_a * b_a * blended + (1.0 - s_a) * b_a * b_rgb)
        / a_out;
    dst[i] = vec4f(c_out, a_out);
}

@compute @workgroup_size(16, 16)
fn cs_tile(@builtin(global_invocation_id) gid: vec3u) {
    let i = gid.y * TILE + gid.x;
    composite_px(i, gid.x, gid.y, unpack4x8unorm(src_tile[i]));
}

@compute @workgroup_size(16, 16)
fn cs_buffer(@builtin(global_invocation_id) gid: vec3u) {
    let i = gid.y * TILE + gid.x;
    composite_px(i, gid.x, gid.y, src_buf[i]);
}
