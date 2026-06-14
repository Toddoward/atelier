# Spec 0049 — Layer mask operations: invert & apply

- **Status:** ☑ done (2026-06-13)
- **Phase:** 2/3 (DOC-4 mask editing)
- **Requirements:** DOC-4
- **Depends on:** 0047 (layer masks)

## Goal
Invert a layer mask, and apply (bake) it into the layer's pixels then clear it — both
undoable (Layer menu).

## Scope
- `atelier-core::command::ApplyLayerMask` — multiply each tile pixel's alpha by the mask
  coverage (offset aware), prune blanks, clear the mask; revert restores the pre-bake tiles
  and mask.
- App `invert_layer_mask` (`SetLayerMask` with `mask.inverted(doc_size)`) and
  `apply_layer_mask` (`ApplyLayerMask`). Layer-menu entries enabled when the selected raster
  layer has a mask.

## Out of scope
- Painting directly on the mask (brush-on-mask edit mode — needs a mask paint target);
  density/feather mask properties; disabling (toggling) a mask without removing it.

## Verification checklist
- [x] `cargo test -p atelier-app` — apply bakes mask into alpha (masked-out → transparent),
      clears the mask; undo restores both pixels and mask
- [x] workspace + clippy `--all-targets -D warnings` clean; smoke run

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-13 | `cargo test -p atelier-app` | PASS | `apply_layer_mask_bakes_and_undoes` (masked-out baked transparent, mask cleared, undo restores pixels+mask); app 50 tests |
| 2026-06-13 | workspace + clippy + smoke | PASS | full suite green, clippy clean, app runs 5s no crash |

## Notes / surprises
- `ApplyLayerMask` snapshots the whole tile map for undo (simple + correct); a per-tile diff
  could shrink memory later if it matters.
- Invert reuses `Mask::inverted(doc_size)` (the same op the Select-menu invert uses).