# Session state — resume point

> **Always current.** Update before ending any session (CLAUDE.md hard rule).
> Cold start: read this, then ROADMAP.md, then the active spec.

## Last session: 2026-06-13-l (spec 0018 — add/remove anchors DONE)

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
