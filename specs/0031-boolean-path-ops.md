# Spec 0031 — Vector engine: boolean path ops (Pathfinder)

- **Status:** ☑ done (2026-06-13)
- **Phase:** 4 (VEC-5 — the last headline vector item)
- **Requirements:** VEC-5 (unite/subtract/intersect/exclude)
- **Depends on:** 0012 (path model), 0028 (selection)

## Goal
Boolean operations on a vector layer's shapes — Unite / Intersect / Minus / Exclude — from
the Properties Pathfinder, undoable. First external geometry dependency (`i_overlay`).

## Scope
- New workspace dep **`i_overlay` 2.2.0** (added to `atelier-vector`). API read from the
  fetched source: `subject.overlay(&clip, OverlayRule, FillRule) -> Shapes<[f32;2]>`.
- `atelier-vector::boolean`: `BoolOp { Union, Intersect, Difference, Exclude }`; `boolean(subj,
  clip, op) -> Path`. Cubics are flattened (24 steps) to polygon contours, the overlay runs
  (NonZero fill), and the result is rebuilt as a line-only `Path` (possibly compound:
  multiple subpaths for holes / disjoint regions). Empty-input/empty-result edge cases handled.
- App `panels::pathfinder(state, id, op)` folds the op left across the selected vector
  layer's shapes, replacing them with the single result shape (undoable `SetVectorShapes`).
  Pathfinder buttons (Unite/Intersect/Minus/Exclude) in the vector Properties section.

## Out of scope
- Cross-layer boolean (between separate layers — needs the result placement design);
  preserving curves through the op (output is flattened polylines — acceptable, re-curving is
  a research task); stroke-aware booleans; choosing the fill rule in the UI.

## Verification checklist
- [x] `cargo test -p atelier-vector` — union spans both rects; intersect = overlap;
      difference removes clip; disjoint intersect = empty; disjoint union = 2 contours
- [x] `cargo test -p atelier-app` — Pathfinder Union merges two overlapping shapes into one
      spanning both; undo restores both shapes
- [x] workspace + clippy `--all-targets -D warnings` clean; smoke run

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-13 | `cargo test -p atelier-vector` | PASS | 5 boolean tests (union/intersect/difference bounds, disjoint empty intersect, disjoint union = 2 subpaths) |
| 2026-06-13 | `cargo test -p atelier-app` | PASS | `pathfinder_union_via_app` (2 overlapping rects → 1 shape spanning x 0..15; undo → 2); app 35 tests |
| 2026-06-13 | workspace + clippy + smoke | PASS | full suite green, clippy clean, app runs 5s no crash |

## Notes / surprises
- `i_overlay` lands in `[dependencies]` (not dev): the `boolean` module is non-test lib code,
  so dev-only would compile under `cargo test` but break `cargo build` (caught exactly that).
- Output is flattened polylines (24 steps/cubic) — booleans on curved paths lose exact curves;
  fine for v1 Pathfinder. Re-fitting curves to the result is a future enhancement.
- API discovery method (worth repeating for new deps): add dep → `cargo fetch` → read the
  crate source under `~/.cargo/registry/src/...` → code against the real signatures. Avoids
  guessing an unfamiliar API.