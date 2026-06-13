# Spec 0014 — Vector engine III: shape tools (rect/ellipse insertion)

- **Status:** ☑ done (2026-06-13)
- **Phase:** 4 (slice c1 — shape insertion; pen/anchor editing is slice c2, spec 0015)
- **Requirements:** VEC-3 (shape primitives: rect, ellipse), VEC-4 (fill/stroke on creation)
- **Depends on:** 0013

## Goal
The user can create vector shape layers: a Rectangle and Ellipse tool drag out a shape on the
canvas, inserting a new vector layer (filled with the current vector fill color) that renders
live (spec 0013) and is undoable. This is the first vector *authoring* — until now vector
layers only existed in tests/fixtures.

## Scope
- App tools `ActiveTool::ShapeRect` / `ShapeEllipse` (Tools panel + keys: U rect, no global
  conflict). A vector fill color in `BrushSettings` (reuse `color` or add `vector_fill`).
- Canvas drag: press→drag rubber-band (preview outline), release → build a
  `VectorContent { shapes: [Shape::filled(Path::rect|ellipse(bounds), fill)] }` and insert as
  a new `NodeKind::Vector` layer above the selection via `AddNode` (undoable). Degenerate
  (near-zero) drags are ignored.
- Reuse the marquee drag plumbing (press_origin start + interact_pointer_pos current) and the
  0013 renderer for display.
- kittest: rect tool drag inserts one vector layer whose shape bounds match the drag; undo
  removes it; ellipse tool likewise.

## Out of scope
- Pen tool / anchor add-move-convert + direct-select (spec 0015); polygon/star/line
  (follow-up); editing an existing shape's geometry; stroke-on-create UI (fill only this
  slice; stroke via Properties later); booleans/align (later Phase-4 slices).

## Design notes
- Shape layers reuse the existing vector render + cache (0013) and `AddNode` undo — no new
  command type. Insert position mirrors `add_adjustment_layer` (above selection / root top).
- Rubber-band preview drawn with the same screen-mapping as the selection marquee.

## Verification checklist
- [ ] `cargo test -p atelier-app` — kittest: rect-drag inserts a vector layer with matching
      shape bounds + undo; ellipse-drag inserts an ellipse shape
- [ ] workspace + clippy `--all-targets -D warnings` clean; smoke run
- [ ] [manual·non-gating] drag a rectangle/ellipse, see it render and pan/zoom

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-13 | `cargo test -p atelier-app` (kittest) | PASS | `shape_tool_drag_inserts_vector_layer_and_undoes`: rect AND ellipse tool drag each add one `NodeKind::Vector` layer with one shape, selected; Ctrl+Z removes it |
| 2026-06-13 | workspace + clippy `--all-targets -D warnings` + smoke | PASS | app 23 / core 35 / full suite green, clippy clean, app runs 5s no crash |

## Notes / surprises
- Reused the marquee drag plumbing (`select_drag` for the rubber band, `press_origin` start)
  and the 0013 renderer — no new command (plain `AddNode`). Shape insertion is queued on
  `EditorState.pending_shape` and drained in the app loop (canvas can't call app helpers),
  same pattern as the magic-wand click queue.
- Fill-only this slice; stroke-on-create + polygon/star/line + editing existing geometry +
  the pen tool are spec 0015 (slice c2).
