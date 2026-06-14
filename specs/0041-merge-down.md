# Spec 0041 — Merge down

- **Status:** ☑ done (2026-06-13)
- **Phase:** 2 follow-up (DOC-2 layer management — merge)
- **Requirements:** DOC-2
- **Depends on:** 0040 (flatten), 0030 (Batch), 0023 (ReplaceNodeKind)

## Goal
Merge the selected raster layer down into the raster layer directly below it (same parent)
into a single raster layer, undoably (Layer → Merge Down, Ctrl+E).

## Scope
- App `merge_down` — composite `[below, selected]` (honoring their blend/opacity) in a temp
  2-layer document, build a raster from the result, then a `Batch`: remove the selected layer,
  swap the below layer's kind to the merged raster (`ReplaceNodeKind`), reset its blend/opacity
  to Normal/100%. Selection moves to the merged layer. No-op unless both are raster layers.
- Layer-menu entry + Ctrl+E.

## Out of scope
- Merging vector or group layers (vectors aren't in the CPU compositor; groups would need
  recursive flatten); merge-visible; merging across different parents.

## Verification checklist
- [x] `cargo test -p atelier-app` — merge red-over-blue → one raster (red covers blue),
      node_count −1, selection on the lower layer; undo restores both
- [x] workspace + clippy `--all-targets -D warnings` clean; smoke run

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-13 | `cargo test -p atelier-app` | PASS | `merge_down_combines_two_rasters_and_undoes` (2→1, merged pixel red, undo→2); app 44 tests |
| 2026-06-13 | workspace + clippy + smoke | PASS | full suite green, clippy clean, app runs 5s no crash |

## Notes / surprises
- Composes a `Batch` of existing commands (RemoveNode + ReplaceNodeKind + SetBlend +
  SetOpacity) — no new command type. The merged pixels are baked, so the lower layer's
  blend/opacity are reset to avoid double-applying them.
- Restricted to raster+raster because the CPU compositor doesn't yet include vector layers
  (they render as an overlay); merging vectors waits on z-interleaved compositing.