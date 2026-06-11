// Checkerboard canvas background, stable in document space under pan/zoom.
//
// `doc = (framebuffer_px - offset_px) / scale_px`; checker cell size is in
// document pixels so the pattern zooms with the document.

struct CheckerUniform {
    // x, y: offset in framebuffer pixels; z: scale (fb px per doc px); w: cell size (doc px)
    transform: vec4<f32>,
    light: vec4<f32>,
    dark: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: CheckerUniform;

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> @builtin(position) vec4<f32> {
    // Fullscreen triangle; egui clamps the pass to the canvas rect via viewport/scissor.
    let x = f32(i32(vi & 1u) * 4 - 1);
    let y = f32(i32(vi >> 1u) * 4 - 1);
    return vec4<f32>(x, y, 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let doc = (pos.xy - u.transform.xy) / u.transform.z;
    let cell = floor(doc / u.transform.w);
    let parity = (i32(cell.x) + i32(cell.y)) & 1;
    return select(u.light, u.dark, parity == 1);
}
