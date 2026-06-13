# Spec 0026 — Vector engine XIII: align & distribute shapes in a layer

- **Status:** ☑ done (2026-06-13)
- **Phase:** 4 (VEC-6 — within-layer subset; cross-layer needs multi-select, later)
- **Requirements:** VEC-6 (align/distribute)
- **Depends on:** 0022, 0024

## Goal
Align the shapes of a vector layer to each other (L/C/R/T/M/B, relative to their union
bounds) and distribute them evenly by center horizontally or vertically — undoable, from the
Properties panel. No multi-node selection needed: it operates on the selected vector layer's
shape list.

## Scope
- `panels::align_shapes_in_layer(state, id, Align)` — translate each shape so its bound aligns
  to the union-bounds edge/center (no-op < 2 shapes).
- `panels::distribute_shapes_in_layer(state, id, horizontal)` — even-space shape centers
  between the extreme shapes along the axis (no-op < 3 shapes).
- Both via `SetVectorShapes` (undoable). Properties: "Align shapes" L/C/R/T/M/B row +
  "Distribute H/V" buttons.

## Out of scope
- Cross-layer align/distribute (needs node multi-select — a later refactor); distribute by
  spacing/gaps (this is distribute-by-center); align to a key object.

## Verification checklist
- [x] `cargo test -p atelier-app` — align-top puts all shapes' tops at the union top;
      distribute-H centers the middle shape at the mean of the extremes; undo restores
- [x] workspace + clippy `--all-targets -D warnings` clean; smoke run

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-13 | `cargo test -p atelier-app` | PASS | `align_and_distribute_shapes_in_layer` (3 rects: align-top→all tops 0, distribute-H→middle center 50, undo×2 restores tops); app 30 tests |
| 2026-06-13 | workspace + clippy + smoke | PASS | full suite green, clippy clean, app runs 5s no crash |

## Notes / surprises
- This is the within-a-layer slice of VEC-6 — it needed no multi-selection because a vector
  layer already holds multiple shapes. Cross-layer align waits on node multi-select.
- Distribute is by center (Illustrator's default); spacing/gap distribution is a follow-up.