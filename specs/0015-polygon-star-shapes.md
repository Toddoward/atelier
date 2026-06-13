# Spec 0015 — Vector engine IV: polygon & star shape primitives

- **Status:** ☑ done (2026-06-13)
- **Phase:** 4 (slice c1b — completes the fillable shape-primitive set; pen/anchor editing is slice c2, future spec)
- **Requirements:** VEC-3 (shape primitives: polygon, star)
- **Depends on:** 0014

## Goal
Two more fillable shape tools — regular Polygon and Star — drawn the same way as
rectangle/ellipse (spec 0014): drag a bounding box, get a filled vector layer that renders
live and is undoable. Completes the fill-capable primitive set (rect/ellipse/polygon/star);
line is deferred until stroke rendering exists.

## Scope
- `atelier-vector::Path::polygon(cx, cy, r, sides)` (first vertex up) and
  `Path::star(cx, cy, r_outer, r_inner, points)` (alternating radii) constructors.
- Generalize the shape pipeline: `ShapeKind { Rect, Ellipse, Polygon, Star }`,
  `ActiveTool::shape_kind()`; `pending_shape` carries the kind. `add_shape_layer` builds the
  path per kind (polygon = 6 sides, star = 5 points / inner = 0.5·outer, derived from the
  drag's bounding box: center + radius = min(w,h)/2).
- Tools panel gets Polygon/Star buttons + shared vector-fill picker (any shape tool).
- Tests: path constructors (vertex counts, radius bounds, first vertex up); the existing
  shape-insert kittest now loops over all four kinds.

## Out of scope
- Line/open-path tool (needs stroke rendering); configurable sides/points UI (fixed 6/5 for
  now); pen tool + anchor/direct-select editing (slice c2); editing an inserted shape.

## Design notes
- Polygon/star reuse 0014's drag→`pending_shape`→`add_shape_layer` plumbing and the 0013
  renderer entirely — only the path constructor differs. Live preview is the bounding-box
  rubber band (exact outline preview deferred).

## Verification checklist
- [x] `cargo test -p atelier-vector` — polygon vertex count + radius fit + vertex-up; star
      alternating radii / vertex count
- [x] `cargo test -p atelier-app` — shape-insert kittest covers rect/ellipse/polygon/star
- [x] workspace + clippy `--all-targets -D warnings` clean; smoke run

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-13 | `cargo test -p atelier-vector` | PASS | `polygon_has_n_vertices_and_fits_radius` (hexagon = 6 verts, within radius, first vertex up), `star_alternates_radii` (5-point = 10 verts) |
| 2026-06-13 | `cargo test -p atelier-app` | PASS | `shape_tool_drag_inserts_vector_layer_and_undoes` loops all four kinds: each adds one vector layer + undo removes it |
| 2026-06-13 | workspace + clippy + smoke | PASS | full suite green (app 23 / core 35 / vector +2), clippy clean, app runs 5s no crash |

## Notes / surprises
- `ShapeKind` + `ActiveTool::shape_kind()` cleanly replaced the earlier ad-hoc
  `is_ellipse: bool` in `pending_shape`, so adding kinds is now a one-line match per site.
- Configurable polygon-sides / star-points and a true shape-outline drag preview are the
  obvious next polish; deferred.
