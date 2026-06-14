# Session state — resume point

> **Always current.** Update before ending any session (CLAUDE.md hard rule).
> Cold start: read this, then ROADMAP.md, then the active spec.

## Last session: 2026-06-13-ap (spec 0048 — persist layer masks DONE)

### Done
- **Spec 0048 ☑** — `.atl` schema **v2**: layer masks persisted as `masks/<id>.bin` parts
  (`Mask::to_region_bytes`/`from_region_bytes`); v0/v1 files still load. Closes R-14 for masks
  (embedded smart-object docs remain). io 15 tests green (mask round-trip), clippy clean, smoke.
  FORMAT-ATL.md updated to v2.

### Next
1. **Paint-on-mask** edit mode; **smart objects** (DOC-5 — embedded doc + its own .atl parts).
2. **z-interleaved compositing**, **INT-4 cross-paste**.
3. **Phase 6 color management** (lcms2 — liblcms2-dev on ubuntu CI or vendor; verify
   cross-platform — big gated item).

### Watch out (additions)
- `.atl` is schema v2 now; the loader handles v0/v1/v2. Mask parts are additive (no JSON
  migration). When adding more skipped payloads, follow the same binary-part pattern + bump.

## Previous session: 2026-06-13-ao (spec 0047 — layer masks DONE)

### Done
- **Spec 0047 ☑** — layer masks (DOC-4): `RasterContent.mask: Option<Mask>` (serde-skip),
  compositor multiplies layer alpha by mask coverage; `SetLayerMask` command + Layer menu
  (Add from Selection / Remove). Session-only persistence (R-14 logged). app 49 tests green,
  clippy clean, smoke clean.

### Next
1. **Paint-on-mask** edit mode (brush edits the active layer mask); **smart objects** (DOC-5).
2. **`.atl` persistence for masks + smart-object docs** (R-14 — needed before Phase-7 freeze).
3. **z-interleaved compositing**, **INT-4 cross-paste**, **Phase 6 color management** (lcms2 —
   liblcms2-dev on ubuntu CI or vendor; verify cross-platform — big gated item).

### Watch out (additions)
- New R-14: masks (and future embedded smart-object docs) are serde-skip / session-only; the
  `.atl` writer must add parts for them before any format freeze.
- Adding fields to RasterContent: update the one explicit struct literal (command.rs) — most
  sites use `..Default::default()`.

## Previous session: 2026-06-13-an (spec 0046 — clipping masks DONE)

### Done
- **Spec 0046 ☑** — clipping masks (DOC-4): CPU compositor clips a run of `clip` raster
  layers to the raster base below (isolated buffers masked by base alpha; non-clip docs use
  the unchanged direct path so golden parity stays bit-exact). `SetClip` command + Layers-panel
  "Clip to below" checkbox. raster 46 / app 48 tests green, clippy clean, smoke clean.

### Next
1. **Layer/vector masks** (DOC-4 remainder — a paintable mask per layer); **smart objects**
   (DOC-5); **z-interleaved compositing** (Phase 5).
2. **INT-4 cross-paste**; **Phase 6 color management** (lcms2 — liblcms2-dev on ubuntu CI or
   vendor; verify cross-platform — big gated item).

### Watch out (additions)
- Clip path only engages when a raster layer has clip=true above a raster base; everything
  else (incl. GPU golden fixtures) uses the direct compositor path. Read all LayerProps fields
  into locals before apply-closures in panels (borrow revival).

## Previous session: 2026-06-13-am (spec 0045 — selection to vector DONE)

### Done
- **Spec 0045 ☑** — Select → To Vector Path (INT-5 reverse): `selection::boundary_paths`
  chains marching-squares segments into simplified closed loops; app `selection_to_vector`
  builds an even-odd Path → Vector layer, undoable. Both INT-5 directions now covered.
  raster +1 / app 48 tests green, clippy clean, smoke clean.

### Next
1. **INT-4 cross-paste** (needs a layer clipboard across docs — currently single-doc;
   in-doc copy/paste exists via spec 0030).
2. **Smart objects** (DOC-5 — embedded Box<Document>, recursive compositor + .atl handling).
3. **z-interleaved raster+vector compositing** (Phase 5); **Phase 6 color management** (lcms2 —
   liblcms2-dev on ubuntu CI or vendor; verify cross-platform — big gated item).

### Watch out (additions)
- `boundary_paths` output is rectilinear (pixel-accurate), not curve-fitted. Selection holes
  become inner loops resolved by even-odd fill.

## Previous session: 2026-06-13-al (spec 0044 — selection from layer DONE)

### Done
- **Spec 0044 ☑** — Select → From Layer (INT-5): `selection_from_layer` builds a doc selection
  from the selected layer's alpha (raster tiles, offset aware; vector via rasterize_vector AA),
  undoable. app 47 tests green, clippy clean, smoke clean.

### Next
1. **INT-5 reverse** (selection→vector path) + combine-with-existing; **INT-4 cross-paste**.
2. **Smart objects** (DOC-5 — embedded Box<Document>; needs recursive compositor + .atl tile
   handling, sizable); **z-interleaved raster+vector compositing** (Phase 5).
3. **Phase 6 color management** (lcms2 — liblcms2-dev on ubuntu CI or vendor; verify
   cross-platform — the big gated item).

### Watch out (additions)
- selection_from_layer replaces the current selection (no add/subtract yet). Vector path uses
  AA rasterization → soft selection edges.

## Previous session: 2026-06-13-ak (spec 0043 — pattern fill DONE)

### Done
- **Spec 0043 ☑** — define pattern + pattern fill: `fill::fill_pattern` (tiled, anchored to
  doc origin) + `Mask::tight_bounds` (pixel-exact extent) + app `define_pattern`/
  `fill_with_pattern` (Edit menu, `EditorState.pattern`). RAS-9 fill set now complete:
  solid/linear/radial/flood/pattern. raster +1 / core +1 / app 46 tests green, clippy clean.

### Next
1. **INT-4 cross-paste** (pixels/paths); **smart objects** (DOC-5); **z-interleaved
   raster+vector compositing** (Phase 5 — also unblocks vector merge).
2. **Phase 6 color management** (lcms2 — liblcms2-dev on ubuntu CI or vendor; verify
   cross-platform — the big gated item).
3. Brush dynamics (flow/spacing/pressure); PSD import (Phase 8, large).

### Watch out (additions)
- `Mask::bounds()` is tile-granular; use `Mask::tight_bounds()` when you need pixel-exact
  selection extent (pattern definition, future crop-to-selection-exact, etc.).

## Previous session: 2026-06-13-aj (spec 0042 — merge visible DONE)

### Done
- **Spec 0042 ☑** — Merge Visible: `command::MergeVisible` merges visible top-level layers
  into one raster (composite auto-skips hidden), keeping hidden layers in place; revert
  restores targets at original indices. Layer → Merge Visible. core 42 / app 45 tests green,
  clippy clean, smoke clean.

### Next
1. **Pattern fill** (RAS-9 last bit); merge-down for vectors once z-interleaved.
2. **INT-4 cross-paste**, **smart objects** (DOC-5), **z-interleaved compositing** (Phase 5).
3. **Phase 6 color management** (lcms2 — liblcms2-dev on ubuntu CI or vendor; verify
   cross-platform — the big gated item).

### Watch out (additions)
- Structural multi-node commands (Flatten/MergeVisible/Group): remove high-index-first,
  restore low-index-first; snapshot test baselines AFTER `::new` (id alloc).

## Previous session: 2026-06-13-ai (spec 0041 — merge down DONE)

### Done
- **Spec 0041 ☑** — Merge Down (Ctrl+E / Layer menu): `merge_down` composites
  [below, selected] in a temp 2-layer doc, builds a raster, and applies a Batch
  (RemoveNode + ReplaceNodeKind + reset blend/opacity). Raster+raster only (vectors aren't in
  the CPU compositor yet). app 44 tests green, clippy clean, smoke clean.

### Next
1. **Pattern fill**; **merge-visible** (no dep).
2. **INT-4 cross-paste**, **smart objects** (DOC-5), **z-interleaved compositing** (Phase 5 —
   would also unblock merging vector layers).
3. **Phase 6 color management** (lcms2 — liblcms2-dev on ubuntu CI or vendor; verify
   cross-platform — the big gated item).

### Watch out (additions)
- Merge/flatten via temp-doc composite then Batch of existing commands. Merged pixels bake in
  blend/opacity, so reset the surviving layer's props to Normal/100%.

## Previous session: 2026-06-13-ah (spec 0040 — flatten image DONE)

### Done
- **Spec 0040 ☑** — Flatten Image: `command::FlattenDocument` (replaces tree with one
  pre-built raster; remove last→first / restore first→last for correct order; app builds the
  raster via composite_rgba8 + from_rgba so core stays compositor-free). Layer → Flatten Image,
  undoable. core 41 / app 43 tests green, clippy clean, smoke clean.

### Next
1. **Merge down** (two adjacent layers); pattern fill (RAS-9).
2. **INT-4 cross-paste**, **smart objects** (DOC-5), **z-interleaved compositing** (Phase 5).
3. **Phase 6 color management** (lcms2 — liblcms2-dev on ubuntu CI or vendor; verify
   cross-platform — the big gated item).

### Watch out (additions)
- Structural multi-node commands: remove children last→first, restore first→last (else order
  reverses). Snapshot test baselines AFTER any `::new` that allocs a NodeId.

## Previous session: 2026-06-13-ag (spec 0039 — radial gradient DONE)

### Done
- **Spec 0039 ☑** — radial gradient: `gradient_region_radial` + `BrushSettings.gradient_radial`
  toggle (Tools panel); `apply_gradient` dispatches linear/radial. raster +1 / app 42 tests
  green, clippy clean, smoke clean. RAS-9 fills now: solid/linear/radial/flood (patterns left).

### Next
1. **Pattern fill** + multi-stop/angular gradients (RAS-9 polish, no dep).
2. **INT-4 cross-paste**, **smart objects** (DOC-5), **z-interleaved compositing** (Phase 5).
3. **Phase 6 color management** (lcms2 — liblcms2-dev on ubuntu CI or vendor; verify
   cross-platform — the big gated item).

### Watch out (additions)
- Gradient tool has a Radial toggle in `BrushSettings.gradient_radial`; both modes share the
  drag + PaintTiles plumbing.

## Previous session: 2026-06-13-af (spec 0038 — paint bucket DONE)

### Done
- **Spec 0038 ☑** — Paint Bucket tool (key `K`): `apply_bucket` composes `magic_wand` (flood
  select on active layer, brush tolerance) + `fill_region` + `PaintTiles` undo. app 41 tests
  green, clippy clean, smoke clean. RAS-9 fill set complete (solid/gradient/flood; patterns +
  radial gradients remain).

### Next
1. **Radial/multi-stop gradients**, pattern fill (RAS-9 polish).
2. **INT-4 cross-paste**, **smart objects** (DOC-5), **z-interleaved compositing** (Phase 5).
3. **Phase 6 color management** (lcms2 — add liblcms2-dev to ubuntu CI apt or vendor; verify
   cross-platform first — the big gated item).

### Watch out (additions)
- Bucket = magic_wand + fill_region + PaintTiles (no new kernel); contiguous + same-layer.

## Previous session: 2026-06-13-ae (spec 0037 — gradient fill DONE)

### Done
- **Spec 0037 ☑** — Gradient tool (key `G`): `atelier-raster::fill::gradient_region` (two-stop
  linear, mask+offset aware) + canvas drag-axis (live preview) → foreground→transparent fill
  via `apply_gradient`, undoable. raster +1 / app 40 tests green, clippy clean, smoke clean.

### Next
1. **Radial/multi-stop gradients**, **flood-fill (contiguous) bucket** (RAS-9 remainder).
2. **INT-4 cross-paste**, **smart objects** (DOC-5), **z-interleaved compositing** (Phase 5).
3. **Phase 6 color management** (lcms2 — add liblcms2-dev to ubuntu CI apt or vendor; verify
   cross-platform first — the big gated item).

### Watch out (additions)
- Gradient uses select_drag for the axis + PaintTiles for undo; only the fill kernel is new.
  Foreground→transparent single-color default (no gradient stop UI yet).

## Previous session: 2026-06-13-ad (spec 0036 — fill selection DONE)

### Done
- **Spec 0036 ☑** — fill selection/layer with brush color: `atelier-raster::fill::fill_region`
  (mask+offset-aware, coverage-blended) + app `fill_selection` (Edit → Fill with Color),
  undoable via PaintTiles. raster +3 / app 39 tests green, clippy clean, smoke clean.

### Next
1. **Gradient & pattern fills** (RAS-9 remainder); **flood-fill** (contiguous) bucket.
2. **INT-4 cross-paste**, **smart objects** (DOC-5), **z-interleaved compositing** (Phase 5).
3. **Phase 6 color management** (lcms2 — system lib; add `liblcms2-dev` to ubuntu CI apt or
   vendor; verify cross-platform before committing — the big gated item).

### Watch out (additions)
- Fill reuses `PaintTiles` (capture touched tile range → fill → push_committed). Feathered
  selections fill soft via coverage-scaled alpha.

## Previous session: 2026-06-13-ac (spec 0035 — eyedropper DONE)

### Done
- **Spec 0035 ☑** — Eyedropper tool (key `I`): `canvas::sample_composite` reads the
  composited pixel under the cursor into brush.color + vector_fill. app 38 tests green
  (green/transparent/out-of-bounds sampling), clippy clean, smoke clean.

### Next
1. **INT-4 cross-paste**, **smart objects** (DOC-5), **z-interleaved compositing** —
   the remaining Phase-5 interop items.
2. **Phase 6 color management** (lcms2) — system lib. CI: add `liblcms2-dev` to the ubuntu
   apt step (or vendor) and verify cross-platform before committing. This is the big gated
   item; eyedropper/picker readouts (Lab/CMYK) come with it.
3. Brush dynamics (flow/spacing/pressure), gradients/fill bucket (RAS-9), merge-down.

### Watch out (additions)
- Eyedropper samples the composite (all layers), not the active layer. Tool key `I` is plain;
  `Ctrl+I` remains Invert (guarded by modifier check).

## Previous session: 2026-06-13-ab (spec 0034 — TIFF/WebP/GIF/BMP DONE)

### Done
- **Spec 0034 ☑** — added `image` features tiff/webp/gif/bmp; `IMPORT_EXTENSIONS`/
  `EXPORT_EXTENSIONS` constants drive Place/Export dialogs; decode is format-agnostic, save
  infers from extension. io 14 tests (TIFF+BMP lossless round-trip), clippy clean, smoke clean.
  FMT-4 raster codecs complete (sans ICC).

### Next
1. **INT-4 cross-paste** (pixels/paths across the doc); **smart objects** (DOC-5);
   **z-interleaved raster+vector compositing**.
2. **Phase 6 color management** (lcms2) — system lib. Before committing: the Ubuntu CI
   runner needs `liblcms2-dev` (add to the apt step in .github/workflows/ci.yml) or use a
   vendored/pure-Rust alternative. Verify cross-platform build first.
3. PSD import (Phase 8) eventually — big.

### Watch out (additions)
- Image format support is feature-gated in the workspace `image` dep; GIF/WebP first-frame
  only. ICC not handled yet.

## Previous session: 2026-06-13-aa (spec 0033 — export PNG/JPEG DONE)

### Done
- **Spec 0033 ☑** — export flattened doc to PNG/JPEG: `atelier-io::encode_png`/`save_image`
  (PNG keeps alpha, JPEG→RGB, buffer-validated); app `export_to`/`export_image_dialog`
  (File → Export Image…) composites via `composite_rgba8` then writes. io 13 / app 37 tests
  green, clippy clean, smoke clean. Round-trip with Place (0032) works (place→export→reload).

### Next
1. **Remaining FMT-4 formats** (TIFF/WebP/GIF/BMP) — add `image` features + extend
   decode/save filters; quick, no new dep.
2. **INT-4 cross-paste**, **smart objects** (DOC-5), **z-interleaved compositing**.
3. **Phase 6 color management** (lcms2 — system lib; verify CI has it or vendor; may need
   apt liblcms2-dev on the ubuntu runner — check before committing).

### Watch out (additions)
- Export reuses the CPU `composite_rgba8` (shares all blend/adjustment handling with canvas).
  ICC-tagged export deferred to Phase 6.

## Previous session: 2026-06-13-z (spec 0032 — place image / INT-3 DONE; Phase 5 started)

### Done
- **Spec 0032 ☑** — place image (INT-3): new dep **image 0.25** (png+jpeg) on atelier-io;
  `image_io` (`DecodedImage`, `decode_image`/`load_image`); `TileMap::from_rgba`; app
  `place_image` / `place_image_dialog` (File → Place Image…) inserts a raster layer
  (undoable). io 10 / app 36 tests green, clippy clean, smoke clean.

### Next — Phase 5 continues
1. **Interactive place** (position/scale of the placed image) + remaining FMT-4 formats
   (TIFF/WebP/GIF/BMP — `image` features).
2. **INT-4 cross-paste** pixels/paths; **smart objects** (DOC-5, embedded sub-doc +
   non-destructive transform — uses ReplaceNodeKind / InsertSubtree groundwork).
3. **z-interleaved raster+vector compositing** (vectors currently overlay the raster
   composite; needs canvas render reorder or compositor rasterizing vectors inline — design pass).
4. Or pivot to **Phase 6 color management** (lcms2).

### Watch out (additions)
- Placed image lands at doc origin, unscaled. `TileMap::from_rgba` skips transparent pixels.
- `image` crate decode is in atelier-io; keep ICC handling for Phase 6.

## Previous session: 2026-06-13-y (spec 0031 — boolean path ops DONE; **PHASE 4 COMPLETE**)

### Done
- **Spec 0031 ☑** — boolean Pathfinder (VEC-5): new dep **i_overlay 2.2.0** in atelier-vector;
  `atelier-vector::boolean` (`BoolOp` Union/Intersect/Difference/Exclude; flatten cubics →
  `i_overlay` overlay (NonZero) → line `Path`, compound-aware). App `panels::pathfinder` folds
  the op across a vector layer's shapes (undoable), Properties buttons. vector +5 / app 35
  tests green, clippy clean, smoke clean.
- **Phase 4 vector engine is COMPLETE** (path model, tessellation, GPU render, all shape
  tools, pen + full anchor/handle editing, align/distribute, compound paths, booleans) +
  INT-2 rasterize w/ AA + full layer mgmt (duplicate/multi-select/group/copy-paste).
- Dep-integration method that worked: add dep → `cargo fetch` → read crate source in
  `~/.cargo/registry/src` → code to real API. Use this for future new deps.

### Next — pick a phase
1. **Phase 5 — focus modes & raster↔vector interop**: New-doc focus chooser exists (INT-1
   groundwork); remaining INT-2 done; **INT-3 place image** (needs `image` decode + rfd file
   dialog — add `image` dep, same fetch+read method), INT-4 cross-paste, smart objects
   (DOC-5), z-interleaved raster+vector compositing (vectors currently overlay).
2. **Phase 6 — color management** (lcms2): working spaces, assign/convert, picker.
3. Boolean polish: re-fit curves to results; cross-layer boolean.

### Watch out (additions)
- New deps go in `[dependencies]` not `[dev-dependencies]` if used by non-test lib code
  (booleans compiled under `cargo test` but broke `cargo build` until moved).
- Boolean output is flattened polylines (24 steps/cubic) — curves not preserved through ops.

## Previous session: 2026-06-13-x (spec 0030 — copy/paste layers DONE)

### Done
- **Spec 0030 ☑** — copy/paste layers: `EditorState.clipboard` (source NodeId); `paste_layer`
  deep-clones fresh each time (clone_subtree + InsertSubtree) above the selection; Ctrl+C/V +
  Edit menu. Independent copies, stale-source no-op. app 34 tests green, clippy clean, smoke.

### Next
1. **Boolean path ops** (VEC-5) — the last big vector item; NEW `i_overlay` dep, read its API
   first. Persistent deferred new-dep task.
2. **Phase 5** — place image (INT-3: needs `image` decode + rfd file dialog), smart objects
   (DOC-5), z-interleaved raster+vector compositing (vectors currently overlay).
3. Group-as-unit move in cross-layer align; OS-clipboard / cross-doc paste.

### Watch out (additions)
- Clipboard stores a source NodeId and re-clones per paste (fresh ids) — copies independent;
  deleted source → paste no-ops.

## Previous session: 2026-06-13-w (spec 0029 — cross-layer align/distribute DONE)

### Done
- **Spec 0029 ☑** — multi-object VEC-6: `panels::align_layers` / `distribute_layers` align or
  distribute the selected raster/vector layers to each other (per-layer translate as one
  `Batch` undo step). New `command::Batch` (apply-in-order/revert-in-reverse) +
  `TileMap::content_bounds` (pixel-exact). Layers-panel controls appear with a multi-selection.
  core 40 / app 33 tests green, clippy clean, smoke clean.
- VEC-6 now complete (within-layer 0026 + canvas-align 0022 + cross-layer 0029).

### Next
1. **Boolean path ops** (VEC-5) — the last big vector item; NEW `i_overlay` dep, read its API
   first. Persistent deferred new-dep task.
2. **Copy/paste layers** (reuse `InsertSubtree` + a clipboard on the app); group-as-unit move.
3. Phase 5 — place image (INT-3, needs `image` decode + file dialog), smart objects,
   z-interleaved raster+vector compositing.

### Watch out (additions)
- `command::Batch` = one undo step for N commands (reuse for any compound edit).
- Raster alignment uses `TileMap::content_bounds` (pixel-exact, O(pixels)); the older
  `bounds` is tile-granular (256 px) — don't use it where precision matters.
- Cross-layer align moves only raster/vector leaves (groups skipped).

## Previous session: 2026-06-13-v (spec 0028 — multi-select + group/ungroup DONE)

### Done
- **Spec 0028 ☑** — additive node multi-select (`EditorState.selected_extra` beside primary
  `editor.selection`; shift/ctrl-click in Layers panel; stale-pruned each frame). Core
  `GroupNodes` / `UngroupNode` commands + `Document::set_children_order`. App `group_selected`
  / `ungroup_selected`, Ctrl+G / Ctrl+Shift+G, Layer-menu entries. core 39 / app 32 tests
  green, clippy clean, smoke clean.

### Next
1. **Cross-layer align/distribute** — reuse `selected_node_set()` to align/distribute whole
   layers (raster offset / vector translate) to each other; the multi-object half of VEC-6.
2. **Boolean path ops** (VEC-5) — NEW `i_overlay` dep, read API first. The persistent
   deferred new-dep task.
3. **Copy/paste layers** (reuse `InsertSubtree` + a clipboard); Phase 5 — place image (INT-3),
   smart objects, z-interleaved compositing.

### Watch out (additions)
- Multi-select is additive: `selected_node_set()` = primary + valid extras (deduped). Group
  requires all members share a parent (`GroupNodes::new` returns None otherwise).
- Ungroup drops contents at the group's slot; pre-group order returns only via undo-group.

## Previous session: 2026-06-13-u (spec 0027 — duplicate layer DONE)

### Done
- **Spec 0027 ☑** — `Document::clone_subtree` (deep copy w/ fresh ids) + `InsertSubtree`
  command; `duplicate_selected_layer` (Ctrl+J / Layer menu) copies the selected layer/group
  above itself, undoable, selects the copy. core / app 31 tests green, clippy clean, smoke.
- `InsertSubtree` is a reusable building block for future paste / drag-duplicate / place.

### Next
1. **Spec 0028 — boolean path ops** (VEC-5): unite/subtract/intersect/exclude. NEW
   `i_overlay` workspace dep — read its API first. The persistent deferred new-dep task.
2. **Node multi-select** (selection → set) → cross-layer align/distribute, group/ungroup of
   multiple, copy/paste layers (reuse InsertSubtree). Sizable; plan carefully.
3. Phase 5 — place image (INT-3), smart objects, z-interleaved raster+vector compositing.

### Watch out (additions)
- `clone_subtree` allocates fresh ids from the doc counter; safe for nested groups.
  `InsertSubtree` apply=restore_subtree(clone), revert=remove_subtree(root).

## Previous session: 2026-06-13-t (spec 0026 — align/distribute shapes DONE)

### Done
- **Spec 0026 ☑** — `panels::align_shapes_in_layer` (L/C/R/T/M/B vs union bounds) +
  `distribute_shapes_in_layer` (even center spacing, H/V), operating on the selected vector
  layer's shape list (no multi-select needed), undoable via SetVectorShapes, with Properties
  buttons. VEC-6 within-layer subset. app 30 tests green, clippy clean, smoke clean.

### Next
1. **Spec 0027 — boolean path ops** (VEC-5): unite/subtract/intersect/exclude. NEW
   `i_overlay` workspace dep — read its API first. The persistent deferred new-dep task.
2. **Node multi-select** (selection: Option<NodeId> → set) → cross-layer align/distribute,
   group ops; sizable refactor touching many call sites — plan carefully.
3. Phase 5 — place image (INT-3), smart objects, z-interleaved raster+vector compositing.

### Watch out (additions)
- Vector align/distribute so far is WITHIN one layer's shapes; cross-layer needs node
  multi-select (not yet built). Distribute is by-center, not by-gap.

## Previous session: 2026-06-13-s (spec 0025 — anti-aliased rasterize DONE)

### Done
- **Spec 0025 ☑** — rewrote `raster_vector` with 4×4 supersample coverage + straight-alpha
  src-over (overlapping shapes blend); edges now anti-aliased. raster / app 29 tests green
  (incl. `edges_are_antialiased`), clippy clean.

### Next
1. **Spec 0026 — boolean path ops** (VEC-5): unite/subtract/intersect/exclude. NEW
   `i_overlay` workspace dep — read its API first (flatten cubics→polygons, op, back to line
   `Path`); `BooleanOp` command over ≥2 shapes; Pathfinder panel. The persistent deferred
   new-dep task — best done fresh.
2. Multi-select → multi-object align/distribute; per-subpath fill memory; gamma-correct AA.
3. Phase 5 — place image (INT-3), smart objects, z-interleaved raster+vector compositing.

### Watch out (additions)
- Rasterize AA blends in straight sRGB components (not linear) — matches the compositor;
  gamma-correct AA waits for the color phase.

## Previous session: 2026-06-13-r (spec 0024 — compound paths DONE)

### Done
- **Spec 0024 ☑** — `Path::append` / `Path::split_subpaths`; `panels::make_compound_path`
  (merge layer shapes → one even-odd compound) and `release_compound_path` (split subpaths
  back), both undoable via SetVectorShapes, with Properties buttons. VEC-8. vector / app 29
  tests green, clippy clean, smoke clean.
- ROADMAP Phase 4: compound paths done; remaining vector items are boolean ops + multi-object
  align/distribute (needs multi-select).

### Next
1. **Spec 0025 — boolean path ops** (VEC-5): unite/subtract/intersect/exclude. NEW
   `i_overlay` workspace dep — read its API first (flatten cubics→polygons, op, back to line
   `Path`); `BooleanOp` command over ≥2 shapes; Pathfinder panel. Still the deferred
   new-dep task — good fresh start.
2. Multi-select → multi-object align/distribute; rasterize AA; per-subpath fill memory.
3. Phase 5 — place image (INT-3), smart objects, z-interleaved raster+vector compositing.

### Watch out (additions)
- Compound = even-odd combine (gives holes via the tessellator), NOT boolean ops. Release
  flattens all subpaths to the source shape's fill (no per-subpath fill memory).

## Previous session: 2026-06-13-q (spec 0023 — rasterize vector layer DONE)

### Done
- **Spec 0023 ☑** — `rasterize_vector` (tessellate + scan-fill triangles into tiles, no AA);
  `ReplaceNodeKind` generic command; Layer → Rasterize Layer converts the selected vector
  layer to a raster layer in place (undoable). First raster↔vector interop (INT-2).
  raster / app 28 tests green, clippy clean, smoke clean.
- ROADMAP: INT-2 done early (vector engine was ready).

### Next
1. **Spec 0024 — boolean path ops** (VEC-5): unite/subtract/intersect/exclude. NEW
   `i_overlay` workspace dep — **read its API first** (flatten cubics→polygons, op, back to
   line `Path`); `BooleanOp` command over ≥2 shapes; Pathfinder panel. Deferred repeatedly
   because it needs the new dep — good fresh-session task.
2. Multi-select → multi-object align/distribute; compound paths; AA for rasterize.
3. Phase 5 proper — focus modes, place raster into vector doc, smart objects, z-interleaved
   raster+vector compositing (currently vectors overlay the raster composite).

### Watch out (additions)
- Rasterize is hard-edged (no AA) and document-sized from origin. `ReplaceNodeKind` is the
  reusable kind-swap command.
- NOTE: the working tree contains transform/crop/resample (`atelier-raster::resample`,
  Transform…/Crop menus) from earlier sessions not in this context — treat as existing.

## Previous session: 2026-06-13-p (spec 0022 — align-to-canvas + README/env docs DONE)

### Done
- **README rewritten** (user ask): full environment setup — rustup install per-OS, MSVC/
  Xcode/Linux build deps, GPU drivers, build/run/test commands, status, platform tiers.
  Reframed "no generative AI" as a scope/over-engineering exclusion (not ideology) across
  README + VISION; recorded as **D-13**. Committed 9221f69.
- **Spec 0022 ☑** — `Path::translate`; `panels::align_vector_to_canvas` (L/C/R/T/M/B) aligns
  the selected vector layer's shapes (as a group) to the document bounds via SetVectorShapes
  (undoable), with Properties buttons. No-dep subset of VEC-6. vector 17 / app 27 tests
  green, clippy clean, smoke clean.
- ROADMAP Phase 4 = ◐ (path editing + canvas-align done; booleans + multi-object align remain).

### Next — Phase 4 finish
1. **Spec 0023 — boolean path ops** (VEC-5): unite/subtract/intersect/exclude. Needs the NEW
   `i_overlay` workspace dep — **read its API/docs first** (don't add blind). Plan: convert
   `Path`→i_overlay polygons (flatten cubics to polylines), run op, convert back to a
   line `Path`; a `BooleanOp` command over ≥2 shapes/layers; Pathfinder panel buttons.
   Document the flattening/precision trade-off. Start this fresh — it's the reason 0022 did
   align instead.
2. Multi-select → multi-object align/distribute (the full VEC-6); compound paths.
3. Phase 5 — focus modes & raster↔vector interop (z-interleaving).

### Watch out (additions)
- Align uses `Path::bounds` (control-hull, not exact curve extrema) — fine for layout.
- Booleans deliberately deferred: new dep + unfamiliar API = fresh-session work.

## Previous session: 2026-06-13-o (spec 0021 — bezier handle UI DONE; path editing complete)

### Done
- **Spec 0021 ☑** — `Path::out_handle`/`in_handle` getters; Direct Select click selects an
  anchor and renders its bezier handles, dragging a handle reshapes the curve via
  `set_out_handle`/`set_in_handle` + merged `SetVectorShapes` (undoable). Added
  `selected_anchor` + `handle_drag` state and `nearest_handle` hit-test. **Interactive path
  editing (shapes, anchors add/move/remove, bezier handles) is now complete.** app 26 /
  core 36 / vector 16 tests green, clippy clean, smoke clean.
- ROADMAP Phase 4 = ◐ (all path-editing slices done; booleans/align/compound remain).

### Next — Phase 4 finish
1. **Spec 0022 — boolean path ops** (VEC-5): unite/subtract/intersect/exclude via the
   `i_overlay` crate (NEW workspace dep — add to root Cargo.toml + atelier-vector). Provide
   `atelier_vector` boolean fns over `Path`, a `BooleanOp` command combining ≥2 selected
   vector layers (or shapes), and a Pathfinder-style UI (panel buttons). i_overlay API is
   unfamiliar — read its docs first; convert `Path`→i_overlay polygons (flatten cubics),
   op, convert back to `Path` (lines). Document precision/flattening trade-off.
2. Align/distribute; compound paths (multi-subpath fill already supported by the model).
3. Phase 5 — focus modes & raster↔vector interop (z-interleaving; vectors currently overlay).

### Watch out (additions)
- Handles move independently (no symmetric mode yet); closing-edge handles unsupported until
  close is a real segment.
- `selected_anchor`/`handle_drag` indices can go stale if shapes change out from under them;
  renders/drags guard with `.get()`, but a future shape-structure edit should clear them.

## Previous session: 2026-06-13-n (spec 0020 — bezier handle model DONE)

### Done
- **Spec 0020 ☑** — `Path::set_out_handle` / `set_in_handle`: set an anchor's outgoing/incoming
  bezier control point, converting the adjacent Line→Cubic (endpoints preserved, boundary
  no-ops). Pure model layer for the handle-drag UI. 15 vector tests, workspace green, clippy
  clean. (Model-only slice — no app wiring, so no smoke change.)
- ROADMAP Phase 4 = ◐ (through slice c2e-model).

### Next — Phase 4 continues
1. **Spec 0021 (slice c2f)** — on-canvas handle UI: render in/out handles for the selected
   anchor(s) in Direct Select, hit-test + drag them (via `set_out_handle`/`set_in_handle` +
   merged `SetVectorShapes`), with symmetric-handle default. Then curves are fully editable.
2. Booleans (i_overlay dep), align/distribute, compound paths; closing-edge-as-real-segment
   (needed for closing-edge handles + clean boolean input).
3. Phase 5 — focus modes & raster↔vector interop.

### Watch out (additions)
- Handle primitives only touch STORED segments; a closed subpath's implicit closing edge has
  no segment, so its handles/inserts are unsupported until close becomes a real segment.

## Previous session: 2026-06-13-m (spec 0019 — segment-click insert DONE)

### Done
- **Spec 0019 ☑** — `Path::closest_segment` (nearest point on lines/cubics → split anchor
  index + distance; `dist_to_segment`/`dist_to_cubic` helpers). Direct Select double-click on
  a segment inserts an anchor there via `SetVectorShapes` (undoable), completing the
  add-anchor gesture. Primitive unit-tested; app gesture manual-verified. app 26 / core 36 /
  vector tests green, clippy clean, smoke clean.
- ROADMAP Phase 4 = ◐ (slices a..c2d done). Path geometry is now fully editable except
  bezier curve handles.

### Next — Phase 4 continues
1. **Spec 0020 (slice c2e)** — bezier control-handle drag: drag handles off an anchor to
   convert its adjacent line segments to cubics and shape curves; needs handle hit-testing +
   a `Path` method to set in/out control points (convert Line→Cubic). True de Casteljau
   segment split (so curve-insert preserves shape) can ride along.
2. Booleans (i_overlay dep: unite/subtract/intersect/exclude), align/distribute, compound paths.
3. Phase 5 — focus modes & raster↔vector interop (z-interleaving; vectors currently overlay).

### Watch out (additions)
- `closest_segment` cubic distance is a 16-sample approximation; inserting on a cubic puts
  the anchor at the click (not an exact split) — revisit with handle work (0020).

## Previous session: 2026-06-13-l (spec 0018 — add/remove anchors DONE)

### Done
- **Spec 0018 ☑** — `Path::remove_anchor` (reconnects, min-2 guard) and `Path::insert_anchor`
  (line anchor before an index); Direct Select Alt+click removes an anchor via
  `SetVectorShapes` (undoable). `nearest_anchor` hit-test factored out. Primitives
  unit-tested; app remove is manual-verified. app 26 / core 36 / vector tests green,
  clippy clean, smoke clean.
- ROADMAP Phase 4 = ◐ (slices a,b,c1,c1b,c2a,c2b,c2c done).

### Next — Phase 4 continues
1. **Spec 0019 (slice c2d)** — bezier control-handle drag: drag out handles to convert a
   line anchor's segments to cubic and reshape curves; segment-click to insert an anchor
   (the `insert_anchor` primitive exists, needs segment hit-testing). Then path is fully
   editable.
2. Booleans (i_overlay: unite/subtract/intersect/exclude) — needs the `i_overlay` workspace
   dep; align/distribute; compound paths.
3. Phase 5 — focus modes & raster↔vector interop (z-interleaving; vectors currently overlay).

### Watch out (additions)
- `Path::insert_anchor` exists + unit-tested but is NOT yet wired to a UI gesture (needs
  segment hit-testing, slice c2d). Only remove (Alt+click) is wired.
- Anchor indices follow `Path::anchors()` order; `remove_anchor(0)` promotes the first
  segment endpoint to the new subpath start.

## Previous session: 2026-06-13-k (spec 0017 — direct-select anchor editing DONE)

### Done
- **Spec 0017 ☑** — Direct Select tool (A): drag an on-path anchor of the selected vector
  layer to reshape it, live + undoable (merged → one entry per drag), anchor-dot overlay.
  Added `Path::anchors()` / `Path::move_anchor()` (cubic handles preserved) and the
  mergeable `SetVectorShapes` command. First editing of existing vector geometry. core 36 /
  app 25 / vector tests green, clippy clean, smoke clean.
- ROADMAP Phase 4 = ◐ (slices a,b,c1,c1b,c2a,c2b done).

### Also done (0017 follow-up)
- Vector fill-color editor in the Properties panel (`panels::apply_vector_fill`, merged
  `SetVectorShapes`, undoable) — recolor a selected vector layer's shapes. app 26 tests.

### Next — Phase 4 continues
1. **Spec 0018 (slice c2c)** — bezier control-handle drag (convert line↔curve), add/remove
   anchor on an existing path, marquee anchor multi-select. Builds on `SetVectorShapes` +
   the DirectSelect hit-testing; will need handle hit-testing + a `Seg` line↔cubic swap.
2. Booleans (i_overlay: unite/subtract/intersect/exclude), align/distribute, compound paths.
3. Phase 5 — focus modes & raster↔vector interop (z-interleaving; vectors currently overlay).

### Watch out (additions)
- DirectSelect hit-testing is screen-space (~10 px); app-level drag is manual-verified
  (headless kittest can't cheaply reconstruct the canvas screen mapping) — editing math +
  command are unit-covered.
- `SetVectorShapes` snapshots the whole shapes vec per edit (fine at current sizes).

## Previous session: 2026-06-13-j (spec 0016 — pen tool DONE)

### Done
- **Spec 0016 ☑** — Pen tool (P): click to drop straight-line anchors, close by clicking
  near the first anchor (≥3 pts) or Enter, finish open with Enter (≥2), Escape cancels;
  inserts a filled `NodeKind::Vector` layer via `AddNode` (undoable), live preview
  (polyline + anchor dots + rubber band). Added `Path::polyline(points, closed)`. First
  multi-anchor authoring. app 25 tests green, clippy clean, smoke clean.
- ROADMAP Phase 4 = ◐ (slices a,b,c1,c1b,c2a done).

### Next — Phase 4 continues
1. **Spec 0017 (slice c2b)** — direct-select / anchor editing: hit-test anchors of the
   selected vector layer's path, drag to move them (undoable edit-path command); bezier
   handle drag (convert line anchor ↔ curve); add/remove anchor. This is the first time we
   *edit* existing path geometry — needs a `SetPath`/`EditShape` command in atelier-core and
   anchor hit-testing on the canvas.
2. Booleans (i_overlay: unite/subtract/intersect/exclude), align/distribute, compound paths.
3. Phase 5 — focus modes & raster↔vector interop (z-interleaving; vectors currently overlay).

### Watch out (additions)
- Pen state (`pen_points`) is transient UI committed once on finish (like a brush stroke) —
  no command until `finish_pen`. Tool-switch/new-doc should clear it (currently cleared on
  finish/Escape only; a stray in-progress path persists if you switch tools mid-draw —
  minor, fix in 0017).
- All vector authoring so far is whole-shape INSERT; editing existing geometry starts in 0017.

## Previous session: 2026-06-13-i (specs 0014 + 0015 — Phase 4 shape tools DONE)

### Done
- **Spec 0014 ☑** — Rectangle (U) + Ellipse shape tools (rubber-band drag → filled vector
  layer, undoable via AddNode, live-rendered). First vector authoring.
- **Spec 0015 ☑** — Polygon + Star tools; added `Path::polygon`/`Path::star`; generalized the
  shape pipeline to `ShapeKind { Rect, Ellipse, Polygon, Star }` + `ActiveTool::shape_kind()`.
  Tools panel has all four + shared vector-fill picker. app 23 / core 35 / vector tests green,
  clippy clean, smoke clean. Both committed (0014 = 164b63c; 0015 pending this commit).
- ROADMAP Phase 4 = ◐ (slices a,b,c1,c1b done).

### Next — Phase 4 continues
1. **Spec 0016** — pen tool (click-add anchors, drag bezier handles) + direct-select
   (move anchors/handles) + line/open-path. Needs an edit-path command + on-canvas anchor
   hit-testing. Editing existing shape geometry still doesn't exist — only whole-shape insert.
2. Booleans (i_overlay), align/distribute, compound paths.
3. Phase 5 — focus modes & raster↔vector interop (z-interleaving; vectors currently overlay).

### Watch out (additions)
- Shape pipeline: drag → `pending_shape: Option<(ShapeKind,min,max)>` → drained in `ui()`
  after `DockArea::show` → `add_shape_layer`. Add a new primitive = new `ShapeKind` variant +
  `shape_kind()` arm + `add_shape_layer` match arm + panel entry.
- Polygon sides (6) and star points (5) are fixed; configurable UI deferred.

## Previous session: 2026-06-13-h (spec 0014 — Phase 4 slice c1 DONE)

### Done
- **Spec 0014 ☑** — shape tools: Rectangle (U) + Ellipse tools rubber-band a shape on the
  canvas and insert a filled `NodeKind::Vector` layer (fill = `BrushSettings.vector_fill`,
  picker in Tools panel), undoable via plain `AddNode`, rendered live by the 0013 path.
  First real vector authoring (layers were test/fixture-only before). Reused marquee drag
  plumbing; insertion queued on `pending_shape`, drained in the app loop. app 23 / core 35
  tests green, clippy clean, smoke clean.
- ROADMAP Phase 4 = ◐ (slices a+b+c1 done).

### Next — Phase 4 continues
1. **Spec 0015 (slice c2)** — pen tool (click to add anchors, drag for bezier handles),
   direct-select (move anchors/handles), and the remaining shape primitives
   (polygon/star/line). Needs an edit-path command + anchor hit-testing on the canvas.
   Editing existing shape geometry doesn't exist yet — only whole-shape insert.
2. Booleans (i_overlay: unite/subtract/intersect/exclude), align/distribute, compound paths.
3. Phase 5 — focus modes & raster↔vector interop (z-interleaving; today vectors overlay).

### Watch out (additions)
- Shape insertion uses the `pending_shape` queue drained in `ui()` after `DockArea::show`
  (canvas can't call `&mut self` app helpers) — same pattern as `wand_click`.
- Vector layers still render as an egui-mesh OVERLAY above the raster composite (R-13 /
  Phase 5), not z-interleaved.

## Previous session: 2026-06-13-g (spec 0013 — Phase 4 slice b DONE)

### Done
- **Spec 0013 ☑** — vector layers now render on the canvas: each visible vector layer's
  shapes are tessellated (cached by history revision in `EditorState.vector_cache`), mapped
  to screen by the viewport, and painted as `egui::epaint::Mesh` above the raster composite,
  below the selection ants. Layer opacity scales vertex alpha. Rendered through egui's wgpu
  mesh path (keeps "only atelier-gpu imports wgpu"); a bespoke GPU pipeline is a later perf
  option. Full suite green (app 22 tests), clippy clean.
- ROADMAP Phase 4 = ◐ (slices a+b done).

### Next — Phase 4 continues
1. **Spec 0014 (slice c)** — authoring tools: pen (add/move/convert anchors), shape tools
   (rect/ellipse/polygon/star/line), direct-select; commands to create `NodeKind::Vector`
   layers and edit shapes. No vector-authoring UI exists yet — layers are only constructed
   in tests/fixtures so far.
2. Booleans (i_overlay: unite/subtract/intersect/exclude), align/distribute, compound paths.
3. Phase 5 — focus modes & raster↔vector interop (incl. true z-interleaving of raster +
   vector in one compositor; today vectors are an overlay above the raster composite).

### Watch out (additions)
- Vector render is an egui-mesh OVERLAY above the raster composite — NOT z-interleaved with
  raster layers yet (Phase 5). `vector_cache` rebuilds on revision change only.
- Tessellation is in doc space; extreme zoom can facet (re-tessellation at screen scale
  deferred).

## Previous session: 2026-06-13-f (spec 0012 — Phase 4 STARTED, slice a DONE)

### Done
- **Spec 0012 ☑** — vector engine slice a: `atelier-vector` crate (pure: serde+lyon+kurbo,
  D-14) with `Path`/`PathBuilder` (cubic Béziers, subpaths, fill rule, rect/ellipse),
  `Shape`/`Stroke`/`VectorContent`, and `tessellate()` → flat-color triangle `Mesh` (fill +
  stroke via lyon). `NodeKind::Vector(VectorContent)` replaces the PlaceholderArt stub;
  `.atl` round-trips vector shapes + migrates legacy `Vector{bounds,color}`. Workspace +
  clippy clean, smoke clean.
- ROADMAP Phase 4 = ◐ (slice a done).

### Next — Phase 4 continues
1. **Spec 0013 (slice b)** — GPU mesh render: a wgpu pipeline in `atelier-gpu` that draws
   `atelier_vector::Mesh` (flat-color triangles) into the canvas viewport; canvas tessellates
   each vector layer's shapes (cache by revision) and draws them over the raster composite.
   Resolution-independent (re-tessellate or transform in vertex shader) for crisp zoom (VEC-7).
2. **Spec 0014 (slice c)** — pen tool (add/move/convert anchors), shape tools (rect/ellipse/
   polygon/star/line), direct-select; create `NodeKind::Vector` layers; commands for shape
   add/edit.
3. Then booleans (i_overlay: unite/subtract/intersect/exclude), align/distribute, compound
   paths. Then Phase 5 (focus modes + raster↔vector interop).

### Watch out (additions)
- `atelier-core` now depends on `atelier-vector` (D-14) — keep atelier-vector pure
  (no GPU/UI). The GPU renderer (0013) consumes `Mesh`, it does not depend on lyon.
- Canvas does NOT yet draw vector shapes (invisible until 0013); they exist in model + .atl.
- even-odd vs non-zero test divergence requires same-winding subpaths (noted in spec 0012).

## Previous session: 2026-06-13-e (spec 0011 — **PHASE 3 COMPLETE ☑**)

### Done
- **Spec 0011 ☑** — magic wand + selection ops. `Mask::select_all`/`inverted` (core);
  `atelier-raster::selection::magic_wand` (BFS flood fill by tolerance), `grow`/`shrink`
  (chebyshev morphology), `feather` (2-pass box blur). App: Magic Wand tool (W) with
  tolerance slider + Shift/Alt combine; Select menu (All Ctrl+A / Deselect / Invert
  Ctrl+Shift+I / Grow / Shrink / Feather). 94 tests, clippy clean, smoke clean.
- **Phase 3 gate met → Phase 3 ☑.** Selection + adjustment toolset complete.
- Bug caught by existing test: new global Ctrl+A swallowed rename field's select-all-text;
  gated selection/adjust shortcuts behind `!ctx.wants_keyboard_input()`.

### Next — PHASE 4 (vector engine), the next major phase
1. **Spec 0012** — vector path model + GPU tessellated render: `atelier-vector` path type
   (cubic Béziers, subpaths, fill rule), fill/stroke, lyon tessellation → triangles, a GPU
   pipeline in `atelier-gpu` to draw them, `NodeKind::Vector` upgraded from PlaceholderArt
   to a real shape list. Slice it: (a) path model + tessellation (pure, tested), (b) GPU
   render of filled/stroked paths on the canvas, (c) pen/shape tools + editing.
2. Then booleans (i_overlay), align/distribute, compound paths (Phase 4 remainder).
3. Phase 5 (focus modes & raster↔vector interop) after.

### Watch out (additions)
- Selection/adjust keyboard shortcuts are gated behind `!wants_keyboard_input()`; keep new
  letter/Ctrl shortcuts on that side unless they should override text fields.
- `NodeKind::Vector` is still `PlaceholderArt` (rect) — Phase 4 replaces it; the compositor
  and canvas currently draw vector layers as placeholder rects only.
- Workspace deps to add in Phase 4: `lyon` (tessellation), `kurbo` (already considered),
  `i_overlay` (booleans, later slice).

## Previous session: 2026-06-13-d (spec 0010 — Phase 3 slice d DONE)

### Done
- **Spec 0010 ☑** — transform/crop/resample. `atelier-raster::resample` (bilinear sample,
  inline affine bake `transform_layer`, `resample_layer`); commands `ReplaceLayerTiles`,
  `ResizeImage`, `CropCanvas` (all undoable, snapshot-based, D-13 destructive bake);
  `Mask::pixel_bounds()` (exact, fixed a tile-granular crop bug). App: Layer → Transform…
  (numeric scale/rotate dialog), Image → Crop to Selection, Image → Image Size… (resample).
  Full suite green, clippy clean, smoke clean.
- GPU golden parity occasionally flakes locally (NVIDIA device churn); serialized via
  GPU_LOCK; CI unaffected (skips on software adapter). Not a compositor defect.
- ROADMAP Phase 3 still ◐ — only magic wand + feather/grow/invert-selection remain before
  the Phase 3 gate.
- Post-commit CI fix `fc4d971`: `transform_layer` was pivoting about tile-granular
  `bounds()` (wrong center); added `TileMap::pixel_bounds()`, pivot about it. CI-caught.

### Next
1. **Spec 0011 — Phase 3 final slice**: magic wand (flood-fill select by color tolerance),
   selection ops feather (gaussian on mask) / grow / shrink / invert / select-all; then
   close the Phase 3 gate (mask op tests + per-tool checklist) and flip Phase 3 ☑.
2. Phase 4 — vector engine (spec 0012+): path model, pen/shapes, booleans, tessellated GPU
   render. Big phase; slice it (path model + render first).

### Watch out (additions)
- `Mask::bounds()` is tile-granular; use `Mask::pixel_bounds()` when you need exact extent
  (crop, future trim). Bit me in 0010.
- Transforms are destructive bakes (D-13) — repeated transforms degrade quality; that's
  expected until Smart Objects (Phase 10).
- Local-only GPU golden flake exists; if you see it, re-run isolated
  (`cargo test -p atelier-gpu --test golden_parity -- --test-threads=1`).

## Previous session: 2026-06-13-c (spec 0009 — Phase 3 slice c DONE)

### Done
- **Spec 0009 ☑** — non-destructive adjustment layers. Moved `Adjustment` enum + pixel
  math to `atelier-core::adjust`; `NodeKind::Adjustment(Adjustment)`; `CompositeOp::Adjust`;
  CPU compositor re-tones the backdrop below (visibility + opacity-as-amount); `.atl`
  round-trips; app "Layer → New Adjustment Layer →" inserts above selection; Properties
  panel edits params via merge-coalesced `SetAdjustment`. 87 tests, clippy clean, smoke clean.
- GPU compositor skips Adjust ops (no-op) — parity debt **R-13** (canvas uses CPU path, so
  output is correct; port to WGSL before any GPU→canvas wiring).
- ROADMAP Phase 3 still ◐ (slices a+b+c done).

### Next
1. **Spec 0010 — Phase 3 slice d**: free transform (scale/rotate/skew of a raster layer via
   resampled tiles), crop tool, image resample. Transform needs a resampler
   (nearest+bilinear) in atelier-raster; commands capture before/after tiles (PaintTiles
   pattern) or an affine on RasterContent — decide at spec time (record as D-13).
2. Magic wand + feather/grow/invert-selection (selection slice).
3. Then Phase 4 (vector engine).

### Watch out (additions)
- `Adjustment` now lives in `atelier-core`; `atelier-raster` re-exports it. New blend/adjust
  math added in core must stay pure (no GPU/UI deps).
- Adjustment layers are CPU-only in the compositor (R-13). Don't add them to GPU golden
  fixtures until WGSL adjustment exists.

## Previous session: 2026-06-13-b (spec 0008 — Phase 3 slice b DONE)

### Done
- **Spec 0008 ☑** — `atelier-raster::adjust` (Invert, Brightness/Contrast, Levels,
  Hue/Saturation as pure per-pixel maps + `apply_tile` with selection-coverage clip +
  `target_tiles`); brush gained `stamp_segment_clipped` so strokes honor the active
  selection; app "Adjust" menu (Invert=Ctrl+I immediate; B/C, Levels, Hue/Sat dialogs)
  applying to the selected raster layer within the selection (whole layer if none).
  Adjustments reuse the generic `PaintTiles` snapshot command (one undo entry each).
  80 tests green, clippy clean, smoke clean.
- ROADMAP Phase 3 still ◐ (slices a+b done).

### Next
1. **Spec 0009 — Phase 3 slice c**: adjustment *layers* — a non-destructive node kind the
   compositor applies to the backdrop beneath it (add `NodeKind::Adjustment(AdjustSpec)`;
   compositor reads it; UI to add + edit). Reuses `atelier_raster::adjust` math.
2. Slice d: free transform + crop + resample (from Phase 2, D-12).
3. Magic wand + feather/grow/invert-selection; then Phase 4 (vector engine).

### Watch out (additions)
- Adjustments/brush operate on the selected layer node; the doc selection *mask* clips
  which pixels change. `apply_adjustment` no-ops if no layer selected or layer not a
  visible/unlocked raster.
- `Mask::bounds()` is tile-granular (256-aligned) — fine for tile iteration, but don't
  use it as a pixel-exact content box in tests (bit me once).

## Previous session: 2026-06-13 (spec 0007 — Phase 3 slice a DONE)

### Done
- **Spec 0007 ☑** — selection model: `atelier-core::mask::Mask` (sparse 256² u8 tiles,
  combine Add/Subtract/Intersect/Replace), `Document.selection: Option<Arc<Mask>>`
  (serde-skipped) + undoable `SetSelection` (Arc snapshots); `atelier-raster::selection`
  (AA rect, supersampled ellipse, even-odd lasso, marching-squares `boundary_segments`);
  app tools Select Rect (M) / Select Ellipse / Lasso (L) with Shift=add / Alt=subtract /
  Shift+Alt=intersect, live drag previews, marching-ants (cached per revision), Ctrl+D
  deselect. 73 tests green, clippy clean, smoke clean.
- ROADMAP Phase 3 = ◐ (slice a done).

### Next
1. **Spec 0008 — Phase 3 slice b**: selection-clipped painting (brush/eraser honor the
   active mask) + first destructive adjustments (levels/curves/brightness-contrast/
   hue-sat/invert), each an undoable command operating within the selection.
2. Slice c: adjustment *layers* (non-destructive node kind in the compositor).
3. Slice d: free transform + crop + resample (moved from Phase 2, D-12).
4. Then magic wand + feather/grow/invert UI; then Phase 4 (vector).

### Watch out (additions)
- Drag-start position must come from `pointer.press_origin()`, NOT `interact_pointer_pos`
  (kittest coalesces press+move; the latter returns the already-moved point). Applies to
  every future click-drag tool. Recorded in spec 0007 notes.
- Selection is session-only (not in `.atl`) and does not yet clip paint — both are slice b+.

## Previous session: 2026-06-12-e (spec 0006 — **PHASE 2 COMPLETE ☑**)

### Done
- **Spec 0006 ☑** — `composite_region_rgba8` (region == slice-of-full, proven incl.
  Dissolve absolute-coord hash + offsets); live brush strokes patch only their dirty rect
  via `ImageDelta::partial` (no revision churn — commit is the single bump); pan/zoom
  recomposite-free (test-proven).
- **Phase 2 gate measured and passed** (release, dev box): 256² region over 50 layers =
  18.6 ms (< 25 ms target); pan/zoom = texture redraw only. ROADMAP row 2 ☑.
- **D-12**: Phase 2 closed via perf slice; free transform + crop tool + image resample
  moved into Phase 3 contents; tablet pressure → future brush-dynamics spec.
- 64 tests green, clippy clean, smoke clean.
- Known debt logged in spec 0006: structural edits full-recomposite (6 s on 4096²×50
  pathological doc) — GPU-canvas wiring + command-level dirty rects when it hurts.

### Next
1. **Phase 3 — selections & adjustments** (+ transform/crop/resample per D-12). Write
   spec 0007 first. Suggested slicing: (a) selection model (8-bit mask + combine ops +
   rect/ellipse/lasso tools + marching ants), (b) selection-clipped painting + adjustments
   (levels/curves/etc., destructive first), (c) adjustment layers, (d) free transform +
   crop + resample.
2. Phase 4 (vector engine) after.

## Previous session: 2026-06-12-c/d (Phase 2 slices b+c — specs 0004 AND 0005 DONE)

### Done
- **Spec 0004 ☑** — GPU compute compositor (`atelier-gpu::compositor` + composite.wgsl):
  full blend-mode set in WGSL, isolation stack, shared op list
  (`atelier-raster::ops`). Golden parity on RTX 3060: **bit-exact**, 0 bytes differ across
  8 fixture docs (gate was ≤1 LSB); Dissolve hash matches exactly. Canvas now renders the
  real composited document (CPU composite → egui texture, cached by `History::revision`);
  placeholder painting removed.
- **Spec 0005 ☑** — brush/eraser (`atelier-raster::brush`: smoothstep hardness, spaced
  stamps, src-over/erase), move tool (`RasterContent.offset` + mergeable `SetOffset`),
  Canvas Size dialog (`CanvasResize`), live-stroke → one `PaintTiles` undo entry via new
  `History::push_committed`; `History::touch()` for live-preview recomposite; Tools panel
  real (V/B/E shortcuts, size/hardness/color); both compositors honor offsets
  (GPU via `TileMap::extract_shifted`, golden tests extended, still bit-exact).
- Gates: **61 tests** green, clippy clean, smoke run clean. Verification logs in both specs.

### Next
1. Phase 2 remainder (one more spec): free transform, crop tool, resample, pressure,
   GPU-canvas wiring + dirty-rect recomposite → then measure the 60 fps gate and flip
   Phase 2 ☑. OR jump to Phase 3 (selections) first if transform work is better after
   masks exist — decide at spec-writing time, record as D-12.
2. Phase 3 (selections & adjustments) per ROADMAP.

### Watch out (additions)
- WGSL mode indices are hand-numbered to match `BlendMode::ALL` order — change together
  (spec 0004 notes).
- Live brush stroke is the second sanctioned direct-mutation exception (commit on release);
  any new tool must follow the same capture→mutate→push_committed pattern.

## Previous session: 2026-06-12-b (Phase 2 slice a — spec 0003 DONE)

### Done this session
- **Spec 0003 ☑** (raster engine slice a): `atelier-core::tile` (sparse 256² RGBA8 TileMap,
  straight alpha), `NodeKind::Raster(RasterContent { art, tiles })` with placeholder-filled
  tiles, `atelier-raster::blend` (all 28 blend modes, W3C formulas, deterministic Dissolve),
  `atelier-raster::compositor` (CPU reference — THE source of truth for spec 0004 GPU
  parity), `.atl` schema v1 (lz4 tile parts + v0 migration). 48 tests, clippy clean,
  smoke run clean. Verification log in spec 0003.
- ROADMAP Phase 2 stays ◐ (slice a of 3 done).

### Next (in order)
1. **Spec 0004 — GPU compositor parity** (write spec first): wgpu compute/render path
   compositing visible tiles, golden tests CPU==GPU within 1 LSB (8-bit) on software
   adapter (CI) + `#[ignore]`-gated hardware tests; canvas renders real tiles (replace
   placeholder rect painting; drop `RasterContent.art` afterwards). Dissolve hash must
   match `atelier-raster::blend::dissolve_keeps` exactly.
2. **Spec 0005 — brush/eraser + move/transform + crop/resize** with pixel-diff undo
   commands (Command pattern extends to tile edits) + kittest coverage.
3. Then Phase 3 (selections & adjustments) per ROADMAP.

### Watch out
- `RasterContent.tiles` is `#[serde(skip)]` at field level — pixels only exist in .atl
  binary parts; any new serialization path must reattach tiles (see io::atl loader).
- PS golden fixtures still missing (R-04) — blend anchored to W3C hand-checks.

## Previous session: 2026-06-12 (verification completion → Phases 0–1 DONE)

### Done this session
- **Specs 0001 + 0002 fully verified and closed (☑). ROADMAP Phases 0 and 1 are ☑.**
- Live app verified via OS automation screenshots: window, docked panels, New Document
  dialog → 1920×1080 Raster doc, status bar showing "NVIDIA GeForce RTX 3060 Laptop
  GPU · Vulkan".
- OS-level *click* automation turned out broken on this box (hover + keyboard reached the
  app; synthetic mouse buttons never did — NVIDIA overlay suspected). Pivoted to
  **egui_kittest headless UI tests** (D-10): 7 tests in `crates/atelier-app/src/main.rs::ui_tests`
  covering the full spec-0002 walkthrough (add/group/nest/reorder via buttons, row select,
  double-click rename + typing, blend combo, delete, Ctrl+Z, History click-jump,
  save→restart→open deep-equal, unsaved-changes guard, canvas zoom/pan incl. ctrl+wheel).
- App refactored for testability (D-11): `AtelierApp::ui(ctx)` frame-independent,
  `with_adapter_info()` headless constructor, dialog-free `open_from`/`save_to`.
- Canvas keyboard nav added: Ctrl+= / Ctrl+− / Ctrl+0, arrow-key pan (PS parity + testability).
- Glyph fix: move buttons now ASCII ("Up", "Down", "Into Group", "Out", "[G]" group prefix) —
  egui default font lacks ↑↓⇒⇐📁.
- Gates at session end: **29 tests green** (15 core + 3 io + 4 gpu + 7 UI),
  `cargo clippy --workspace --all-targets -- -D warnings` clean, 6s smoke run clean.
- Cargo.toml notes: egui now pins `features=["accesskit"]` (+ eframe "accesskit") — required
  by kittest; `accesskit = "0.17"` is an atelier-app dev-dep (match Cargo.lock when bumping egui).

### In flight
- Nothing mid-edit. Trunk green. Baseline commit 966c535 pushed to public repo
  **https://github.com/Toddoward/atelier** (branch `main`); CI matrix runs on push
  (Windows gate, Linux/macOS allowed-to-fail). Commit per spec going forward (CLAUDE.md).
- CI fully green on all three platforms as of a2dda29 (Windows/macOS/Ubuntu — Ubuntu needed
  eframe "x11"/"wayland" features for winit's Linux backends). R-12 cross-platform drift
  now caught continuously.

### Next (in order)
1. **Phase 2 — raster engine** (REQUIREMENTS RAS-1,2,4,5; DOC-3,8). Write spec 0003 from the
   template first (CLAUDE.md: spec before code). Slice order (per ROADMAP working agreement):
   a. `atelier-raster`: 256² sparse tile store + CPU reference compositor with the full
      blend-mode set (pure, unit-tested — this is the source of truth, D-9/R-04);
   b. `atelier-gpu`: GPU compositor matching CPU within 1 LSB (golden tests, software adapter
      in CI, `#[ignore]`-gated hardware tests);
   c. brush/eraser tools + move/transform + crop/resize, each with kittest coverage (D-10).
2. Phase 3 onward per ROADMAP (selections & adjustments next).

### Environment facts (save re-discovery)
- cargo not on PATH in fresh shells: `$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"`.
- Dev GPU: NVIDIA RTX 3060 Laptop (CUDA-capable — relevant for Phase 12 ONNX EPs).
- MSVC build tools present (VS2019 BT + VS2022 Community). Win11 x64.
- Launch app: `cargo run -p atelier-app`; logs need `$env:RUST_LOG='info'`.
- Computer-use MCP: app reachable as `atelier-app.exe` (request_access), but synthetic mouse
  *clicks* don't reach the app on this machine — don't retry that path for UI verification;
  use kittest (D-10).
