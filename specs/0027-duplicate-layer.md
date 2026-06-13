# Spec 0027 — Document: duplicate layer / subtree

- **Status:** ☑ done (2026-06-13)
- **Phase:** 1 follow-up (general layer op; lands now that raster+vector layers carry real content)
- **Requirements:** DOC-2 (layer management) — duplicate
- **Depends on:** 0002

## Goal
Duplicate the selected layer (or group, with all descendants) as a deep copy with fresh
NodeIds, inserted directly above the original, undoably. Ctrl+J and Layer → Duplicate Layer.

## Scope
- `Document::clone_subtree(id, new_parent) -> (new_root, Vec<(NodeId, Node)>)` — DFS copy
  with a fresh-id remap (parent/children relinked, root re-parented), ready for
  `restore_subtree`.
- `atelier-core::command::InsertSubtree` — insert a pre-built subtree (apply = restore,
  revert = remove by root). Reusable for paste/import later.
- App `duplicate_selected_layer` — clone the selection under its parent at the original's
  index (copy lands above), select the copy. Ctrl+J + Layer-menu entry.

## Out of scope
- Cross-document duplicate / copy-paste of layers (needs a clipboard model); duplicating the
  document selection mask; "duplicate into group" drag.

## Verification checklist
- [x] `cargo test -p atelier-core` — clone_subtree gives fresh ids for a group+child;
      InsertSubtree apply inserts, revert removes
- [x] `cargo test -p atelier-app` — Duplicate Layer adds one node, selects the copy, keeps the
      original; undo removes the copy
- [x] workspace + clippy `--all-targets -D warnings` clean; smoke run

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-13 | `cargo test -p atelier-core` | PASS | `insert_subtree_duplicates_with_fresh_ids` (group+child cloned, ids distinct, apply/revert) |
| 2026-06-13 | `cargo test -p atelier-app` | PASS | `duplicate_layer_and_undo` (node_count+1, selection=copy, original kept, undo restores); app 31 tests |
| 2026-06-13 | workspace + clippy + smoke | PASS | full suite green, clippy clean, app runs 5s no crash |

## Notes / surprises
- `InsertSubtree` is a general building block — paste, drag-duplicate, and place-as-copy can
  all reuse it.
- `clone_subtree` allocates fresh ids from the document counter (ids never reused), so a
  duplicate is fully independent including nested smart-object/group structure.