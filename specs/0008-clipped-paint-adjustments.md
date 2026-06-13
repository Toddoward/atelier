# Spec 0008 — Selections II: selection-clipped painting + destructive adjustments

- **Status:** ☑ done (2026-06-13)
- **Phase:** 3 (slice b)
- **Requirements:** RAS-3 (selection masks edits), RAS-6 (core adjustments, destructive form first)
- **Depends on:** 0007

## Goal
The active selection actually constrains edits: brush/eraser strokes are masked by the
selection coverage, and a set of core image adjustments (Invert, Brightness/Contrast,
Levels, Hue/Saturation) apply to the selected raster layer within the selection — each a
single undoable command capturing affected tiles. With no selection, edits affect the
whole layer (PS behavior).

## Scope
- `atelier-raster::adjust`: pure per-pixel functions over straight-alpha RGBA8 —
  `invert`, `brightness_contrast(b,c)`, `levels(black,white,gamma)`,
  `hue_saturation(h,s,l)`. Each takes a tile + optional mask coverage and writes in place,
  scaling the change by `coverage/255` (partial selection = partial apply).
- `atelier-raster::brush::stamp_segment` gains an optional `&Mask` clip: per-pixel coverage
  multiplied by mask coverage at that doc pixel (selection-aware painting).
- `atelier-core::command::ApplyAdjust` — captures before/after tiles of the target layer
  over the selection bounds (or whole layer when unselected); reuses the PaintTiles
  snapshot pattern via a shared helper.
- App: an "Adjust" menu (Invert Ctrl+I; Brightness/Contrast…, Levels…, Hue/Saturation…
  dialogs with sliders + live preview-on-apply); operate on the selected raster layer;
  no-op with a status note when the selection excludes the layer entirely.
- Tests: adjust math unit tests (known pixel in/out); clipped brush only paints inside
  mask; ApplyAdjust apply/revert identity; kittest Invert changes pixels + undo; adjust
  restricted to selection leaves outside pixels untouched.

## Out of scope
- Adjustment *layers* (non-destructive — slice c); curves UI (levels covers the gate);
  magic wand, feather/grow/invert-selection (later); selection persistence; color-managed
  adjustments (Phase 6 — math in sRGB-component space for now, consistent with 0003).

## Design notes
- Mask sampling in layer space: a layer-space pixel maps to doc pixel `+ offset`; the clip
  reads `mask.get(doc_x, doc_y)`.
- Adjustments iterate only tiles intersecting the selection bounds (or all layer tiles when
  unselected), reusing the brush capture→mutate→`push_committed` pattern so undo replays
  exact snapshots.
- Hue/sat in HSL; brightness/contrast as `c*(x-0.5)+0.5+b`; levels standard
  `((x-black)/(white-black))^(1/gamma)`.

## Verification checklist
- [ ] `cargo test -p atelier-raster` — adjust math + clipped-brush mask containment
- [ ] `cargo test -p atelier-core` — ApplyAdjust apply/revert identity
- [ ] `cargo test -p atelier-app` — kittest: Invert via menu changes pixels + undo;
      adjust-within-selection leaves outside untouched; suite stays green
- [ ] workspace + clippy `--all-targets -D warnings` clean; smoke run
- [ ] [manual·non-gating] eyes-on adjustment dialogs

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-13 | `cargo test -p atelier-raster` | PASS | 28 tests incl. invert involution, brightness/contrast known values, levels endpoint clamps, hue/sat identity + desaturate-to-gray, `apply_tile` mask clip (inside inverted, outside untouched) |
| 2026-06-13 | `cargo test -p atelier-app` (kittest) | PASS | Invert via `apply_adjustment` changes red→255−r, preserves alpha, undo restores; adjustment within a 20px selection leaves outside pixels untouched |
| 2026-06-13 | workspace 80 tests + clippy `--all-targets -D warnings` + smoke | PASS | clean |

## Notes / surprises
- No separate `ApplyAdjust` command — adjustments are tile pixel-diffs, so they reuse the
  generic `PaintTiles` snapshot command (capture→mutate→`push_committed`) with the
  adjustment's label. Same machinery as brush strokes; one undo entry per adjustment.
- Brush gained `stamp_segment_clipped(.., Option<(&Mask,offset)>)`; `stamp_segment` stays as
  the unclipped convenience wrapper. Canvas clones the selection `Arc` before borrowing
  tiles to satisfy the borrow checker.
- Adjustments target the selected *layer node* (`editor.selection`); the *selection mask*
  (`doc.selection`) restricts which pixels change (whole layer when no mask). Live preview
  in dialogs deferred — apply-on-OK only (noted; not gating).
