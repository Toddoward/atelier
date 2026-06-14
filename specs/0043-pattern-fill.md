# Spec 0043 — Define pattern & pattern fill

- **Status:** ☑ done (2026-06-13)
- **Phase:** 2 follow-up (RAS-9 patterns — completes the fill set)
- **Requirements:** RAS-9 (patterns)
- **Depends on:** 0036 (fill), 0007 (selection)

## Goal
Define a fill pattern from the selected layer's content (or selection) and tile-fill the
selection/layer with it, undoably (Edit → Define Pattern, Edit → Fill with Pattern).

## Scope
- `atelier-raster::fill::fill_pattern(tiles, pat, pw, ph, offset, region, mask?)` — tile a
  straight-alpha RGBA pattern anchored to doc origin, clipped + offset aware, source-over.
- `Mask::tight_bounds()` — pixel-exact selection extent (the coarse `bounds()` is tile-
  granular, which had captured a 256-wide pattern).
- App: `EditorState.pattern: Option<DecodedImage>`; `define_pattern` copies the selected
  raster layer's pixels over the tight selection/content bounds; `fill_with_pattern` tiles it
  over the selection/layer (undoable `PaintTiles`). Edit-menu entries.

## Out of scope
- Pattern library/swatches, scale/rotation/offset of the pattern, seamless-tile assist
  (3D-4), pattern from the full composite.

## Verification checklist
- [x] `cargo test -p atelier-raster` — pattern tiles with wrap
- [x] `cargo test -p atelier-app` — define a 2×1 red/green pattern, fill a 4-wide selection →
      red,green,red,green; undo clears
- [x] workspace + clippy `--all-targets -D warnings` clean; smoke run

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-13 | `cargo test -p atelier-raster` | PASS | `pattern_tiles_with_wrap` (period-2 wrap) |
| 2026-06-13 | `cargo test -p atelier-app` | PASS | `define_and_fill_with_pattern` (tiled red/green, undo clears); app 46 tests |
| 2026-06-13 | workspace + clippy + smoke | PASS | full suite green, clippy clean, app runs 5s no crash |

## Notes / surprises
- `Mask::bounds()` is tile-granular (256-aligned) — the recurring trap; pattern definition
  needed the new `tight_bounds()` for a pixel-exact source region.
- Pattern is anchored to doc origin so adjacent fills tile seamlessly.