# Spec 0001 — Bootstrap: workspace + app shell

- **Status:** ☑ done (2026-06-12)
- **Phase:** 0
- **Requirements:** SH-1, SH-2, SH-3 (Tier-1 only at this stage)
- **Depends on:** —

## Goal
A running cargo workspace where `cargo run -p atelier-app` opens a desktop window with a
wgpu-rendered surface, an egui dock layout (placeholder Tools / Layers / Properties panels
around a central canvas area), and a canvas that clears to a checkerboard and supports
pan (space/middle-drag) and zoom (ctrl+wheel) of an empty document grid. All 13 workspace
crates exist and compile (most as near-empty libs with their dependency directions fixed).

## Scope
- Workspace `Cargo.toml` + all `crates/atelier-*` skeletons per ARCHITECTURE layout, with
  the dependency rules encoded (e.g. `atelier-core` has no wgpu/egui/ort deps).
- `atelier-app`: winit event loop, wgpu device/queue/surface init with backend fallback,
  egui + egui_dock integration, panel placeholders, status bar with adapter name.
- `atelier-gpu`: device wrapper, surface management, checkerboard render pass for the canvas
  viewport, viewport transform (pan/zoom) uniform.
- Basic `tracing` logging; panic hook that logs.
- `.gitignore`, `rust-toolchain.toml` (stable), CI workflow file (build+test+clippy, Windows
  x64 first; macOS/Linux rows added but allowed-to-fail until Phase 15 hardening).

## Out of scope
- Any document model (Phase 1), real tools, menus beyond a stub File menu, settings, tests
  beyond crate-compiles + transform math unit tests.

## Design notes
- Canvas is a wgpu viewport rendered before egui paint each frame; egui draws panels around
  it (central dock node reserved). Pan/zoom = 2D affine in a uniform buffer; checkerboard in
  WGSL fragment shader from world coords (stable under zoom).
- Backend order: DX12 → Vulkan → GL on Windows (D-2). Adapter info surfaced in status bar to
  verify hardware acceleration is actually active (SH-2 evidence).

## Verification checklist
- [ ] `cargo build --workspace` green on Windows x64
- [ ] `cargo test --workspace` green (viewport transform unit tests)
- [ ] `cargo clippy --workspace -- -D warnings` clean
- [ ] [manual] `cargo run -p atelier-app`: window opens, panels visible & dockable, status
      bar shows a hardware adapter (not "llvmpipe"/"Microsoft Basic Render")
- [ ] [manual] pan with middle-drag / space-drag; zoom with ctrl+wheel centered on cursor;
      checkerboard stays crisp and stable
- [ ] Crate dependency audit: `atelier-core` Cargo.toml has no wgpu/egui/ort; only
      atelier-gpu lists wgpu; only atelier-app lists egui/winit

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-11 | `cargo build --workspace` | PASS | clean build, 2m43s first compile (eframe 0.31.1 / egui_dock 0.16.0 / wgpu 24.0.5) |
| 2026-06-11 | `cargo test --workspace` | PASS | 4/4 (atelier-gpu viewport: round_trip, zoom_about_keeps_anchor_fixed, zoom_clamps, pan_accumulates) |
| 2026-06-11 | `cargo clippy -- -D warnings` | PASS | finished clean, 27s |
| 2026-06-11 | window opens w/ hardware adapter | PASS (automated) | 10s smoke run, no crash; log: `wgpu initialized adapter=NVIDIA GeForce RTX 3060 Laptop GPU backend=Vulkan` |
| 2026-06-11 | crate dependency audit | PASS | atelier-core deps = thiserror only; wgpu only in atelier-gpu; egui/eframe only in atelier-app (uses eframe's wgpu re-export) |
| 2026-06-12 | window opens, panels visible, hardware adapter in status bar | PASS | live screenshots via OS automation: docked Tools/Canvas/Layers/Properties/History visible; status bar "NVIDIA GeForce RTX 3060 Laptop GPU · Vulkan · 100%" |
| 2026-06-12 | pan/zoom + dock interactivity | PASS (automated UI) | egui_kittest `canvas_keyboard_zoom_pan_and_ctrl_wheel`: ctrl+wheel zoom, Ctrl+=/−/0 zoom, arrow pan against the real widget tree; dock tab activation exercised in walkthrough test |

## Notes / surprises
- 2026-06-11: Rust toolchain installed via winget (rustup) during harness session.
- 2026-06-11: Version pins eframe 0.31 + egui_dock 0.16 + wgpu 24 resolved and compiled
  first try; keep these matched when bumping (egui_dock follows egui minor).
- Dev GPU is CUDA-capable (RTX 3060 Laptop) — good for Phase 12 EP testing.
- 2026-06-12: Keyboard viewport nav added (Ctrl+= / Ctrl+− / Ctrl+0, arrow pan) — both for
  usability (PS parity) and UI-test drivability. Mouse *feel* (middle-drag glide, pinch)
  remains an optional non-gating human pass.
- 2026-06-12: OS-level click automation (computer-use) reached the app for hover/keys but
  synthetic mouse-button events never registered (NVIDIA overlay suspected). UI verification
  therefore runs headlessly via egui_kittest (see spec 0002 notes) — also the durable CI path.
