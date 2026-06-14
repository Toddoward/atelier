# Spec 0045 — Selection to vector path (trace)

- **Status:** ☑ done (2026-06-13)
- **Phase:** 5 (INT-5 reverse — selection outline → vector path)
- **Requirements:** INT-5
- **Depends on:** 0007 (selection boundary), 0012 (path)

## Goal
Trace the current selection's boundary into a new vector layer (Select → To Vector Path),
undoably.

## Scope
- `atelier-raster::selection::boundary_paths(mask)` — chain the marching-squares unit
  segments into closed loops, simplifying collinear runs to corners.
- App `selection_to_vector` — build a `Path` (one subpath per loop, even-odd) filled with the
  vector fill color, added as a Vector layer. Select-menu entry.

## Out of scope
- Curve fitting (output is rectilinear polylines from the pixel mask); per-loop hole
  detection beyond even-odd; combining with an existing vector layer.

## Verification checklist
- [x] `cargo test -p atelier-raster` — rect selection traces to one 4-corner loop with
      matching bounds
- [x] `cargo test -p atelier-app` — selection → vector layer whose path bounds match; undo
      removes it
- [x] workspace + clippy `--all-targets -D warnings` clean; smoke run

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-13 | `cargo test -p atelier-raster` | PASS | `boundary_paths_of_rect_is_one_quad` (1 loop, 4 corners, bounds match) |
| 2026-06-13 | `cargo test -p atelier-app` | PASS | `selection_to_vector_traces_rect` (vector layer, path bounds = selection, undo removes); app 48 tests |
| 2026-06-13 | workspace + clippy + smoke | PASS | full suite green, clippy clean, app runs 5s no crash |

## Notes / surprises
- Together with spec 0044 (layer → selection) this closes both INT-5 directions for the
  common cases. Output is rectilinear (pixel-accurate); smooth curve fitting is a later polish.
- Loops come straight from `boundary_segments`, so holes in the selection produce inner loops
  handled by the even-odd fill rule.