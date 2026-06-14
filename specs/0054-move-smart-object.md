# Spec 0054 — Move smart objects with the Move tool

- **Status:** ☑ done (2026-06-14)
- **Phase:** 10 (smart objects — placement)
- **Requirements:** DOC-5 (smart objects), RAS-5 (move)
- **Depends on:** 0052 (smart objects embed & composite), 0005 (move tool / SetOffset)

## Goal
A selected smart object can be repositioned by dragging with the Move tool, exactly like a
raster layer — one undoable, drag-coalesced history entry that updates its `SmartContent.offset`
(the offset the compositor already honours, spec 0052).

## Scope
- `atelier-core::command::SetOffset` — generalize from raster-only to `Raster | Smart` via an
  `offset_mut` helper returning `&mut [i32; 2]` for either kind; `new()` reads the old offset
  from either. Merge/label behaviour unchanged.
- `atelier-app` canvas: a `movable_layer` helper (selected, visible, unlocked, `Raster` **or**
  `Smart`) drives the Move-tool arm; `layer_offset` reads the offset from either kind.

## Out of scope
- Editing the embedded document's contents ("Edit Contents") — separate spec.
- Non-destructive scale/rotate of the smart object (only integer translate here).

## Verification checklist
- [x] `cargo test -p atelier-core` — `SetOffset` on a smart object updates its offset and
      reverts on undo
- [x] `cargo test -p atelier-app` — a Move-tool drag on a selected smart object changes its
      offset; undo restores it
- [x] `cargo build/test --workspace`; clippy `--all-targets -D warnings` clean; smoke run

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-14 | `cargo test -p atelier-core` | PASS | `set_offset_moves_smart_object_and_reverts`; core 43 tests |
| 2026-06-14 | `cargo test -p atelier-app` | PASS | `move_tool_drags_smart_object_offset` (drag moves, single undo restores); app 53 tests |
| 2026-06-14 | workspace + clippy + smoke | PASS | full suite green; clippy `--all-targets -D warnings` clean; app alive 12s no crash |

## Notes / surprises
- `SetOffset` already merges per drag; generalizing the accessor keeps one history entry per
  move regardless of layer kind.
