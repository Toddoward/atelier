# Spec 0029 — cross-layer align & distribute (multi-object VEC-6)

- **Status:** ☑ done (2026-06-13)
- **Phase:** 4 (completes VEC-6: multi-object align/distribute; builds on 0028 multi-select)
- **Requirements:** VEC-6 (align/distribute across objects)
- **Depends on:** 0028

## Goal
With multiple layers selected, align them to each other (L/C/R/T/M/B vs the union of their
content bounds) and distribute them evenly by center (H/V) — one undo step for the whole
operation. Works on raster (moves the layer offset) and vector (translates shapes) layers.

## Scope
- `atelier-core::command::Batch` — apply a list of commands as one history entry (apply in
  order, revert in reverse).
- `TileMap::content_bounds` — pixel-exact (non-zero-alpha) bounds, so sub-tile content aligns
  precisely (the existing tile-granular `bounds` is too coarse for alignment).
- App `panels::align_layers` / `distribute_layers` — compute each selected raster/vector
  layer's doc bounds, build a per-layer translate command (raster→`SetOffset`,
  vector→`SetVectorShapes`), and apply them together as a `Batch`. Layers-panel "Align
  layers" / "Distribute" controls appear when ≥2 (≥3 for distribute) nodes are selected.

## Out of scope
- Aligning/distributing groups as units (only raster/vector leaves move); align-to-key-object
  or to canvas (canvas-align is spec 0022); distribute by spacing/gaps.

## Verification checklist
- [x] `cargo test -p atelier-core` — Batch applies in order / reverts in reverse;
      content_bounds is pixel-exact
- [x] `cargo test -p atelier-app` — align two raster layers left → equal left edge; single
      undo (the Batch) restores both
- [x] workspace + clippy `--all-targets -D warnings` clean; smoke run

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-13 | `cargo test -p atelier-core` | PASS | `batch_applies_in_order_and_reverts_in_reverse`; core 40 tests |
| 2026-06-13 | `cargo test -p atelier-app` | PASS | `cross_layer_align_left_and_undo` (two raster layers → equal content-left; one undo restores both via Batch); app 33 tests |
| 2026-06-13 | workspace + clippy + smoke | PASS | full suite green, clippy clean, app runs 5s no crash |

## Notes / surprises
- Needed pixel-exact `content_bounds` — the tile-granular `bounds` snaps to 256 px, which made
  sub-tile layers "already aligned". `content_bounds` scans alpha (O(pixels)); fine at these
  sizes, optimize later if it shows up in profiles.
- `Batch` makes multi-command operations one undo step — reusable for any future compound edit.
- Group layers are skipped (only raster/vector leaves translate); moving a whole group needs
  a recursive translate, deferred.