# Spec 0058 — Multiple documents & cross-document paste (INT-4)

- **Status:** ☑ done (2026-07-12)
- **Phase:** 5 (focus modes & interop — closes the INT-1..4 set)
- **Requirements:** INT-4 (copy/paste paths and pixels across documents of either focus)
- **Depends on:** 0030 (copy/paste in-doc), 0052 (smart objects — subtree clone covers them)

## Goal
The app holds several open documents at once (tab strip); a layer copied in one document
pastes into any other — pixels, vector paths, masks, and smart objects alike, regardless of
either document's focus. In-document paste behaves exactly as before.

## Scope
- `atelier-core::Document`: `snapshot_subtree(id)` (owned, root-first, ids as-is) and
  `import_subtree(nodes, new_parent)` (remap every id to fresh local ids, fix links,
  reparent root) — the doc-independent clipboard form and its importer.
- App: `AtelierApp.background: Vec<EditorState>` (inactive docs) + `switch_doc(i)` (swap with
  active; most-recently-active-first tab order). File→New/Open now *push* the current doc to
  the background instead of dropping it. Tab strip under the menu bar (shown when >1 doc);
  "Close Document" File menu item (drops active, activates the next background doc).
- Clipboard moves from `EditorState.clipboard: Option<NodeId>` (dies with its doc) to
  `AtelierApp.clipboard: Option<Vec<(NodeId, Node)>>` (owned snapshot, survives doc switch
  and doc close). Copy = snapshot; paste = import + existing `InsertSubtree` command. One
  code path for in-doc and cross-doc paste.

## Out of scope
- Dirty-check prompt on Close Document (no such prompt exists for Exit either — consistent).
- OS-clipboard interchange with other apps (own format only); PSD/AI clipboard.
- Per-tab close buttons / drag-reorder (menu close only).

## Verification checklist
- [x] `cargo test -p atelier-core` — `import_subtree` remaps overlapping ids, preserves
      structure/content, advances `next_id`
- [x] `cargo test -p atelier-app` — copy a pixel layer in doc A → File>New → paste into doc B
      (pixels present, undoable); same for a vector layer (paths survive); switch back to A
      intact
- [x] full suite green; clippy `--all-targets -D warnings`; smoke run

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-07-12 | `cargo test -p atelier-core` | PASS | `import_subtree_remaps_ids_and_content` (overlapping id spaces, group structure + pixels intact); core 46 tests |
| 2026-07-12 | `cargo test -p atelier-app` | PASS | `cross_document_copy_paste_pixels_and_paths` (pixels A→B undoable, paths B→A, tab switch intact); old in-doc `copy_paste_layer_and_undo` unchanged on the unified path; app 57 tests |
| 2026-07-12 | full suite + clippy + smoke | PASS | core 46 / raster 52 / io 17 / gpu 4+2 / app 57 green; clippy clean; app alive 12s |

## Notes / surprises
- Replacing the per-doc `Option<NodeId>` clipboard with an owned snapshot *unified* in-doc and
  cross-doc paste into one code path (snapshot → import → InsertSubtree) — the in-doc special
  case and its "source still exists?" check were deleted, not preserved.
- Tab order is "swap" semantics (switching to a background doc leaves the old active doc in
  that slot), not MRU — deterministic and one line; revisit only if it bothers real use.