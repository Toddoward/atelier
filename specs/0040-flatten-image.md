# Spec 0040 — Flatten image

- **Status:** ☑ done (2026-06-13)
- **Phase:** 1/2 follow-up (DOC layer management — flatten)
- **Requirements:** DOC-2 (layer management)
- **Depends on:** 0006 (compositor), 0032 (from_rgba)

## Goal
Flatten the entire document to a single raster layer (Layer → Flatten Image), undoably.

## Scope
- `atelier-core::command::FlattenDocument` — replaces all root children with one pre-built
  raster node (apply removes+captures children last→first; revert restores first→last at
  original indices). Core stays compositor-free: the app builds the raster.
- App `flatten_document` — composite the doc (`composite_rgba8`), build a raster via
  `TileMap::from_rgba`, apply the command (no-op when ≤1 layer). Layer-menu entry.

## Out of scope
- Merge-down (two adjacent layers — separate spec); flatten preserving a transparent
  background flag; flatten to the active artboard.

## Verification checklist
- [x] `cargo test -p atelier-core` — FlattenDocument replaces the tree with one layer; revert
      restores the full tree (order + ids) exactly
- [x] `cargo test -p atelier-app` — flatten 2 content layers → one raster layer; undo restores
- [x] workspace + clippy `--all-targets -D warnings` clean; smoke run

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-13 | `cargo test -p atelier-core` | PASS | `flatten_replaces_tree_and_reverts` (3 layers → 1, revert == baseline); core 41 tests |
| 2026-06-13 | `cargo test -p atelier-app` | PASS | `flatten_image_and_undo` (2 layers → 1 raster, undo restores 3 nodes); app 43 tests |
| 2026-06-13 | workspace + clippy + smoke | PASS | full suite green, clippy clean, app runs 5s no crash |

## Notes / surprises
- Two bugs caught by the verify gate: (1) removing children front→first reversed their order
  on restore — fixed by removing last→first and restoring first→last; (2) the test snapshotted
  the baseline before `FlattenDocument::new` allocated the raster id (ids never reused), so the
  `next_id` differed — snapshot after construction. Both are recurring patterns now well
  understood.
- Core takes a pre-built raster `Node`, so `atelier-core` never depends on the compositor.