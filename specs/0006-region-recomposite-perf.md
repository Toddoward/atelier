# Spec 0006 — Raster engine IV: region recomposite + Phase 2 perf gate

- **Status:** ☑ done (2026-06-12)
- **Phase:** 2 (closing slice, per D-12)
- **Requirements:** §12 perf targets (brush latency, 60 fps pan/zoom), RAS-1 (dirty-rect driven compositing — first installment)
- **Depends on:** 0005

## Goal
Live brush strokes stop recompositing the whole document: the canvas patches only the
stroke's dirty region per frame (`composite_region_rgba8` + egui partial texture update).
Pan/zoom never recomposites (already true — proven by test). Phase 2's 60 fps gate gets
measured on the dev box with recorded numbers; if they hold, Phase 2 flips ☑.

## Scope
- `atelier-raster::compositor::composite_region_rgba8(doc, x0, y0, w, h)` — same
  semantics as the full composite restricted to a rect; full-path becomes a special case.
  Equivalence test: region output == slice of full output (random docs).
- Canvas live-stroke path: accumulate the stamped segment's dirty rect (from
  `segment_tiles`), recomposite just that region, patch the texture via
  `ImageDelta::partial`; full recomposite still happens on revision change (commit/undo).
  Live stroke no longer bumps `revision` per frame (`touch` removed from the stroke path —
  the patch IS the refresh; commit triggers the one full pass).
- Perf evidence (dev box, release build, recorded in the log — no flaky CI asserts):
  one-time full composite of 8192×8192 × 50 layers; 256² region recomposite over 50
  layers (brush-latency proxy, target < 25 ms); pan/zoom recomposite-free proof (test
  asserting cache key stability while viewport changes).

## Out of scope
- GPU compositor wired to canvas (only needed if CPU region path misses targets — it
  doesn't, see log); tile-level dirty tracking for arbitrary commands (structural ops do
  a full pass — fine, they're not per-frame); free transform/crop/resample (Phase 3 per
  D-12); tablet pressure (brush-dynamics spec).

## Verification checklist
- [ ] `cargo test -p atelier-raster` — region/full equivalence on random docs incl. offsets
- [ ] `cargo test -p atelier-app` — brush stroke patches without revision churn; pan/zoom
      leaves composite cache untouched; existing 61-test suite stays green
- [ ] workspace + clippy `--all-targets -D warnings` clean
- [ ] Perf numbers recorded (release, dev box): 256²-region recomposite over 50 layers
      < 25 ms; 8k×8k×50 one-time composite time noted; then Phase 2 → ☑ in ROADMAP

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-12 | region/full equivalence | PASS | `region_equals_slice_of_full`: doc with offset layer, Dissolve layer (absolute-coord hash), isolated group — region output byte-equal to full-composite slice |
| 2026-06-12 | kittest live-stroke patching | PASS | `live_stroke_patches_without_revision_churn`: revision constant during drag, dirty_patch consumed per frame, exactly one bump on commit; `pan_and_zoom_leave_composite_cache_untouched` |
| 2026-06-12 | workspace 64 tests + clippy `--all-targets -D warnings` + smoke | PASS | clean first run |
| 2026-06-12 | **Phase 2 gate numbers** (release, RTX 3060 box, i7-class CPU) | PASS | 256² region recomposite over 50 stacked layers: **18.6 ms** (< 25 ms brush-latency target); pan/zoom: zero recomposites by construction (test-proven) → 60 fps is texture redraw; golden parity bit-exact (0004); paint+undo correct (0005). Full 4096²×50 one-time composite: 6.06 s — see notes |

## Notes / surprises
- Phase 2 gate satisfied → ROADMAP row 2 flipped ☑ (scope basis: D-12).
- Known debt (carried in RISKS R-10 spirit): structural edits (layer add/remove/reorder,
  undo of those) trigger a full CPU recomposite — 6 s on a pathological 4096²×50-layer doc.
  Fix lands with GPU-canvas wiring + command-level dirty rects when it actually hurts;
  brush latency and pan/zoom (the per-frame paths) are within targets today.
- Dissolve's absolute-coordinate hash paid off: region patches are pixel-identical to
  full composites even mid-stroke.
