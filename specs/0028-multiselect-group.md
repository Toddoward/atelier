# Spec 0028 — Document: node multi-select + group / ungroup

- **Status:** ☑ done (2026-06-13)
- **Phase:** 1 follow-up (DOC-2 group management; unblocks cross-layer ops)
- **Requirements:** DOC-2 (group management)
- **Depends on:** 0027

## Goal
Select multiple layers (shift/ctrl-click in the Layers panel), group them under a new group
(Ctrl+G), and ungroup a selected group (Ctrl+Shift+G) — all undoable.

## Scope
- `Document::set_children_order` (permutation reorder) — supports exact group-undo.
- `atelier-core::command::GroupNodes` (members must share a parent; group takes the topmost
  member's slot; members keep relative order) and `UngroupNode` (children replace the group
  at its slot; revert restores the group with its props).
- App: additive multi-select — `EditorState.selected_extra: Vec<NodeId>` beside the primary
  `editor.selection`; shift/ctrl-click toggles extras; stale extras pruned each frame.
  `group_selected` / `ungroup_selected`; Ctrl+G / Ctrl+Shift+G; Layer-menu entries.

## Out of scope
- Cross-layer align/distribute using the multi-selection (next, builds on this); marquee
  rubber-band selection in the layer list; reordering multi-selection by drag; copy/paste of
  the multi-selection (reuses InsertSubtree, later).

## Verification checklist
- [x] `cargo test -p atelier-core` — group (non-contiguous members) + ungroup round trip;
      group rejects cross-parent members; undo restores original order
- [x] `cargo test -p atelier-app` — group two layers (one primary + one extra) → group node
      with both; ungroup; undo re-groups then removes
- [x] workspace + clippy `--all-targets -D warnings` clean; smoke run

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-13 | `cargo test -p atelier-core` | PASS | `group_and_ungroup_round_trip` (group a+c, members ordered, ungroup→[a,c,b], undo chain→[a,b,c]); `group_rejects_cross_parent_members`; core 39 tests |
| 2026-06-13 | `cargo test -p atelier-app` | PASS | `group_and_ungroup_layers_via_app` (2 layers via primary+extra → group, ungroup, undo×2); app 32 tests |
| 2026-06-13 | workspace + clippy + smoke | PASS | full suite green, clippy clean, app runs 5s no crash |

## Notes / surprises
- Multi-select is additive (primary `Option<NodeId>` + `selected_extra` vec) — no churn to the
  many existing single-selection call sites; `selected_node_set()` is the unified accessor.
- Ungroup places the group's contents at the group's slot (not the pre-group order); the
  pre-group order returns only by undoing the *group* command.
- `set_children_order` validates a true permutation (reuses `TreeError::NotFound` to signal
  a mismatch) so group-undo can't silently corrupt the tree.