# Spec 0023 — Interop I: rasterize vector layer

- **Status:** ☑ done (2026-06-13)
- **Phase:** 5 groundwork (INT-2) — landed early since the vector engine is ready
- **Requirements:** INT-2 (rasterize vector layer)
- **Depends on:** 0013 (tessellation), 0022

## Goal
Convert the selected vector layer into a raster layer: its filled shapes are tessellated and
scan-filled into 256² tiles at document resolution, replacing the node's content in place,
undoably. First raster↔vector interop.

## Scope
- `atelier-raster::raster_vector::rasterize_vector(content, w, h) -> TileMap` — tessellate
  each filled shape (`atelier_vector::tessellate`) and scan-fill its triangles (edge-function
  point-in-triangle at pixel centers; winding-agnostic). No AA this slice.
- `atelier-core::command::ReplaceNodeKind` — swap a node's `kind` wholesale (undoable;
  props/children/parent untouched).
- App: Layer → Rasterize Layer (enabled when the selection is a vector layer) →
  `rasterize_vector` → replace kind with `Raster(RasterContent { tiles, .. })`.

## Out of scope
- Anti-aliased rasterization (pixel-center coverage only); stroke rasterization (fills only);
  rasterizing at a chosen resolution / DPI; rasterizing groups; the reverse (raster→vector
  trace, VEC-9, a later AI/CV slice).

## Verification checklist
- [x] `cargo test -p atelier-raster` — filled rect rasterizes (inside set, outside clear);
      unfilled shape produces nothing
- [x] `cargo test -p atelier-app` — Rasterize Layer turns a vector layer into a raster layer
      with pixels; undo restores the vector layer
- [x] workspace + clippy `--all-targets -D warnings` clean; smoke run

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-13 | `cargo test -p atelier-raster` | PASS | `rasterizes_a_filled_rect` (interior red, exterior clear), `unfilled_shape_produces_nothing` |
| 2026-06-13 | `cargo test -p atelier-app` | PASS | `rasterize_vector_layer_and_undo` (Vector→Raster w/ pixels, undo→Vector); app 28 tests |
| 2026-06-13 | workspace + clippy + smoke | PASS | full suite green, clippy clean, app runs 5s no crash |

## Notes / surprises
- No AA yet — edges are hard. Supersampled or coverage-based rasterization is a follow-up.
- `ReplaceNodeKind` is a reusable generic command (will also serve convert-to-smart-object
  etc. later).
- Rasterizes at document size from origin; offset/placed rasterization can come with the
  smart-object / place work in Phase 5.