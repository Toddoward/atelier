# Spec 0004 — Raster engine II: GPU compositor parity + real pixels on canvas

- **Status:** ☑ done (2026-06-12)
- **Phase:** 2 (slice b of 3)
- **Requirements:** RAS-1 (GPU-composited tiles), DOC-3 (blend modes on GPU), SH-2
- **Depends on:** 0003

## Goal
A wgpu compute-shader compositor that reproduces the CPU reference (spec 0003) within
1 LSB (8-bit) — proven by golden tests that run the same documents through both paths on a
real adapter. The canvas stops drawing placeholder rects and shows the actual composited
document (CPU-composited, cached by document revision, uploaded as an egui texture);
the GPU compositor's job this slice is *parity validation* per D-9 — wiring it to the
canvas render path is a later perf slice.

## Scope
- `atelier-raster::ops`: `CompositeOp` list (Layer / Push / Pop) built from the layer tree —
  the single tree-walk both compositors share structurally.
- `atelier-gpu::compositor`: compute pipelines executing an op list per 256² tile —
  packed-RGBA8 tile sources, f32 intermediate buffers, isolation stack, all 28 blend modes
  in WGSL (same W3C formulas; Dissolve uses the identical integer hash), readback API
  `composite_rgba8(device, queue, doc, w, h)`.
- `History::revision` counter (bumps on apply/undo/redo) for cheap recomposite caching.
- `atelier-app` canvas: draws the composited document as a nearest-filtered egui texture in
  the doc rect; selected-layer outline from tile bounds; placeholder rect painting removed
  (`RasterContent.art` stays as the new-layer fill template only).
- `TileMap::bounds()` for selection outlines.
- Golden tests: deterministic pseudo-random documents (layers, nested groups, modes,
  opacities) → CPU vs GPU byte-compare ≤1 LSB. Tests acquire a real adapter and **skip
  with a notice when none exists** (CI runners) — deviation from the `#[ignore]` plan in
  CLAUDE.md, recorded because skip-on-no-adapter runs everywhere it can without manual
  flags. Local dev box (RTX 3060) executes them for real.

## Out of scope
- GPU compositor driving the canvas (perf slice, after Phase 2); dirty-rect incremental
  recomposite; brush/tools (0005); vector layers in compositor (Phase 4); CPU compositor
  refactor onto the op list (kept recursive — golden tests pin the two together).

## Design notes
- Op list: `Layer { tiles, offset?, mode, opacity }` composites a tile source onto the
  stack top; `Push` opens an isolated transparent buffer; `Pop { mode, opacity }` blends
  it onto the previous top. PassThrough(op=1) groups emit children inline (no Push/Pop).
- Two compute pipelines sharing a WGSL blend library: `cs_tile` (source = packed u32 tile
  buffer or absent) and `cs_buffer` (source = f32 stack buffer). Workgroups 16×16.
- Quantization on GPU mirrors CPU exactly: `clamp(c*255+0.5)` then `u8`.
- Readback via staging buffer + `map_async`; per-tile dispatch over the covering grid.

## Verification checklist
- [ ] `cargo test -p atelier-raster` — op-list construction (groups, pass-through, hidden)
- [ ] `cargo test -p atelier-gpu` — golden parity: ≥8 deterministic random documents
      (nested groups, every blend mode covered, fractional opacities) CPU==GPU within
      1 LSB on hardware adapter; skip-with-notice if no adapter
- [ ] `cargo test -p atelier-app` — composite texture cache invalidates on revision bump
- [ ] workspace tests + clippy `--all-targets -D warnings` clean
- [ ] [manual·non-gating] `cargo run`: new doc + add layer shows real composited pixels,
      pan/zoom crisp (nearest)

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-12 | `cargo test -p atelier-raster` (op list) | PASS | 14 tests incl. op-list structure: isolation push/pop, pass-through inlining, hidden-layer skip |
| 2026-06-12 | GPU golden parity | PASS — **bit-exact** | RTX 3060 Laptop GPU (Vulkan): 8 deterministic docs (3 layers + isolated group + nested pass-through, all 27 layer modes cycled, fractional opacities, 64×64 + 300×280 tile-spanning): **0 bytes differ**; Dissolve exact-equality test passes (hash matches bit-for-bit). Exceeds ≤1 LSB gate |
| 2026-06-12 | composite cache invalidation (kittest) | PASS | `composite_cache_follows_history_revision`: cache keyed on History::revision, recomposites on edit/undo, stable when idle |
| 2026-06-12 | workspace + clippy `--all-targets -D warnings` | PASS | 53 tests total, clippy clean |
| 2026-06-12 | [manual·non-gating] real pixels on canvas | smoke-run only | app runs; layer-add visual confirmed headlessly via composite math; eyes-on optional |

## Notes / surprises
- Parity came out bit-exact (not just ≤1 LSB) — identical IEEE f32 ops and the shared
  quantizer (`atelier_raster::quantize_rgba8`) on both paths; no fma divergence observed
  on NVIDIA/Vulkan/naga. Keep watching on other adapters (CI runners skip — no adapter).
- WGSL/Rust must keep `BlendMode::ALL` ordering in sync with the shader's mode indices
  (`mode_index()` derives from ALL; shader switch is hand-numbered — change together).
- Golden tests skip-with-notice when no adapter exists instead of `#[ignore]`
  (CLAUDE.md deviation recorded in spec Scope).
- Canvas texture path is CPU-composited per revision; GPU compositor is parity-validation
  only until the perf slice (post-Phase-2 gate needs it wired for the 60 fps target).
