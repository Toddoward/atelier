# Spec 0005 — Raster engine III: brush/eraser, move tool, canvas resize

- **Status:** ☑ done (2026-06-12)
- **Phase:** 2 (slice c of 3)
- **Requirements:** RAS-2 (brush/eraser core), RAS-4 (move), RAS-5 (canvas resize), DOC-6 (pixel-edit undo)
- **Depends on:** 0004

## Goal
First real editing: a round brush (size/hardness/color) and eraser paint into the selected
raster layer's tiles with full undo; the move tool drags a layer by an offset; Image →
Canvas Size resizes the document — all as history commands, all visible live on the canvas,
all covered by kittest, with GPU parity preserved for offset layers.

## Scope
- `RasterContent.offset: [i32;2]` (`serde(default)` — v1 files load as 0,0); both
  compositors honor it (CPU samples shifted; GPU uploads a CPU-extracted shifted tile via
  `TileMap::extract_shifted`); golden fixtures gain random offsets.
- `atelier-raster::brush`: `BrushParams { radius, hardness, color, erase }`,
  `segment_tiles()` (touched-tile preflight for undo capture), `stamp_segment()`
  (spaced circular stamps, smoothstep hardness falloff, src-over or erase).
- Commands: `PaintTiles` (before/after tile snapshots, committed post-stroke via new
  `History::push_committed`), `SetOffset` (mergeable — one undo step per move-drag),
  `CanvasResize`.
- `History::touch()` — revision bump for live stroke preview (recomposite per stamp).
- App: Tools panel becomes real (Move/Brush/Eraser buttons + size/hardness sliders +
  color picker); canvas routes primary-drag to the active tool (space-drag still pans);
  V/B/E shortcuts; Image → Canvas Size… dialog.
- kittest: paint→pixels→undo; eraser clears alpha; move changes offset + single undo
  step; canvas-resize command + undo. Golden GPU test extended with offsets.

## Out of scope
- Pressure/tablet (RAS-2 P1 part), brush spacing/flow/opacity dynamics, smoothing;
  free transform scale/rotate (next raster spec, backlog noted in ROADMAP); crop *tool*
  and image resample (same); dirty-rect recomposite (perf slice — live painting
  recomposites the whole doc per frame, acceptable at current sizes, logged as perf debt).

## Design notes
- Live stroke mutates tiles directly during the drag (transient preview, like the rename
  buffer) and commits one `PaintTiles` command on release with pre-captured `before`
  tiles — history integrity holds (apply/revert replay the snapshots; redo = after).
  This is the second sanctioned live-edit exception to "UI mutates only via commands".
- Paint math: straight-alpha src-over per pixel in f32, quantized back to u8; eraser
  scales destination alpha by (1 − coverage).
- Stamp spacing radius/3 along the segment; per-stamp coverage =
  `smoothstep(edge0=hardness·r, edge1=r, dist)` inverted.

## Verification checklist
- [ ] `cargo test -p atelier-raster` — brush geometry/falloff/erase unit tests
- [ ] `cargo test -p atelier-core` — PaintTiles/SetOffset/CanvasResize apply-revert
      identity; push_committed redo correctness
- [ ] `cargo test -p atelier-gpu` — golden parity incl. random layer offsets (hardware)
- [ ] `cargo test -p atelier-app` — kittest: brush paints + undo, eraser, move-drag one
      undo step, canvas resize
- [ ] workspace + clippy `--all-targets -D warnings` clean
- [ ] [manual·non-gating] eyes-on stroke feel

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-12 | `cargo test -p atelier-raster` | PASS | 18 tests; brush: solid-center + falloff zone, segment coverage with preflight-tile containment, eraser alpha clear, hard-edge no-fringe |
| 2026-06-12 | `cargo test -p atelier-core` | PASS | 19 tests; SetOffset merge + apply/revert, PaintTiles snapshot apply/revert via app-level brush tests, CanvasResize |
| 2026-06-12 | GPU golden parity incl. random layer offsets | PASS — still **bit-exact** | RTX 3060: 8 docs, offsets ±64, 0 bytes differ (extract_shifted path) |
| 2026-06-12 | kittest UI (5 new) | PASS | brush stroke paints tiles + exactly one history entry + undo/redo; move-drag changes offset with single-step undo; eraser stroke recorded; Canvas Size dialog applies + undoes; composite cache invalidation still green |
| 2026-06-12 | workspace 61 tests + clippy `--all-targets -D warnings` | PASS | clean after two lint fixes (needless lifetime, field-assign-outside-initializer) |
| 2026-06-12 | smoke run | PASS | 6s, no crash, GPU adapter active |

## Notes / surprises
- Stroke→undo integrity verified end-to-end through the UI: pointer-drag → live tile
  mutation → one committed `PaintTiles` → Ctrl+Z restores pre-stroke tiles exactly.
- Move drag merges via the existing `try_merge` machinery (`set_merging` during drag) —
  one undo step per drag confirmed by applied_len assertions.
- Backlog pushed to ROADMAP (Phase 2 remainder): free transform (scale/rotate/skew),
  crop tool, image resample, tablet pressure, GPU-canvas wiring + dirty-rect recomposite
  for the 60 fps gate. Live painting currently recomposites the full doc per stamp frame
  on the CPU — fine at test sizes, the known perf debt for the perf slice.
