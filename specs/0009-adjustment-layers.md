# Spec 0009 — Selections III: non-destructive adjustment layers

- **Status:** ☑ done (2026-06-13)
- **Phase:** 3 (slice c)
- **Requirements:** DOC-1 (adjustment node kind), RAS-6 (adjustments as adjustment layers)
- **Depends on:** 0008

## Goal
An adjustment layer is a node that non-destructively re-tones everything composited beneath
it (within its group). Adding/removing/reordering it re-renders live; its parameters are
editable in the Properties panel; it round-trips through `.atl`. The destructive
adjustments from 0008 and these layers share the same `Adjustment` value type.

## Scope
- Move the `Adjustment` data enum + `map_pixel` (and HSL helpers) from
  `atelier-raster::adjust` into `atelier-core::adjust` (pure, serde-derive, no deps).
  `atelier-raster` keeps `apply_tile`/`target_tiles` operating on the core type and
  re-exports `Adjustment` for source compatibility.
- `NodeKind::Adjustment(Adjustment)` (was a unit stub); serde round-trips (no binary
  parts — it's pure params in the manifest).
- `CompositeOp::Adjust(Adjustment)` in the op list; CPU compositor maps the current
  backdrop buffer in place when it hits one (respecting layer opacity as a blend amount,
  and visibility). GPU compositor: Adjust ops are a documented no-op for now (GPU is
  parity-validation only; canvas composites on CPU) — golden fixtures exclude adjustment
  layers; recorded as debt.
- App: "Layer → New Adjustment Layer →" (Invert / Brightness-Contrast / Levels /
  Hue-Saturation) inserts an `Adjustment` node above the selection; Properties panel shows
  + edits the selected adjustment layer's parameters (live recomposite via revision bump).
- Tests: compositor — adjustment layer inverts the backdrop below, ignores layers above,
  respects visibility/opacity; op-list includes Adjust; `.atl` round-trips an adjustment
  layer; kittest — add adjustment layer changes the composite + undo; edit param recomposites.

## Out of scope
- GPU adjustment execution (deferred to GPU-canvas wiring); adjustment-layer *masks*
  (own coverage mask — later); clipping an adjustment to the layer directly below (PS clip);
  curves.

## Design notes
- Adjustment layer opacity = blend amount: `out = lerp(backdrop, map(backdrop), opacity)`,
  per pixel, alpha untouched. Hidden adjustment layer = skipped op.
- Within a group the adjust applies to that group's isolated buffer (only affects siblings
  below in the same group) — falls out of the existing Push/Pop op structure for free.
- Properties panel edits mutate via a `SetAdjustment` command (old/new params) — undoable,
  bumps revision so the canvas recomposites.

## Verification checklist
- [ ] `cargo test -p atelier-core` — Adjustment map_pixel (moved) + SetAdjustment apply/revert
- [ ] `cargo test -p atelier-raster` — adjust ops; compositor adjustment-layer behavior
      (inverts below, not above; opacity lerp; hidden skipped)
- [ ] `cargo test -p atelier-io` — `.atl` round-trips an adjustment layer
- [ ] `cargo test -p atelier-app` — kittest: add adjustment layer changes composite + undo
- [ ] workspace + clippy `--all-targets -D warnings` clean; smoke run

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-13 | `cargo test -p atelier-core` | PASS | 29 tests; Adjustment map_pixel (moved here) + map_pixel_amount lerp; SetAdjustment apply/revert + merge |
| 2026-06-13 | `cargo test -p atelier-raster` | PASS | 27 tests; adjustment layer inverts below not above, respects visibility (hidden=no-op) + opacity lerp; op-list includes Adjust |
| 2026-06-13 | `cargo test -p atelier-io` | PASS | 7 tests; `.atl` round-trips a Levels adjustment layer (params in manifest, deep-equal) |
| 2026-06-13 | `cargo test -p atelier-app` (kittest) | PASS | add Invert adjustment layer over a filled raster → composite inverts below; undo removes it and restores |
| 2026-06-13 | workspace 87 tests + clippy `--all-targets -D warnings` + smoke | PASS | clean |

## Notes / surprises
- `Adjustment` enum + `map_pixel` moved to `atelier-core::adjust` (pure, serde) so
  `NodeKind::Adjustment(Adjustment)` can hold it; `atelier-raster::adjust` keeps
  `apply_tile`/`target_tiles` and re-exports `Adjustment` (app code unchanged).
- GPU compositor skips `CompositeOp::Adjust` (documented no-op) — canvas uses the CPU
  compositor, which applies adjustment layers correctly. Parity debt recorded as RISKS R-13;
  golden fixtures must not include adjustment layers until GPU adjustment lands.
- Properties panel edits adjustment params via merge-coalesced `SetAdjustment` (one undo
  entry per slider drag), recomposite via revision bump.
