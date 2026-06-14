# Spec 0050 — Paint on layer mask

- **Status:** ☑ done (2026-06-13)
- **Phase:** 2/3 (DOC-4 mask editing — completes masks)
- **Requirements:** DOC-4, RAS-2 (brush on mask)
- **Depends on:** 0047 (layer masks), 0005 (brush)

## Goal
With "Edit mask" on, the brush reveals (raises coverage) and the eraser hides (lowers
coverage) the selected layer's mask; one undoable step per stroke.

## Scope
- `atelier-raster::brush::stamp_mask_segment(mask, from, to, radius, hardness, erase)` — paint
  smoothstep coverage into a `Mask` (doc space): brush `max`-blends toward 255, eraser scales
  down.
- App: `EditorState.mask_edit` toggle (+ `mask_stroke` snapshot); canvas brush/eraser arm
  routes to `handle_mask_paint` when editing a masked layer — live preview during the drag,
  committing one `SetLayerMask` (pre-stroke snapshot → new) on release. Tools-panel "Edit
  mask" checkbox (shown when the layer has a mask).

## Out of scope
- Mask density/feather sliders; gradient/shape tools on the mask; a dedicated mask thumbnail
  with active-target indicator (toggle stands in).

## Verification checklist
- [x] `cargo test -p atelier-app` — a brush stroke in mask-edit mode adds mask coverage; undo
      restores the prior mask
- [x] workspace + clippy `--all-targets -D warnings` clean; smoke run

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-13 | `cargo test -p atelier-app` | PASS | `paint_on_mask_edits_mask_and_undoes` (stroke adds coverage, undo clears); app 51 tests |
| 2026-06-13 | workspace + clippy + smoke | PASS | full suite green, clippy clean, app runs 5s no crash |

## Notes / surprises
- Mask painting follows the live-edit pattern (mutate for preview, commit one command on
  release) like the pixel brush — but commits a `SetLayerMask` snapshot rather than
  `PaintTiles`. With specs 0047–0050, layer masks are complete: create, paint, invert, apply,
  persist.