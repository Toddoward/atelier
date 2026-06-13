# Spec 0010 — Transform, crop, image resample

- **Status:** ☑ done (2026-06-13)
- **Phase:** 3 (slice d; moved from Phase 2 per D-12)
- **Requirements:** RAS-4 (transform: scale/rotate), RAS-5 (crop, image resample)
- **Depends on:** 0009

## Goal
Resample-based editing: a layer can be scaled/rotated (numeric Transform dialog, baked into
its tiles), the canvas can be cropped to the current selection bounds, and the whole image
can be resampled to a new pixel size. All undoable; bilinear sampling for quality.

## Scope
- `atelier-raster::resample`: `sample_bilinear(&TileMap, x, y) -> [u8;4]` (straight-alpha,
  transparent outside content); `transform_layer(&TileMap, offset, Affine) -> (TileMap, offset)`
  baking an affine (about a pivot) into a fresh tile set with a recomputed offset;
  `resample_layer(&TileMap, offset, scale) -> (TileMap, offset)` for whole-doc resize.
- `kurbo::Affine` for the transform math (already a planned dep — add to atelier-raster).
- Commands (atelier-core::command): `TransformLayer` (before tiles+offset → after
  tiles+offset, undoable; built post-bake like PaintTiles but also restores offset),
  `ImageResample` (new size + per-layer tile/offset snapshots), reuse `CanvasResize` for
  crop dimensions plus a layer-offset shift.
- App: Layer → Transform… dialog (scale X/Y %, rotate °) baking the selected raster layer;
  Image → Crop to Selection (uses `doc.selection` bounds; no-op note when no selection);
  Image → Image Size… dialog (resample all raster layers + set doc size).
- Tests: bilinear known values; identity transform ≈ source; 2× scale doubles content
  bbox; 90° rotate maps corner; crop shifts offsets + resizes; resample halves size;
  command apply/revert identity; kittest: transform dialog changes layer, undo restores.

## Out of scope
- Interactive on-canvas transform handles (numeric dialog only this slice — note for later);
  skew/perspective/warp; content-aware scale; crop *tool* with drag rectangle (crop-to-
  selection covers RAS-5 gate); resample interpolation choice UI (bilinear fixed).

## Design notes
- **D-13**: transforms are destructive bakes (new tiles), captured as before/after snapshots
  for undo — avoids per-layer affine state in the compositor and keeps GPU parity unaffected
  (compositor still sees plain tiles+offset). Quality loss on repeated transforms accepted
  (PS "Smart Object" non-destructive transform is a Phase-10 concern).
- Bake: compute the transformed content bbox, allocate tiles covering it, for each dest
  pixel inverse-map through the affine and bilinear-sample the source; new offset = bbox min.
- Crop to selection: new size = selection bbox size; shift every raster layer offset by
  −bbox.min; doc size set; one compound undo step.

## Verification checklist
- [ ] `cargo test -p atelier-raster` — bilinear, identity, 2× scale bbox, rotate, resample
- [ ] `cargo test -p atelier-core` — TransformLayer/ImageResample apply/revert identity
- [ ] `cargo test -p atelier-app` — kittest: transform dialog bakes + undo; crop-to-
      selection resizes; suite stays green
- [ ] workspace + clippy `--all-targets -D warnings` clean; smoke run

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-13 | `cargo test -p atelier-raster` | PASS | bilinear midpoint average, identity preserves content, 2× scale ~doubles bbox, 90° rotate turns a wide bar tall, resample-half scales offset + content |
| 2026-06-13 | `cargo test -p atelier-core` | PASS | ReplaceLayerTiles / CropCanvas / ResizeImage apply-revert identity; CropCanvas shifts offsets + resizes; Mask::pixel_bounds exact (not tile-granular) |
| 2026-06-13 | `cargo test -p atelier-app` (kittest) | PASS | Transform dialog 2× scale widens layer + undo restores; Crop-to-selection resizes doc + undo (crop+deselect) restores |
| 2026-06-13 | workspace + clippy `--all-targets -D warnings` + smoke | PASS | clippy clean; full suite green (see GPU note) |

## Notes / surprises
- Implemented affine inline (2×2 + translate, with inverse) instead of pulling in `kurbo`
  — fewer deps for simple scale/rotate; revisit if skew/perspective arrive.
- Bilinear samples in premultiplied alpha then un-premultiplies, so transformed edges blend
  cleanly against transparency.
- **Caught a real bug:** crop-to-selection initially used `Mask::bounds()` (tile-granular,
  256-aligned) → cropped to 256px for a tiny selection. Added `Mask::pixel_bounds()`
  (per-pixel exact) and used it for crop.
- **Second real bug, caught by CI not local:** `transform_layer` pivoted about
  `TileMap::bounds()` (tile-granular, 256-aligned), so a small layer rotated about the tile
  grid center instead of its content center — flinging content far away. Passed locally by
  FP luck, failed on CI (subtract-with-overflow when the test scan found no pixels). Added
  `TileMap::pixel_bounds()` (pixel-exact) and pivot/bake about it. Lesson reinforced: never
  use tile-granular `bounds()` where pixel-exact extent is meant.
- GPU golden parity occasionally flakes locally under back-to-back full-workspace runs
  (NVIDIA/Vulkan device-churn validation error); passes isolated and in most runs. Added a
  process-wide `GPU_LOCK` to serialize the two golden tests. CI is unaffected (software
  adapter → golden tests skip). Not a code defect in the compositor.
