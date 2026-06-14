# Spec 0046 — Clipping masks

- **Status:** ☑ done (2026-06-13)
- **Phase:** 2/3 (DOC-4 clipping masks)
- **Requirements:** DOC-4 (clipping masks)
- **Depends on:** 0006 (compositor)

## Goal
A layer marked "clip to below" only shows where the raster layer beneath it (the clip base)
is opaque, composited within that base's alpha. Toggle in the Layers panel; renders live.

## Scope
- CPU compositor: a run of `clip` raster layers above a non-clip raster base is rendered into
  isolated buffers, each masked by the base's per-pixel alpha, composited onto the base, then
  the whole group blends to the backdrop with the base's blend mode. Non-clip docs use the
  existing direct path unchanged (golden parity preserved).
- `atelier-core::command::SetClip` (prop command on `LayerProps.clip`); Layers-panel
  "Clip to below" checkbox.

## Out of scope
- Clipping with a group or vector/adjustment base (raster base only this slice); GPU
  compositor clip (parity path stays non-clip — R-13 family); clip indicator arrow icon.

## Verification checklist
- [x] `cargo test -p atelier-raster` — clip layer visible over base alpha, hidden where base
      is transparent
- [x] workspace + clippy `--all-targets -D warnings` clean; smoke run; golden parity still
      bit-exact (clip path is additive)

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-13 | `cargo test -p atelier-raster` | PASS | `clipping_mask_limits_layer_to_base_alpha` (red clip over half-green base → red left, transparent right); raster 46 tests |
| 2026-06-13 | workspace + clippy + smoke | PASS | app 48 / core; clippy clean; golden parity unaffected; app runs 5s |

## Notes / surprises
- Clip only triggers the buffered path when a raster layer actually has clip=true above a
  raster base, so all existing (non-clip) documents — including the GPU golden fixtures —
  composite via the unchanged direct path; parity stays bit-exact.
- Borrow note: read all `LayerProps` fields (incl. `clip`) into locals before the
  apply-closures in the Layers panel (a late `node.props` read revives the borrow).