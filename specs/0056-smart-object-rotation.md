# Spec 0056 — Non-destructive smart-object rotation

- **Status:** ☑ done (2026-06-14)
- **Phase:** 10 (smart objects — non-destructive transform)
- **Requirements:** DOC-5, DOC-10 (non-destructive transform)
- **Depends on:** 0055 (smart-object scale), 0054 (move)

## Goal
A smart object rotates non-destructively: the compositor samples the embedded document through
a full affine inverse (rotate + scale), resampling the original pixels each frame. The
Transform dialog's rotation field now applies to smart objects (combined with scale in one
undoable step).

## Scope
- `atelier-core`: `SmartContent.rotation: f32` (radians; serde default `0.0`); `embed` sets 0.
  `SetSmartRotation` command (undoable).
- `atelier-raster::compositor`: replace `ScaledSource` with `AffineSource` — inverse of
  `M = R(θ)·S` mapped per pixel: `e = M⁻¹·(p − offset)`, `M⁻¹ = [[c/sx, s/sx], [−s/sy, c/sy]]`
  (nearest-neighbour). At θ=0 this is identical to the old scaled sampler.
- `atelier-app`: `apply_transform` on a `Smart` selection applies scale **and** rotation as a
  single `Batch` (`SetSmartScale` + `SetSmartRotation`); rotation accumulates additively.

## Out of scope
- Pivot/anchor control — rotation (and scale) are about the smart object's offset origin.
- Bilinear/bicubic resampling (still nearest-neighbour; quality pass later).
- Shear; linked smart objects.

## Verification checklist
- [x] `cargo test -p atelier-raster` — a 90°-rotated smart object lands its marker pixel at the
      rotated position; θ=0 still matches the scaled path
- [x] `cargo test -p atelier-core` — `SetSmartRotation` sets the angle and reverts on undo
- [x] `cargo test -p atelier-app` — Transform applies scale+rotation to a smart object in one
      undoable step
- [x] `cargo build/test --workspace`; clippy `--all-targets -D warnings` clean; smoke run

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-14 | `cargo test -p atelier-raster` | PASS | `smart_object_rotates_about_center` (marker (3,0)→(3,3) at 90°), `smart_object_scales_non_destructively` (centre-pivot 2×); raster 51 tests |
| 2026-06-14 | `cargo test -p atelier-core` | PASS | `set_smart_rotation_applies_and_reverts`; core 45 tests |
| 2026-06-14 | `cargo test -p atelier-app` | PASS | `transform_rotates_and_scales_smart_object_one_step` (one history step, undo restores both); app 55 tests |
| 2026-06-14 | workspace + clippy + smoke | PASS | full suite green; clippy `--all-targets -D warnings` clean; app alive 12s no crash |

## Notes / surprises
- `AffineSource` subsumes the spec-0055 scaled sampler, so one sampler now covers
  translate+scale+rotate. **Pivot changed to the embedded centre** (0055 had a corner pivot):
  corner-pivot rotation throws all content off-canvas, so centre pivot is both correct and
  testable. The 0055 scale test was updated to the centre-pivot result. Pixel-centre sampling
  (`+0.5`) keeps the 90° case off the float-edge of the `< 0` guard.
