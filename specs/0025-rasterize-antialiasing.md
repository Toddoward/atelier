# Spec 0025 — Interop I.b: anti-aliased vector rasterization

- **Status:** ☑ done (2026-06-13)
- **Phase:** 5 groundwork (INT-2 polish)
- **Requirements:** INT-2 (rasterize quality)
- **Depends on:** 0023

## Goal
Rasterizing a vector layer (spec 0023) now produces smooth edges: 4×4 supersample coverage
per pixel, written as straight-alpha source-over so overlapping shapes blend.

## Scope
- Rewrite `atelier-raster::raster_vector`: per filled shape, accumulate subsample hits
  (4×4 = 16 per pixel) across the shape's triangles into a coverage map, then write each
  touched pixel as `fill` with alpha `× coverage`, src-over onto existing tiles.

## Out of scope
- Higher sample counts / analytic coverage; gamma-correct AA (blends in straight sRGB
  components, consistent with the rest of the engine for now); stroke rasterization.

## Verification checklist
- [x] `cargo test -p atelier-raster` — interior full coverage, exterior clear, **edge pixels
      partially covered** (0 < alpha < 255), overlapping shapes blend
- [x] `cargo test -p atelier-app` — existing rasterize+undo still passes (interior solid)
- [x] workspace + clippy `--all-targets -D warnings` clean

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-13 | `cargo test -p atelier-raster` | PASS | `edges_are_antialiased` (half-pixel-offset rect: left column 0<α<255, interior α=255); `rasterizes_a_filled_rect`, `unfilled_shape_produces_nothing` |
| 2026-06-13 | `cargo test -p atelier-app` | PASS | `rasterize_vector_layer_and_undo` still green (app 29) |
| 2026-06-13 | workspace + clippy | PASS | full suite green, clippy clean |

## Notes / surprises
- Coverage accumulates additively across a shape's triangles (clamped to 16), so internal
  shared triangle edges sum to full coverage — no seams — while shape boundaries get true
  partial coverage.
- AA blends in straight sRGB components (not linear) — matches the compositor's current
  space; a gamma-correct pass can come with the color-management phase.