# Spec 0002 — Document model: layer tree, commands/undo, .atl v0

- **Status:** ☑ done (2026-06-12)
- **Phase:** 1
- **Requirements:** DOC-1, DOC-2, DOC-3 (model fields only), DOC-4 (model only), DOC-6, DOC-7 (v0)
- **Depends on:** 0001

## Goal
`atelier-core` holds a real document: a layer tree with groups and typed nodes, every
mutation a undoable `Command`, and save/load to a versioned `.atl` v0 container. The app
gains File → New/Open/Save, a Layers panel that shows the tree and supports
add/delete/rename/reorder/nest/visibility/opacity/blend-mode selection, and a History panel.
Rendering of pixel content is **not** required yet — layers may display as colored
placeholder rects on the canvas.

## Scope
- `atelier-core`: `Document`, `NodeId` (slotmap or generational arena), `Node` enum
  (Raster/Vector/Group/Adjustment/Text/Smart/Fill — non-Group variants may be near-empty
  structs), common `LayerProps { name, visible, locked, opacity, blend, clip }`,
  `BlendMode` enum (full PS set, math deferred to Phase 2), tree ops (insert, remove,
  move-within/into-group, reparent guards against cycles), `Command` trait +
  `History { undo/redo stacks, coalescing hook }`, document-dirty flag.
- `atelier-io`: `.atl` v0 = ZIP(manifest.json) via `zip` + `serde_json`; schema_version
  field; loader rejects future versions cleanly. Start `docs/FORMAT-ATL.md`.
- `atelier-app`: New-document dialog (size, focus chooser stub recording Raster/Vector —
  INT-1 groundwork), Open/Save/Save As with rfd file dialogs, Layers panel (tree view,
  drag-reorder ok to defer to buttons, blend/opacity controls), History panel (list +
  click-to-jump), placeholder canvas rects per layer with selection highlight.
- Unit tests: tree invariants (no cycles, ids stable), undo/redo round-trips for every
  command, save→load→deep-equal, version rejection.

## Out of scope
- Pixel/vector data payloads, masks behavior, smart-object semantics (Phases 2/4/10);
  blend-mode math (Phase 2); tile binary parts in .atl (Phase 2 extends format).

## Design notes
- IDs: `slotmap::SlotMap<NodeKey, Node>` inside `Document`; tree = parent/children links by
  key. Commands store keys + minimal state to revert (D-9 unaffected).
- Command examples: AddLayer, RemoveLayer, MoveNode, SetLayerProp(prop, old, new) with
  coalescing for slider drags (opacity).
- `.atl` manifest: `{ schema_version: 0, size, color_mode: "rgb8" stub, focus, tree: [...] }`.
- New workspace deps: `slotmap`, `serde`, `serde_json`, `zip`, `rfd`.

## Verification checklist
- [ ] `cargo test -p atelier-core` — tree + undo invariants
- [ ] `cargo test -p atelier-io` — round-trip + version rejection
- [ ] `cargo clippy --workspace -- -D warnings`
- [ ] [manual] New doc (both focuses) → add/nest/rename/reorder layers → toggle
      visibility/opacity/blend → undo/redo across all of it → history panel jump
- [ ] [manual] Save .atl, restart app, Open → identical tree state
- [ ] Crate audit: atelier-core still free of wgpu/egui/ort/serde-UI deps

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-11 | `cargo test -p atelier-core` | PASS | 15 tests: tree invariants (cycle rejection, subtree remove/restore identity, panel-order traversal, id non-reuse), command apply/revert identity for all 7 commands, undo/redo round trips, redo-clear, opacity merge, jump_to, dirty tracking |
| 2026-06-11 | `cargo test -p atelier-io` | PASS | 3 tests: save→load deep-equal, future-schema rejection (v99), garbage-file rejection |
| 2026-06-11 | `cargo clippy --workspace -- -D warnings` | PASS | clean after ExtractedSubtree type alias |
| 2026-06-11 | app smoke run | PASS | 10s run, no crash/panic, RTX 3060 via Vulkan |
| 2026-06-11 | crate audit | PASS | atelier-core deps = serde + thiserror only |
| 2026-06-12 | full layer-edit + undo/redo + history-jump UI walkthrough | PASS (automated UI) | egui_kittest, 7 tests: add/group via buttons, row-click select, Into Group/Out/Down reorder-nest, Ctrl+Z undo, History-panel "(document opened)" click-jump + redo jump, double-click rename + type "Background" + Enter (undoable), blend combo → Multiply (undoable), Delete + undo, unsaved-changes guard Discard flow |
| 2026-06-12 | save → restart → open round trip via UI | PASS (automated UI) | `save_then_reopen_restores_identical_tree`: save_to file, fresh app instance, open_from → deep-equal tree (rfd dialogs bypassed via the same code path the dialogs call) |
| 2026-06-12 | dialog flow: New Document → Create | PASS | both kittest (`create_doc`) and live screenshots (dialog rendered, Create produced 1920×1080 Raster doc, title "Atelier — Untitled", Properties panel correct) |

## Notes / surprises
- `AddNode::new` pre-allocates its NodeId, advancing the document id counter at command
  *construction* (ids never reused by design); snapshot-equality tests must snapshot after
  construction. Caught by two failing tests on first run.
- Group expand/collapse is view state mutated directly on the model (one sanctioned
  exception to commands-only; not recorded in history, matching PS behavior).
- Single-document only this phase; multi-doc tabs (SH-4) deferred to a later spec.
- Known gap: blocking rfd dialogs run inside the frame (fine on Windows; revisit for
  macOS/Linux tiers).
- UI deliveries: Layers panel (add/group/delete/reorder/nest via buttons, rename on
  double-click, visibility, blend combo, merging opacity slider), History panel with
  click-to-jump, Properties panel, New-doc dialog with focus chooser (INT-1 groundwork),
  unsaved-changes confirm, error popup, dirty-marker window title, Ctrl+Z/Y/N/O/S(+Shift).
- 2026-06-12: UI test infrastructure added — egui_kittest dev-dep; `AtelierApp::ui(ctx)`
  extracted (frame-independent), `with_adapter_info` headless constructor,
  `open_from`/`save_to` dialog-free file paths. 7 UI tests in `atelier-app::ui_tests`
  drive the real widget tree headlessly; they are the template for all future panel/tool
  verification (CI-safe, no GPU).
- kittest gotchas learned: egui reads modifiers from sticky `RawInput::modifiers` (set it,
  not just the Key event); accesskit `.click()` can't double-click (push raw PointerButton
  events); egui_dock tab titles aren't accesskit nodes (use `dock.find_tab`+`set_active_tab`);
  ComboBox has no label (query `accesskit::Role::ComboBox`).
- Arrow/folder glyphs (↑↓⇒⇐📁) missing from egui default font → replaced with ASCII labels
  ("Up", "Down", "Into Group", "Out", "[G]").
