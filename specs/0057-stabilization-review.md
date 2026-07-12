# Spec 0057 — Phase 4→10 stabilization review & fixes

- **Status:** ☑ done (2026-07-12)
- **Phase:** cross-cutting (2/4/5/10 hardening)
- **Requirements:** DOC-4 (clipping), DOC-5 (smart objects), RAS-1 (compositor)
- **Depends on:** 0046, 0051, 0052, 0055, 0056

## Goal
Directed review of everything shipped Phase 4 → now; fix the real defects found. Verify the
implementation still tracks the original objectives (architecture invariants, phase gates).

## Review findings

| # | Finding | Severity | Action |
|---|---------|----------|--------|
| 1 | Layers panel offers "Clip to below" for **every** layer kind, but the compositor honors clip only for raster-above-raster (0046 scoped it that way before vectors/smarts had compositor arms in 0051/0052). Clip on a vector/smart layer is a silent no-op, and a clipped **vector** above a raster base breaks the whole clip run (composites unclipped). | HIGH (silent wrong render) | Honor clip for all content layers (Raster\|Vector\|Smart) as both base and clip; shared `blend_content` + `render_layer_isolated` helpers remove the raster-only paths |
| 2 | `apply_transform` smart-object branch skips the `visible && !locked` guard the raster branch enforces — a locked/hidden smart object can be transformed. | MED | Add the same guard |
| 3 | `convert_to_smart` and `rasterize_selected_layer` ignore `locked` — both replace the layer's kind (destructive for rasterize). | MED | Add `!locked` guards |
| 4 | Clip on an **adjustment** layer is still ignored (adjustments re-tone the whole backdrop). | LOW | Documented simplification — deferred, noted here |
| 5 | Architecture invariants audit: core stays GPU/UI-free; all mutations via commands; only gpu imports wgpu; CPU compositor is blend source of truth; GPU parity gap for vector/smart/adjust arms already tracked (R-13). Specs 0052–0056 are Phase-10 slices pulled forward while Phases 5–9 are open — justified by closing R-14 before format freeze; recorded as D-15 (DECISIONS.md also gains a note for the historical duplicate D-13 numbering). | INFO | DECISIONS.md D-15 |

## Scope
- `atelier-raster::compositor`: `blend_content(doc, kind, backdrop, mode, opacity)` shared by
  `composite_node` and a new `render_layer_isolated` (replaces `render_raster_isolated`);
  `composite_children` clip-run detection accepts any content layer (Raster|Vector|Smart) as
  base and as clip member.
- `atelier-app`: guards per findings 2–3.
- `docs/DECISIONS.md`: D-15 (phase-order pull-forward rationale) + duplicate-D-13 note.

## Out of scope
- Clipped adjustment layers (finding 4); GPU parity for non-raster arms (R-13); perf work
  (vector re-rasterized per region composite — known, spec 0051 notes).

## Verification checklist
- [x] `cargo test -p atelier-raster` — clipped raster over a **vector** base clips correctly;
      clipped **vector** over a raster base clips correctly (both previously wrong)
- [x] `cargo test -p atelier-app` — transform on a locked smart object is a no-op; convert/
      rasterize on a locked layer are no-ops
- [x] full existing suite green (no behavior change for raster-only clip docs — golden parity
      fixtures unaffected); clippy `-D warnings`; smoke run

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-07-12 | `cargo test -p atelier-raster` | PASS | `clipping_mask_works_across_layer_kinds` (raster-over-vector + vector-over-raster both clip); old `clipping_mask_limits_layer_to_base_alpha` unchanged; raster 52 tests |
| 2026-07-12 | `cargo test -p atelier-app` | PASS | `locked_layer_blocks_transform_convert_rasterize` (history length unchanged, kind unchanged); app 56 tests |
| 2026-07-12 | full suite + clippy + smoke | PASS | core 45 / raster 52 / io 17 / gpu 4+2 / app 56 green; clippy `--all-targets -D warnings` clean; app alive 12s |

## Notes / surprises
- The refactor made the fix *smaller* than the bug: `blend_content` + `render_layer_isolated`
  replaced three copies of per-kind source construction, and the clip run just swapped
  `is_raster` for `is_content`. Raster-only documents take the identical code path (golden
  parity untouched).
- DECISIONS.md had duplicate D-13 numbering (two decisions, same id, both referenced) — kept
  with an inline note rather than renumbering across 6 referencing files.