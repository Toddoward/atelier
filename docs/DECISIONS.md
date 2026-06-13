# Decision log (ADR-lite)

One line of context, the decision, and why. Newest last. Reference as D-N.

- **D-1 (2026-06-11)** Language = Rust stable. Safety + cross-compile + C FFI (lcms2/ort) + wgpu maturity. Alternatives: C++/Qt/Skia (more legacy risk, slower iteration), Electron (fails perf pillar).
- **D-2 (2026-06-11)** GPU = wgpu/WGSL everywhere; CUDA/MPS only via ONNX Runtime EPs for AI inference. Avoids per-platform kernel code; satisfies prompt's CUDA/MPS ask where it actually matters.
- **D-3 (2026-06-11)** UI = winit + egui + egui_dock for now; canvas isolated from UI framework; revisit after Phase 5 (see R-03).
- **D-4 (2026-06-11)** Drop Windows 32-bit (x86). Prompt's fallback clause invoked; GPU/AI stacks are 64-bit only (R-08).
- **D-5 (2026-06-11)** Native format `.atl` = ZIP(manifest.json + lz4 tiles + path JSON), open-spec'd. Enables crash recovery, partial load, third-party readers (VISION pillar 4).
- **D-6 (2026-06-11)** Native CMYK *editing* deferred post-v1; v1 = RGB/Gray native + CMYK export/soft-proof (R-05).
- **D-7 (2026-06-11)** AI models: MobileSAM (select), U²-Net (bg removal), LaMa (inpaint), Real-ESRGAN (upscale, P2) — permissive licenses, known ONNX exports. Download-on-demand, never bundled in repo.
- **D-8 (2026-06-11)** Working codename "Atelier", crate prefix `atelier-`, native extension `.atl`. Repo name stays photo-illustration-shop. Rename is cheap until Phase 7 (format freeze).
- **D-9 (2026-06-11)** Blend compositing in document color space with mode-specific linearization, premultiplied alpha internally; correctness anchored by CPU reference + Photoshop golden images (R-04).
- **D-10 (2026-06-12)** UI verification = egui_kittest headless tests driving the real widget tree (CI-safe, no GPU/display). Every spec's UI checklist items must be automated this way where mechanically possible; eyes-on "feel" checks are optional and non-gating. Born from OS-level click automation failing on this dev box (synthetic button events never reached the app; NVIDIA overlay suspected).
- **D-11 (2026-06-12)** App code stays kittest-friendly by construction: per-frame UI in `AtelierApp::ui(&mut self, ctx)` (no eframe::Frame deps), native file dialogs isolated in thin wrappers over dialog-free `open_from`/`save_to`-style methods, headless constructor maintained.
- **D-12 (2026-06-12)** Phase 2 closes with a perf slice (spec 0006: region recomposite + gate measurement) rather than the full original contents list. Free transform, crop tool, and image resample move into Phase 3 (they pair naturally with selections); tablet pressure moves to a brush-dynamics spec later. The Phase 2 gate (golden parity ✓, paint+undo ✓, 60 fps) is the closing criterion, not the original bullet list.
- **D-13 (2026-06-13)** Layer transforms (scale/rotate, spec 0010) are *destructive bakes*: the affine is bilinear-resampled into a fresh tile set and captured as before/after snapshots for undo. Keeps the compositor seeing plain tiles+offset (GPU parity unaffected) and avoids per-layer affine state. Non-destructive transforms are a Smart-Object concern (Phase 10). Interactive on-canvas handles deferred — numeric Transform dialog first.
