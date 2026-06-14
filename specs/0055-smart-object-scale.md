# Spec 0055 — Non-destructive smart-object scale

- **Status:** ☑ done (2026-06-14)
- **Phase:** 10 (smart objects — non-destructive transform)
- **Requirements:** DOC-5, DOC-10 (non-destructive transform)
- **Depends on:** 0052 (smart objects embed & composite), 0054 (move smart objects)

## Goal
A smart object can be scaled **non-destructively** — the embedded document keeps its original
pixels; the compositor samples it at the requested scale. Re-scaling never compounds quality
loss (it always resamples from the embedded source). The Transform dialog drives it.

## Scope
- `atelier-core`: `SmartContent.scale: [f32; 2]` (serde default `[1,1]`); `SmartContent::embed`
  helper (offset `[0,0]`, scale `[1,1]`). `SetSmartScale` command (undoable) sets the scale.
- `atelier-raster::compositor`: the `Smart` arm composites the embedded doc at native
  resolution into its own buffer, then blends through a nearest-neighbour **scaled** source
  (parent → embedded = `(p - offset) / scale`), placing it at `offset` with size
  `doc.size · scale`.
- `atelier-app`: `apply_transform` branches on kind — a `Smart` selection multiplies its scale
  by the dialog's X/Y percentages via `SetSmartScale` (rotation ignored for smart objects in
  this slice); raster selections bake as before.

## Out of scope
- Bilinear/bicubic sampling (nearest-neighbour here; quality pass later).
- Rotation / shear of smart objects (translate + scale only).
- Anchor/pivot control (scales from the smart object's offset origin).

## Verification checklist
- [x] `cargo test -p atelier-raster` — a 2× smart object covers 2× the area in the composite;
      pixels resample from the embedded source
- [x] `cargo test -p atelier-core` — `SetSmartScale` sets the scale and reverts on undo
- [x] `cargo test -p atelier-app` — Transform on a smart object sets a non-destructive scale
      (embedded pixels unchanged) and undo restores
- [x] `cargo build/test --workspace`; clippy `--all-targets -D warnings` clean; smoke run

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-14 | `cargo test -p atelier-raster` | PASS | `smart_object_scales_non_destructively` (2×2 block → 4×4 at 2× scale); raster 50 tests |
| 2026-06-14 | `cargo test -p atelier-core` | PASS | `set_smart_scale_applies_and_reverts`; core 44 tests |
| 2026-06-14 | `cargo test -p atelier-app` | PASS | `transform_scales_smart_object_non_destructively` (scale [2,1.5] set, embedded 64² untouched, undo restores); app 54 tests |
| 2026-06-14 | workspace + clippy + smoke | PASS | full suite green; clippy `--all-targets -D warnings` clean; app alive 12s no crash; io deep-equal round-trips still pass (serde `unit_scale` default) |

## Notes / surprises
- Scaling resamples from the embedded buffer every composite, so it's lossless under repeated
  edits — the whole point of a smart object vs. baking raster tiles.
- **Superseded pivot:** this slice scaled about the offset origin; spec 0056 moved scale (and
  rotation) to pivot about the embedded centre. The compositor scale test here was updated to
  the centre-pivot result when 0056 landed.
