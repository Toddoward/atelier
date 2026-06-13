# Spec 0030 — copy / paste layers

- **Status:** ☑ done (2026-06-13)
- **Phase:** 1 follow-up (DOC-2 layer management)
- **Requirements:** DOC-2 (layer management)
- **Depends on:** 0027 (clone_subtree / InsertSubtree)

## Goal
Copy the selected layer and paste independent deep copies above the current selection,
undoably, with Ctrl+C / Ctrl+V and Edit-menu entries.

## Scope
- App `EditorState.clipboard: Option<NodeId>` (same-document source reference).
- `copy_selected_layer` stores the selection; `paste_layer` deep-clones the source fresh each
  time (`clone_subtree` + `InsertSubtree`) above the selection and selects the copy. Multiple
  pastes yield independent copies; pasting a deleted source is a no-op.
- Ctrl+C / Ctrl+V (gated to non-text-editing) + Edit → Copy/Paste Layer.

## Out of scope
- Cross-document paste (single-document app today); OS-clipboard / cross-app paste of pixels;
  pasting into a specific group by drag; copying the document selection mask.

## Verification checklist
- [x] `cargo test -p atelier-app` — copy then paste adds one fresh node (≠ source, source
      intact); a second paste is independent; undo removes both
- [x] workspace + clippy `--all-targets -D warnings` clean; smoke run

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-13 | `cargo test -p atelier-app` | PASS | `copy_paste_layer_and_undo` (paste→+1 fresh node, source intact; 2nd paste→+2; undo×2→0); app 34 tests |
| 2026-06-13 | workspace + clippy + smoke | PASS | full suite green, clippy clean, app runs 5s no crash |

## Notes / surprises
- Clipboard holds the source NodeId and re-clones on each paste (fresh ids via
  `clone_subtree`), so copies are fully independent and a stale source just no-ops.
- Reuses `InsertSubtree` (spec 0027) — same building block as duplicate; an OS-clipboard /
  cross-document path can layer on later.