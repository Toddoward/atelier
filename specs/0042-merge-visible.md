# Spec 0042 — Merge visible

- **Status:** ☑ done (2026-06-13)
- **Phase:** 2 follow-up (DOC-2 layer management — merge visible)
- **Requirements:** DOC-2
- **Depends on:** 0040 (flatten)

## Goal
Merge all visible top-level layers into one raster layer, leaving hidden layers in place,
undoably (Layer → Merge Visible).

## Scope
- `atelier-core::command::MergeVisible` — removes the given top-level `targets`, inserts one
  pre-built raster at the top; revert restores the targets at their original indices (so
  retained hidden layers keep their positions).
- App `merge_visible` — `targets` = visible root children; composite the doc (hidden auto-
  skipped) → raster; apply (no-op < 2 visible). Layer-menu entry.

## Out of scope
- Merging visible inside a specific group only; choosing the merged layer's insert position
  (lands on top); vector layers (CPU compositor scope, as with flatten/merge-down).

## Verification checklist
- [x] `cargo test -p atelier-core` — merge two of three children, hidden retained; revert exact
- [x] `cargo test -p atelier-app` — 2 visible + 1 hidden → merged + hidden; undo restores
- [x] workspace + clippy `--all-targets -D warnings` clean; smoke run

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-13 | `cargo test -p atelier-core` | PASS | `merge_visible_keeps_hidden_layers` (targets removed, hidden kept, revert == baseline); core 42 tests |
| 2026-06-13 | `cargo test -p atelier-app` | PASS | `merge_visible_keeps_hidden_and_undoes` (node_count −1, hidden retained, undo restores); app 45 tests |
| 2026-06-13 | workspace + clippy + smoke | PASS | full suite green, clippy clean, app runs 5s no crash |

## Notes / surprises
- `MergeVisible` removes targets high-index-first and restores low-index-first (the recurring
  structural-command ordering rule) so retained hidden layers' positions reconstruct exactly.
- Composite auto-excludes hidden layers, so the merged raster = exactly the visible result.