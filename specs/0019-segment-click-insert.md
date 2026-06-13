# Spec 0019 — Vector engine VIII: segment-click anchor insert

- **Status:** ☑ done (2026-06-13)
- **Phase:** 4 (slice c2d — click a segment to add an anchor; bezier handle drag is slice c2e, spec 0020)
- **Requirements:** VEC-2 (add anchors on a path)
- **Depends on:** 0018

## Goal
Double-clicking on a path segment with Direct Select inserts a new anchor there, splitting
the segment, undoably — completing the add-anchor gesture whose primitive (`insert_anchor`)
landed in spec 0018.

## Scope
- `atelier-vector::Path::closest_segment(query) -> (anchor_index, distance)` — nearest point
  on any segment (lines exact; cubics sampled), returning the `index` to pass to
  `insert_anchor` plus the doc-space distance. Free helpers `dist_to_segment`/`dist_to_cubic`.
- App: Direct Select (A) double-click — find the closest segment across the selected vector
  layer's shapes within ~10 screen px and `insert_anchor` at the click via `SetVectorShapes`
  (undoable).

## Out of scope
- Bezier control-handle drag / line↔curve conversion (spec 0020); inserting onto a cubic
  preserves the cubic's shape only approximately (anchor goes at the click, not a true
  de Casteljau split — acceptable for now, noted).

## Design notes
- Screen-space threshold (`distance * zoom < 10`) keeps the hit zone zoom-independent.
- Reuses `SetVectorShapes`; no new command.

## Verification checklist
- [x] `cargo test -p atelier-vector` — `closest_segment` returns the right split index +
      distance for rect edges; inserting at it grows the path by one anchor
- [x] workspace + clippy `--all-targets -D warnings` clean; smoke run
- [ ] [manual·non-gating] double-click a shape edge adds an anchor there; undo restores

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-13 | `cargo test -p atelier-vector` | PASS | `closest_segment_finds_split_index` (top edge → split index 1, right edge → 2, insert grows to 5 anchors) |
| 2026-06-13 | workspace + clippy + smoke | PASS | full suite green (app 26 / core 36 / vector), clippy clean, app runs 5s no crash |

## Notes / surprises
- Cubic distance is a 16-sample polyline approximation — fine for hit-testing; exact
  de Casteljau split (so inserting on a curve preserves shape) is deferred with handle work.
- App double-click insert is manual-verified (headless screen-mapping caveat); the
  geometry primitive is unit-covered.