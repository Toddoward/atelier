# Spec 0024 — Vector engine XII: compound paths (make / release)

- **Status:** ☑ done (2026-06-13)
- **Phase:** 4 (VEC-8)
- **Requirements:** VEC-8 (compound paths)
- **Depends on:** 0022

## Goal
Combine a vector layer's shapes into a single compound path (even-odd fill, so overlaps cut
holes) and release it back into separate shapes — both undoable, from the Properties panel.

## Scope
- `atelier-vector::Path::append(&other)` — concatenate subpaths; `Path::split_subpaths()` —
  one single-subpath `Path` per subpath (carries the fill rule).
- App `panels::make_compound_path` — merge the layer's shapes into one shape whose path holds
  all subpaths, `FillRule::EvenOdd`, keeping the first shape's fill/stroke (no-op < 2 shapes).
- App `panels::release_compound_path` — split every shape's subpaths into separate shapes
  (no-op when nothing is multi-subpath). Both via `SetVectorShapes` (undoable).
- Properties: "Make Compound" / "Release" buttons in the vector section.

## Out of scope
- Boolean ops (i_overlay) — compound paths are the even-odd "combine" only, not true
  unite/subtract/intersect; preserving per-shape fills on release (all released shapes take
  the source shape's fill); winding-rule UI toggle.

## Verification checklist
- [x] `cargo test -p atelier-vector` — append grows subpath count; split_subpaths inverts and
      carries fill rule + per-rect bounds
- [x] `cargo test -p atelier-app` — make (2→1) then release (1→2); undo chain restores both
- [x] workspace + clippy `--all-targets -D warnings` clean; smoke run

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-13 | `cargo test -p atelier-vector` | PASS | `append_and_split_subpaths_round_trip` (1+1→2 subpaths, split→2 paths, EvenOdd carried, bounds correct) |
| 2026-06-13 | `cargo test -p atelier-app` | PASS | `compound_path_make_and_release` (2→1→2, undo→1→2); app 29 tests |
| 2026-06-13 | workspace + clippy + smoke | PASS | full suite green, clippy clean, app runs 5s no crash |

## Notes / surprises
- Compound = even-odd combine, which already gives holes for overlapping subpaths via the
  existing tessellator fill rule — true boolean Pathfinder ops (i_overlay) are still separate.
- Release flattens all subpaths to the source shape's fill; per-subpath fill memory would
  need richer shape data — deferred.