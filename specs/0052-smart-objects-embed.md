# Spec 0052 — Smart objects: embed & composite

- **Status:** ☑ done (2026-06-14)
- **Phase:** 2/5 (DOC-5 smart objects — first slice)
- **Requirements:** DOC-5 (embedded document layer), RAS-1 (compositor)
- **Depends on:** 0051 (z-interleaved compositing), 0046 (group/clip compositing)

## Goal
A layer can be converted into a **smart object** that embeds an independent document; the
embedded document composites recursively (in place, at an offset) through the CPU compositor.
This is the create+composite slice — persistence of embedded pixels and editing the embedded
contents come in follow-ups (mirrors masks: create 0047 → persist 0048 → edit 0049/0050).

## Scope
- `atelier-core`: `SmartContent { doc: Box<Document>, offset: [i32; 2] }`; turn the
  `NodeKind::Smart` stub into `NodeKind::Smart(SmartContent)`. Re-export `SmartContent`.
- `atelier-raster::compositor`: a `NodeKind::Smart` arm — composite the embedded document's
  tree into an offset-shifted isolated buffer (so nested groups/clips/vectors/smart objects
  all work via recursion), then blend onto the backdrop with the smart layer's blend/opacity.
- `atelier-app`: `convert_to_smart` — wrap the selected non-group layer's content in a fresh
  embedded `Document` (same size/focus, one layer carrying the original content at Normal/1.0),
  then `ReplaceNodeKind` the selected node to `Smart(...)`. Undo restores the original kind.
  Layer-menu "Convert to Smart Object" entry (enabled for a non-group, non-smart selection).

## Out of scope
- Persisting embedded-doc pixels/masks in `.atl` (embedded tree structure serializes inline,
  but tiles are `#[serde(skip)]`) — **deferred to spec 0053** (schema v3, embedded parts).
- Editing the embedded document in place / "Edit Contents" (deferred).
- Non-destructive transform of the smart object beyond integer offset (scale/rotate later).
- GPU-path smart compositing (CPU compositor only; R-13).

## Verification checklist
- [x] `cargo test -p atelier-raster` — an embedded document composites at its offset and in
      z-order with sibling layers
- [x] `cargo test -p atelier-app` — convert-to-smart wraps a layer (composite unchanged
      pixels) and undo restores the original `NodeKind`
- [x] `cargo build/test --workspace`; clippy `--all-targets -D warnings` clean; smoke run

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-14 | `cargo test -p atelier-raster` | PASS | `smart_object_composites_embedded_doc_at_offset` (red square shifted to offset), `smart_object_opacity_applies_once` (alpha 128 once) |
| 2026-06-14 | `cargo test -p atelier-app` | PASS | `convert_to_smart_wraps_and_undoes` (composite unchanged, embedded holds 1 layer, undo → Raster); app 52 tests |
| 2026-06-14 | workspace + clippy + smoke | PASS | core 42, raster 49, io 15, gpu 4+2, app 52 green; clippy clean; app alive 12s no crash |

## Notes / surprises
- The embedded layer is held at Normal/opacity 1.0 so blend/opacity apply exactly once — at
  the smart-object node when its buffer blends onto the backdrop.
- Recursion reuses `composite_children`, so a smart object can itself contain groups, clipping
  runs, vectors, or further smart objects with no extra code.
