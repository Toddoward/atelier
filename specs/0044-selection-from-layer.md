# Spec 0044 — Selection from layer (alpha → mask)

- **Status:** ☑ done (2026-06-13)
- **Phase:** 5 (INT-5 — vector→selection / raster alpha→selection)
- **Requirements:** INT-5 (mask ↔ path/layer)
- **Depends on:** 0007 (selection), 0023 (rasterize_vector)

## Goal
Build a document selection from the selected layer's alpha (Select → From Layer): raster
layers use their tiles' alpha; vector layers are rasterized first. Undoable.

## Scope
- App `selection_from_layer` — raster: per-doc-pixel alpha from the layer's tiles (offset
  aware); vector: `rasterize_vector` then take alpha; build a `Mask` and apply `SetSelection`.
  Select-menu entry (enabled for raster/vector layers).

## Out of scope
- Mask → path (the other INT-5 direction); intersect/add/subtract the layer mask with the
  existing selection (replaces for now); threshold control.

## Verification checklist
- [x] `cargo test -p atelier-app` — raster alpha square → selection covers it (outside not);
      undo deselects; vector rect → selection covers the shape
- [x] workspace + clippy `--all-targets -D warnings` clean; smoke run

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-13 | `cargo test -p atelier-app` | PASS | `selection_from_layer_raster_and_vector` (raster square in/out + undo; vector rect inside selected); app 47 tests |
| 2026-06-13 | workspace + clippy + smoke | PASS | full suite green, clippy clean, app runs 5s no crash |

## Notes / surprises
- Reuses `rasterize_vector` (AA) for the vector path, so a vector layer's selection is
  anti-aliased coverage.
- The reverse direction (selection outline → vector path) and combine-with-existing are the
  remaining INT-5 work.