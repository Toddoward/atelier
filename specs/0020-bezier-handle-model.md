# Spec 0020 — Vector engine IX: bezier handle model primitives

- **Status:** ☑ done (2026-06-13)
- **Phase:** 4 (slice c2e-model — handle data primitives; the on-canvas handle-drag UI is slice c2f, spec 0021)
- **Requirements:** VEC-2 (convert anchors to curves / bezier handles)
- **Depends on:** 0019

## Goal
The path model can carry bezier control handles on any anchor: set an anchor's outgoing or
incoming control point, converting the adjacent line segment to a cubic in place (endpoints
preserved). This is the data layer the handle-drag UI (spec 0021) will drive.

## Scope
- `atelier-vector::Path::set_out_handle(index, point)` — sets the control point leaving the
  anchor; converts its outgoing segment Line→Cubic (keeps the far control / endpoint).
- `atelier-vector::Path::set_in_handle(index, point)` — sets the control point arriving at
  the anchor; converts its incoming segment Line→Cubic (keeps the near control via the
  previous anchor). No-ops at subpath boundaries (no outgoing seg at the end / no incoming at
  the start).

## Out of scope
- On-canvas handle rendering, hit-testing, and drag gesture (spec 0021); symmetric/mirrored
  vs. independent handle modes; handles across the implicit closing edge of a closed subpath
  (the closing edge isn't a stored segment yet); exact de Casteljau curve-split on insert.

## Design notes
- Anchor indices follow `Path::anchors()`. Outgoing handle edits `segs[local]`; incoming edits
  `segs[local-1]`. Line→Cubic conversion fills the untouched control with the existing
  endpoint/neighbor so geometry stays put until a handle is actually moved.

## Verification checklist
- [x] `cargo test -p atelier-vector` — set_out/in_handle convert Line→Cubic, preserve
      endpoints, no-op at boundaries
- [x] workspace + clippy `--all-targets -D warnings` clean

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-13 | `cargo test -p atelier-vector` | PASS | `set_handles_convert_line_to_cubic` (out handle on anchor 0 + in handle on anchor 2 → cubics, endpoints preserved, anchor count stable, boundary no-ops); 15 vector tests |
| 2026-06-13 | workspace + clippy | PASS | full suite green, clippy clean |

## Notes / surprises
- Closing-edge handles (last→first anchor of a closed subpath) need the close to be a real
  segment; deferred. For now only stored segments take handles.
- Pure model slice — no app wiring yet, so no smoke-relevant change; UI drag is spec 0021.