# Spec 0007 — Selections I: mask model, marquee/ellipse/lasso, combine ops, ants

- **Status:** ☑ done (2026-06-13)
- **Phase:** 3 (slice a)
- **Requirements:** RAS-3 (rect/ellipse/lasso, add/subtract/intersect, marching ants; wand/feather/grow later slices)
- **Depends on:** 0006

## Goal
A document-level selection: sparse single-channel 8-bit coverage mask with PS-style combine
ops, created by rectangular marquee (AA), elliptical marquee (AA), and freehand lasso
(scanline polygon), driven by canvas drags with Shift=add / Alt=subtract /
Shift+Alt=intersect; selection changes are undoable commands; the canvas shows a
marching-squares boundary outline ("ants", static) plus live drag previews. Ctrl+D deselects.

## Scope
- `atelier-core::mask::Mask` — sparse 256² u8 tiles: get/set/fill, bounds, is_empty,
  `combine(other, CombineOp{Replace,Add,Subtract,Intersect})` (max / min(a,255−b) / min).
- `Document.selection: Option<Arc<Mask>>` — serde-skipped (not persisted this slice),
  mutated only via new `SetSelection` command (old/new Arc snapshots — cheap undo).
- `atelier-raster::selection`: `rect_mask` (AA edges), `ellipse_mask` (2×2 supersampled
  coverage), `polygon_mask` (even-odd scanline, binary), `boundary_segments`
  (marching squares at threshold 128, doc-space unit segments for ants).
- App: tools Select Rect (M), Select Ellipse, Lasso (L) in Tools panel + shortcuts;
  drag interactions with combine modifiers read at release; live preview outlines;
  ants cached per history revision; Edit → Deselect (Ctrl+D).
- kittest: rect-select drag → mask set + undoable; modifier add grows mask;
  Ctrl+D clears; unit tests for shapes/combine/boundary.

## Out of scope
- Selection-clipped painting/adjustments (slice b), magic wand, feather/grow/shrink/
  invert UI, quick mask, animated ants phase, mask persistence in .atl, mask→path (INT-5).

## Design notes
- Mask tiles mirror TileMap's sparse layout (separate type — 1 byte/px, no premul concerns).
- Combine materializes over the union of tile grids then prunes blanks.
- Ants: marching-squares over mask bounds once per selection change (cached by revision),
  drawn as light/dark paired 1-px segments — static dashes; animation is polish debt.
- Lasso points accumulate in doc space during drag; polygon closes on release; degenerate
  (<3 points) drags are ignored.

## Verification checklist
- [ ] `cargo test -p atelier-core` — mask ops + SetSelection apply/revert identity
- [ ] `cargo test -p atelier-raster` — shape masks (known coverages), combine semantics,
      boundary segments of a known rect
- [ ] `cargo test -p atelier-app` — kittest: drag-select sets mask + one undo step;
      Shift-add unions; Ctrl+D deselects; suite stays green
- [ ] workspace + clippy `--all-targets -D warnings` clean; smoke run
- [ ] [manual·non-gating] eyes-on ants/preview feel

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-13 | `cargo test -p atelier-core` | PASS | 23 tests incl. mask get/set/default-zero, combine (Add/Subtract/Intersect/Replace) semantics, subtract-to-empty, SetSelection apply/revert identity + deselect round-trip |
| 2026-06-13 | `cargo test -p atelier-raster` | PASS | 23 tests incl. rect AA edges (half-covered col=128), ellipse center/corner/AA, polygon even-odd triangle + degenerate, combine+boundary (3×2 rect→10 unit segments) |
| 2026-06-13 | `cargo test -p atelier-app` (kittest) | PASS | rect-select drag → selection set + one undo step; Shift-add second marquee (two "Rectangular Select" entries, selection persists); Ctrl+D deselect + undo restores |
| 2026-06-13 | workspace 73 tests + clippy `--all-targets -D warnings` + smoke | PASS | clean |

## Notes / surprises
- kittest coalesces the press+first-move into one frame, so at `drag_started` the live
  pointer already sits at the moved position and `interact_pointer_pos` returns *current*,
  not the press point — start collapsed onto current (zero-area marquee). Fix: take the
  drag start from `pointer.press_origin()` (the button-down location), update `current`
  per drag frame from `interact_pointer_pos`. Recorded for future drag tools.
- Added `AntSegments` type alias (clippy `type_complexity` on the ants cache field).
- Selection not yet persisted in `.atl` (session-only this slice, by scope) and does not
  yet clip painting/adjustments — that's slice b.
