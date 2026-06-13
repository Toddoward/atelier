# Spec 0016 — Vector engine V: pen tool (polyline/polygon authoring)

- **Status:** ☑ done (2026-06-13)
- **Phase:** 4 (slice c2a — click-to-place path authoring; anchor/handle editing is slice c2b, spec 0017)
- **Requirements:** VEC-2 (pen tool: place anchors), VEC-1 (multi-anchor paths)
- **Depends on:** 0015

## Goal
A Pen tool: click to drop straight-line anchors, building a path live on the canvas; close
it (click near the first anchor, or Enter with ≥3 points) into a filled vector layer, or
finish open. Escape cancels. This is the first multi-anchor authoring; bezier-handle drag
and editing existing anchors are slice c2b (spec 0017).

## Scope
- `ActiveTool::Pen` (key P). `EditorState.pen_points: Vec<[f32;2]>` — the in-progress path.
- Canvas: primary click appends `pointer_doc` to `pen_points`; clicking within ~8 screen px
  of the first point (with ≥3 points) closes and finishes; Enter finishes (closed if ≥3,
  else open polyline if ≥2); Escape clears without inserting. Live preview: the polyline so
  far + a dot per anchor + a rubber-band segment to the cursor.
- `finish_pen(state, closed)` (free fn, mirrors `finish_selection`): builds a `Path`
  (`move_to` + `line_to`…, `close()` if closed) and inserts a filled `NodeKind::Vector`
  layer via `AddNode` (undoable), selects it, clears `pen_points`.
- Tools panel Pen button; P shortcut.
- Tests: kittest — three clicks + Enter inserts one vector layer whose shape has the
  expected anchor count; Escape after clicks inserts nothing; undo removes the inserted layer.

## Out of scope
- Bezier handle drag (curve anchors), editing/moving anchors of an existing path,
  add/remove anchor on an existing path, direct-select (all slice c2b / spec 0017);
  stroke-on-create; rubber-band of curve previews.

## Design notes
- Reuse `pointer_doc` + the 0013 renderer; pen state is transient UI (like a brush stroke),
  committed once as a single `AddNode`. No new command type.
- Close detection in screen space (zoom-independent ~8 px) using the viewport mapping.

## Verification checklist
- [ ] `cargo test -p atelier-app` — pen: N clicks + Enter → vector layer with N anchors;
      Escape inserts nothing; undo removes the layer
- [ ] workspace + clippy `--all-targets -D warnings` clean; smoke run

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-13 | `cargo test -p atelier-app` (kittest) | PASS | `pen_tool_builds_path_layer_and_undoes`: 3 clicks → 3 anchors, Enter → one closed Vector layer (start + 2 line segs), undo removes it; `pen_tool_escape_inserts_nothing` |
| 2026-06-13 | workspace + clippy `--all-targets -D warnings` + smoke | PASS | full suite green (app 25), clippy clean, app runs 5s no crash |

## Notes / surprises
- `Path::polyline(points, closed)` added to atelier-vector; pen reuses it + the 0013
  renderer + `AddNode` (no new command), committing once on finish like a brush stroke.
- Close detection is in screen space (~8 px) so it's zoom-independent.
- Bezier-handle drag and editing/moving anchors of an existing path are the remaining
  vector-authoring gap (spec 0017, slice c2b).
