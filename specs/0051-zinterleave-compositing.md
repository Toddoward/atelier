# Spec 0051 — Z-interleaved raster+vector compositing

- **Status:** ☑ done (2026-06-14)
- **Phase:** 5 (Focus modes & interop) — also hardens Phase 2 compositor
- **Requirements:** RAS-1 (CPU compositor), VEC-7 (vector render), supports INT-2/3/4
- **Depends on:** 0046 (clip/group compositing), vector engine (Phase 4)

## Goal
Vector layers composite through the CPU compositor in correct tree z-order, interleaved
with raster layers — so a vector layer above a raster covers it and a raster above a vector
covers the vector. The single document composite (`composite_rgba8`) is now the one source of
truth for what's on screen and for export/flatten/merge, which previously **dropped** vector
layers entirely.

## Scope
- `atelier-raster::compositor::composite_node` — add a `NodeKind::Vector` arm: rasterize the
  vector content to doc space via `rasterize_vector` (up to the region's far edge) and
  `blend_onto` the backdrop with the layer's blend/opacity, in tree order.
- App: remove the separate `paint_vector_layers` egui overlay and the `vector_cache` field +
  its tessellation cache (canvas now shows vectors straight from the document composite
  texture). Vectors therefore appear in export, flatten, and merge with no extra code.

## Out of scope
- Resolution-independent vector rendering: vectors now rasterize at **document** resolution
  into the composite texture, so they don't re-sharpen when zoomed past 100%. Crisp-at-zoom
  re-rasterization (render at view scale, or a GPU tessellated overlay layered on the
  composite) is a follow-up.
- GPU-path vector compositing (CPU compositor only here; the GPU compositor parity gap for
  vectors stays tracked under R-13).

## Verification checklist
- [x] `cargo test -p atelier-raster` — a vector layer interleaves in z-order (raster above
      covers it; it covers a raster below)
- [x] `cargo test -p atelier-app` — a vector layer appears in `composite_rgba8` (inside the
      shape opaque, outside transparent)
- [x] `cargo build/test --workspace`; clippy `--all-targets -D warnings` clean; smoke run

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-14 | `cargo test -p atelier-raster` | PASS | `vector_layer_interleaves_in_z_order` (red raster over / blue raster under a green vector) |
| 2026-06-14 | `cargo test -p atelier-app` | PASS | `vector_layer_composites_inline` (rect green inside, transparent outside); app 51 tests |
| 2026-06-14 | workspace + clippy + smoke | PASS | full suite green (core 42, raster 47, io 15, gpu 4+2, app 51), clippy clean, app alive 12s no crash |

## Notes / surprises
- This fixes a latent data-loss bug: export/flatten/merge silently omitted vector layers
  because they were only ever drawn by the egui overlay, never composited. Routing vectors
  through `composite_node` closes that and removes a whole duplicate render path.
- Trade-off accepted: composite-resolution vectors (see Out of scope). Recorded so the
  crisp-zoom follow-up isn't forgotten.
