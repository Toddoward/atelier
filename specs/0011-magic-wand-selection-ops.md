# Spec 0011 — Selections IV: magic wand + feather/grow/shrink/invert/all (Phase 3 gate)

- **Status:** ☑ done (2026-06-13)
- **Phase:** 3 (final slice — closes the phase)
- **Requirements:** RAS-3 (magic wand, feather, grow/shrink, invert)
- **Depends on:** 0010

## Goal
Round out selections: a magic-wand tool selects connected pixels by color tolerance, and the
Select menu offers All / Deselect / Invert / Grow / Shrink / Feather. All produce undoable
`SetSelection` changes and render with the existing marching ants. With these, Phase 3's
selection + adjustment toolset is complete and the phase gate closes.

## Scope
- `atelier-core::mask`: `Mask::select_all(size)` and `Mask::inverted(size)` (within the doc
  rect — outside the canvas is never selected).
- `atelier-raster::selection`: `magic_wand(tiles, offset, seed, tolerance, size) -> Mask`
  (BFS flood fill over doc pixels whose RGBA is within `tolerance` of the seed, clamped to
  the canvas); `grow(&Mask, r)` / `shrink(&Mask, r)` (chebyshev morphology); `feather(&Mask, r)`
  (separable box-blur approximation of a gaussian).
- App: Magic Wand tool (W) — primary-click selects (Shift/Alt combine like the marquees);
  Select menu — All (Ctrl+A), Deselect (Ctrl+D, already), Invert (Ctrl+Shift+I), Grow,
  Shrink, Feather… (radius dialog). Tolerance slider in the Tools panel for the wand.
- Tests: core invert/select_all; raster magic-wand (selects a solid region not the
  background), grow/shrink round trip on a rect, feather softens edges; kittest — wand click
  selects + undo, Select All fills, Invert flips.

## Out of scope
- Quick-mask mode; selection persistence in `.atl`; animated ants; per-channel/“contiguous
  off” wand options; refine-edge; magnetic lasso.

## Design notes
- Magic wand BFS bounded by the canvas rect and a visited bitset over the doc area; sample
  the layer at `doc − offset`. Tolerance compares max abs channel delta (incl. alpha).
- Grow/shrink: for each pixel, max/min over a (2r+1)² window (chebyshev) — fine at the small
  radii used; revisit with a distance transform if it ever shows up in profiles.
- Feather: two passes of a box blur of radius r over the 8-bit coverage (separable H then V).
- Invert/all need the doc size (selection is bounded to the canvas), passed from the app.

## Verification checklist
- [ ] `cargo test -p atelier-core` — select_all fills doc rect; inverted flips within rect, 0 outside
- [ ] `cargo test -p atelier-raster` — magic wand selects region not background; grow then
      shrink ≈ original; feather produces partial-coverage edge
- [ ] `cargo test -p atelier-app` — kittest: wand click selects + undo; Select All; Invert
- [ ] workspace + clippy `--all-targets -D warnings` clean; smoke run
- [ ] Phase 3 gate: selection + adjustment toolset complete → flip Phase 3 ☑ in ROADMAP

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-13 | `cargo test -p atelier-core` | PASS | 35 tests; select_all fills doc rect, inverted flips within rect + 0 outside |
| 2026-06-13 | `cargo test -p atelier-raster` | PASS | 35 tests; magic wand selects red region not blue background, grow-then-shrink restores edge, feather yields partial-coverage edge |
| 2026-06-13 | `cargo test -p atelier-app` (kittest) | PASS | wand click selects red half not blue + undo; Ctrl+A select-all; invert-of-all clears |
| 2026-06-13 | workspace 94 tests + clippy `--all-targets -D warnings` + smoke | PASS | clean |
| 2026-06-13 | **Phase 3 gate** | PASS | selection model (0007) + clipped paint/destructive adjust (0008) + adjustment layers (0009) + transform/crop/resample (0010) + wand/feather/grow/shrink/invert/all (0011) — Phase 3 ☑ |

## Notes / surprises
- **Shortcut collision bug, caught by an existing test:** the new global Ctrl+A (Select All)
  swallowed the rename field's select-all-text, breaking `rename_via_double_click_and_typing`.
  Fix: gate the selection/adjust shortcuts (Ctrl+A / Ctrl+D / Ctrl+I / Ctrl+Shift+I) behind
  `!ctx.wants_keyboard_input()` so focused text fields win. Undo/redo/save/new/open stay global.
- Magic-wand click is queued on `EditorState.wand_click` and drained in the app loop — the
  canvas can't borrow the `magic_wand_at` helper (same pattern as other deferred actions).
- Grow/shrink use brute-force chebyshev windows (fine at r≤a few px); swap for a distance
  transform if it ever profiles hot. Feather is a 2-pass box blur (gaussian approximation).
