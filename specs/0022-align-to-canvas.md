# Spec 0022 — Vector engine XI: align vector layer to canvas

- **Status:** ☑ done (2026-06-13)
- **Phase:** 4 (slice — a no-dep subset of VEC-6 align; multi-object align + distribute wait for multi-select)
- **Requirements:** VEC-6 (align) — single-layer-to-canvas subset
- **Depends on:** 0021

## Goal
Align the selected vector layer (its shapes as a group) to the document bounds — left /
horizontal-center / right / top / vertical-middle / bottom — undoably, from the Properties
panel.

## Scope
- `atelier-vector::Path::translate(dx, dy)` — shift all anchors + control points.
- App `panels::align_vector_to_canvas(state, id, Align)` — union the layer's shape bounds,
  compute the offset to the chosen canvas edge/center, translate all shapes via
  `SetVectorShapes` (undoable). Six small buttons (L/C/R/T/M/B) in the vector Properties
  section.

## Out of scope
- Multi-object align / distribute (needs multi-selection, which doesn't exist yet);
  align to selection bounds or to a key object; snapping/guides.
- Boolean path ops (unite/subtract/intersect/exclude) — deferred to a fresh session: they
  need the new `i_overlay` workspace dependency whose API should be read first, not added
  blind near a budget limit.

## Verification checklist
- [x] `cargo test -p atelier-vector` — `translate` shifts bounds by the delta
- [x] `cargo test -p atelier-app` — align-left moves the layer's left edge to 0; undo restores
- [x] workspace + clippy `--all-targets -D warnings` clean; smoke run

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-13 | `cargo test -p atelier-vector` | PASS | `translate_shifts_all_points` (ellipse bounds shift by dx,dy); 17 vector tests |
| 2026-06-13 | `cargo test -p atelier-app` | PASS | `vector_align_to_canvas_left_and_undo` (left edge 20→0, undo→20); app 27 tests |
| 2026-06-13 | workspace + clippy + smoke | PASS | full suite green, clippy clean, app runs 5s no crash |

## Notes / surprises
- Align uses `Path::bounds` (control-hull, not exact curve extrema) — close enough for layout;
  exact-extrema bounds can come later if needed.
- This is the no-dependency slice of VEC-6; the multi-object align/distribute and boolean
  Pathfinder ops are the remaining Phase-4 vector items.