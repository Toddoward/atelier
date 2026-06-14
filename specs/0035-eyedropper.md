# Spec 0035 — Eyedropper tool

- **Status:** ☑ done (2026-06-13)
- **Phase:** 6 groundwork (COL-6 color picking; no color-management dep yet)
- **Requirements:** COL-6 (eyedropper)
- **Depends on:** 0006 (compositor), 0014 (brush color)

## Goal
An Eyedropper tool (key `I`) samples the composited document color under the cursor into the
brush color and the vector fill color.

## Scope
- `ActiveTool::Eyedropper` + Tools-panel entry + `I` shortcut (plain; `Ctrl+I` stays Invert).
- Canvas: on click/drag, `canvas::sample_composite(state, doc_px)` composites the document
  (`composite_rgba8`) and reads the pixel; the result sets `brush.color` and `vector_fill`.

## Out of scope
- Sample radius / average; sample from a single layer vs. composite; "copy hex" UI; managed
  color values (Lab/CMYK readouts) — those come with Phase 6.

## Verification checklist
- [x] `cargo test -p atelier-app` — sample over a placed green square returns green; sample
      where nothing is painted returns transparent; out-of-bounds returns None
- [x] workspace + clippy `--all-targets -D warnings` clean; smoke run

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-13 | `cargo test -p atelier-app` | PASS | `eyedropper_samples_composite_color` (green in-square, transparent outside, None out-of-bounds); app 38 tests |
| 2026-06-13 | workspace + clippy + smoke | PASS | full suite green, clippy clean, app runs 5s no crash |

## Notes / surprises
- Samples the *composite* (what you see), not the active layer — matches Photoshop's default
  "Sample: All Layers". Per-layer sampling is a later option.
- Re-composites on each pick (cheap at current sizes); could read the cached composite texture
  later if it shows up in profiles.