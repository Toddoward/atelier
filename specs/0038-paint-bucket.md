# Spec 0038 — Paint bucket (flood fill)

- **Status:** ☑ done (2026-06-13)
- **Phase:** 2 follow-up (RAS-9 flood fill — completes the fill set)
- **Requirements:** RAS-9 (fill bucket)
- **Depends on:** 0011 (magic_wand), 0036 (fill)

## Goal
A Paint Bucket tool (key `K`): click to flood-fill the contiguous same-color region under the
cursor on the active raster layer with the brush color, undoably.

## Scope
- App `apply_bucket(state, seed)` — flood-select via `atelier_raster::selection::magic_wand`
  (brush tolerance) on the active layer, then `fill_region` over the mask bounds with the
  brush color; capture tiles, commit one `PaintTiles`. `ActiveTool::Bucket` + Tools-panel
  entry + `K`. Composes existing pieces — no new kernel.

## Out of scope
- Global (non-contiguous) fill; fill-all-layers sampling; gap tolerance / anti-aliased flood
  edges; fill blend modes beyond the brush alpha.

## Verification checklist
- [x] `cargo test -p atelier-app` — bucket-fill a red square → recolored blue (whole
      contiguous region), outside untouched; undo restores
- [x] workspace + clippy `--all-targets -D warnings` clean; smoke run

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-13 | `cargo test -p atelier-app` | PASS | `paint_bucket_flood_fills_and_undoes` (red square → blue across the contiguous region, outside untouched, undo restores); app 41 tests |
| 2026-06-13 | workspace + clippy + smoke | PASS | full suite green, clippy clean, app runs 5s no crash |

## Notes / surprises
- Pure composition: `magic_wand` (selection) + `fill_region` (fill) + `PaintTiles` (undo) —
  the bucket added no new geometry/raster kernel, just wiring.
- Flood is contiguous + same-layer (matches `magic_wand`'s semantics); a "sample all layers"
  variant would composite first, like the eyedropper.