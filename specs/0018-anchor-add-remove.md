# Spec 0018 — Vector engine VII: add / remove path anchors

- **Status:** ☑ done (2026-06-13)
- **Phase:** 4 (slice c2c — add/remove anchors; bezier control-handle drag is slice c2d, future)
- **Requirements:** VEC-2 (pen/direct-select: add & remove anchors)
- **Depends on:** 0017

## Goal
Edit an existing path's anchor count: remove an anchor (path reconnects across the gap) and
insert a new line anchor between two existing ones, undoably. Alt+click an anchor in Direct
Select removes it.

## Scope
- `atelier-vector::Path::remove_anchor(index)` — removes the anchor and the segment that
  ended at it, reconnecting; refuses to drop below 2 anchors per subpath (returns bool).
- `atelier-vector::Path::insert_anchor(index, point)` — inserts a line anchor before the
  anchor at `index` (same subpath; refuses a subpath-start boundary / OOB).
- App: Direct Select (A) — Alt+click within ~10 screen px of an anchor removes it via a
  `SetVectorShapes` command (undoable). Anchor hit-testing factored into `nearest_anchor`.

## Out of scope
- Bezier control-handle drag / line↔curve conversion (slice c2d); double-click-on-segment
  to insert (needs segment hit-testing — provided primitive, app wiring deferred);
  multi-select; snapping.

## Design notes
- Anchor indices follow `Path::anchors()` order (subpath start + each segment endpoint).
  `remove_anchor` at local 0 promotes the first segment's endpoint to the new start.
- Reuses the DirectSelect plumbing + `SetVectorShapes`; no new command.

## Verification checklist
- [x] `cargo test -p atelier-vector` — remove then insert restores anchor count + placement;
      min-2-anchor guard; OOB / start-boundary no-ops
- [x] workspace + clippy `--all-targets -D warnings` clean; smoke run
- [ ] [manual·non-gating] Alt+click an anchor of a drawn shape removes it; undo restores

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-13 | `cargo test -p atelier-vector` | PASS | `remove_and_insert_anchor` (rect 4→3→4, placement, OOB/start no-ops), `remove_anchor_keeps_minimum_two` |
| 2026-06-13 | workspace + clippy + smoke | PASS | full suite green (app 26 / core 36 / vector), clippy clean, app runs 5s no crash |

## Notes / surprises
- Insert-anchor primitive is unit-tested but its on-segment app interaction (double-click to
  split a segment) is deferred to slice c2d — needs segment (not just anchor) hit-testing.
- Alt+click remove reuses the shared `nearest_anchor` hit-test; app interaction is
  manual-verified (headless mapping caveat from 0017).