# Spec 0017 — Vector engine VI: direct-select anchor editing

- **Status:** ☑ done (2026-06-13)
- **Phase:** 4 (slice c2b — move on-path anchors; bezier-handle drag + add/remove anchor are slice c2c, future)
- **Requirements:** VEC-2 (direct selection: move anchors)
- **Depends on:** 0016

## Goal
The Direct Select tool (A) lets the user drag an on-path anchor of the selected vector layer
to reshape it, live and undoable (one history entry per drag). First editing of *existing*
vector geometry (everything before only inserted whole shapes).

## Scope
- `atelier-vector::Path::anchors()` (subpath starts + each segment endpoint, in order) and
  `Path::move_anchor(index, to)` (moves a line point or a cubic endpoint, leaving bezier
  handles untouched).
- `atelier-core::command::SetVectorShapes { id, old, new }` — snapshots the layer's shapes
  vec; mergeable so a drag is one undo entry.
- `ActiveTool::DirectSelect` (key A): on drag-start, hit-test the nearest anchor across the
  selected vector layer's shapes within ~10 screen px (zoom-independent); during drag, move
  it via merged `SetVectorShapes`; anchor dots drawn as an overlay.
- `EditorState.anchor_drag: Option<(shape_idx, anchor_idx)>`.

## Out of scope
- Bezier control-handle drag, converting line↔curve anchors, add/remove anchor, multi-anchor
  marquee selection, snapping (all slice c2c / later); editing across multiple layers at once.

## Design notes
- Reuses `press_origin` for the grab point (event-coalescing lesson from spec 0007) and
  `interact_pointer_pos` for the live position; merged command via `set_merging` during drag,
  exactly like the Move tool's `SetOffset`.
- Whole-shapes snapshot (not a per-anchor delta) keeps the command trivial and correct;
  vector shape vectors are small.

## Verification checklist
- [x] `cargo test -p atelier-vector` — `anchors`/`move_anchor` (line + cubic-endpoint,
      handles preserved, out-of-range no-op)
- [x] `cargo test -p atelier-core` — `SetVectorShapes` apply/revert identity + merge
- [x] workspace + clippy `--all-targets -D warnings` clean; smoke run
- [ ] [manual·non-gating] drag an anchor of a drawn shape; one undo reverts the whole drag

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-13 | `cargo test -p atelier-vector` | PASS | `anchors_and_move_anchor` (rect = 4 anchors, move start + endpoint, OOB no-op); `move_anchor_keeps_cubic_handles` (ellipse cubic endpoint moves, control points unchanged) |
| 2026-06-13 | `cargo test -p atelier-core` | PASS | `set_vector_shapes_apply_revert_and_merge` (36 core tests total) |
| 2026-06-13 | workspace + clippy + smoke | PASS | full suite green (app 25 / core 36 / vector), clippy clean, app runs 5s no crash |

## Notes / surprises
- App-level anchor-drag hit-testing is manual-verified (its exact screen mapping isn't
  cheaply reconstructable in a headless kittest); the editing math + command are fully
  unit-covered, and the tool reuses the well-tested Move-tool merged-drag plumbing.
- Whole-shape snapshots are fine at current scene sizes; switch to per-anchor deltas only if
  a profiling need appears.