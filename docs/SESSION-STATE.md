# Session state — resume point

> **Always current.** Update before ending any session (CLAUDE.md hard rule).
> Cold start: read this, then ROADMAP.md, then the active spec.

## Last session: 2026-06-12 (verification completion → Phases 0–1 DONE)

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
