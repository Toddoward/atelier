# Spec 0036 — Fill selection with color

- **Status:** ☑ done (2026-06-13)
- **Phase:** 2 follow-up (RAS-9 fill — the solid-fill half; gradients/pattern later)
- **Requirements:** RAS-9 (fill)
- **Depends on:** 0007 (selection), 0035 (brush color)

## Goal
Fill the active selection (or the whole layer if nothing is selected) of the selected raster
layer with the current brush color, undoably (Edit → Fill with Color).

## Scope
- `atelier-raster::fill::fill_region(tiles, color, offset, region, mask?)` — straight-alpha
  source-over fill of a doc-space rect, clipped by selection coverage (partial coverage =
  partial alpha), respecting the layer offset (layer-space write).
- App `fill_selection` — region = selection bounds (clamped) or whole doc; captures touched
  tiles, fills, commits one `PaintTiles` (undoable). Edit-menu entry.

## Out of scope
- Flood-fill / magic-wand fill (contiguous color region); gradient & pattern fills (RAS-9
  remainder); fill blend modes / opacity beyond the brush color's alpha; stroke-selection.

## Verification checklist
- [x] `cargo test -p atelier-raster` — fill region unclipped; mask + offset clip; partial
      coverage blends
- [x] `cargo test -p atelier-app` — fill a 4×4 selection red → inside filled, outside empty;
      undo clears
- [x] workspace + clippy `--all-targets -D warnings` clean; smoke run

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-13 | `cargo test -p atelier-raster` | PASS | `fills_region_unclipped`, `fill_respects_mask_and_offset`, `partial_coverage_blends` |
| 2026-06-13 | `cargo test -p atelier-app` | PASS | `fill_selection_with_color_and_undo` (selection filled, outside empty, undo clears); app 39 tests |
| 2026-06-13 | workspace + clippy + smoke | PASS | full suite green, clippy clean, app runs 5s no crash |

## Notes / surprises
- Reuses the `PaintTiles` snapshot command (same undo machinery as brush/adjustment) by
  capturing the touched tile range before filling.
- Coverage-scaled alpha means feathered selections fill with soft edges for free.