# Spec 0047 — Layer masks

- **Status:** ☑ done (2026-06-13)
- **Phase:** 2/3 (DOC-4 layer masks)
- **Requirements:** DOC-4 (layer masks)
- **Depends on:** 0006 (compositor), 0007 (selection)

## Goal
A raster layer can carry a mask (doc-space coverage) that multiplies its alpha during
compositing. Add a mask from the current selection or remove it, undoably.

## Scope
- `RasterContent.mask: Option<Mask>` (serde-skip — session-only for now). Compositor's
  `TileSource` multiplies sampled alpha by the mask coverage at the doc pixel (direct and
  isolated/clip render paths).
- `atelier-core::command::SetLayerMask`; app `set_layer_mask(from_selection)` (Layer → Add
  Layer Mask from Selection / Remove Layer Mask).

## Out of scope
- Painting on the mask (brush-on-mask editing — needs a mask edit mode); vector masks;
  mask on non-raster layers; mask persistence in `.atl` (session-only — recorded as debt
  R-14 alongside the smart-object/embedded-tile persistence work).

## Verification checklist
- [x] `cargo test -p atelier-app` — masked-in area composites visible, outside hidden; undo
      removes the mask (full layer visible again)
- [x] workspace + clippy `--all-targets -D warnings` clean; smoke run; golden parity
      unaffected (mask defaults to None)

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-13 | `cargo test -p atelier-app` | PASS | `layer_mask_from_selection_and_undo` (left-half mask → left visible / right hidden in composite; undo restores full); app 49 tests |
| 2026-06-13 | workspace + clippy + smoke | PASS | full suite green, clippy clean, app runs 5s no crash |

## Notes / surprises
- Mask is doc-space; the compositor multiplies layer alpha by `mask.get(doc_x, doc_y)`, so a
  feathered selection makes a soft-edged mask for free.
- Persistence: like tiles and the smart-object source, masks aren't yet written to `.atl`
  (serde-skip). Tracked as data-persistence debt to close before a format freeze (R-14).