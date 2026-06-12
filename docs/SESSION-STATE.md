# Session state — resume point

> **Always current.** Update before ending any session (CLAUDE.md hard rule).
> Cold start: read this, then ROADMAP.md, then the active spec.

## Last session: 2026-06-12-c/d (Phase 2 slices b+c — specs 0004 AND 0005 DONE)

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
