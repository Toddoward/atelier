# Atelier (photo-illustration-shop)

Cross-platform GPU raster+vector image editor (Photoshop/Illustrator hybrid). Rust workspace,
wgpu, egui. Read order for a cold session:

1. `docs/SESSION-STATE.md` — where work stopped, what's next (always current; update before ending any session)
2. `docs/ROADMAP.md` — phase table with status and verify gates
3. The active spec in `specs/` (lowest-numbered ◐)
4. `docs/VISION.md`, `docs/REQUIREMENTS.md`, `docs/ARCHITECTURE.md`, `docs/RISKS.md`, `docs/DECISIONS.md` — consult as needed, don't re-read wholesale every session

## Hard rules

- **Spec before code.** No feature work without a spec in `specs/` (template: `specs/0000-TEMPLATE.md`). Use the `implement-spec` skill workflow.
- **Verify before done.** A spec/phase is done only when its Verification checklist is executed and results are recorded in its Verification Log. Never mark done on "should work".
- **Sequential phases.** Follow ROADMAP order; don't open phase N+1 while N is red.
- **Keep the trunk green.** `cargo build --workspace` and `cargo test --workspace` must pass at every session end; if they can't, record the breakage in SESSION-STATE.md.
- **No generative AI features** (AI-9 constraint). CV-assist only.
- **Update SESSION-STATE.md before ending every session.** It is the resume point; stale state is worse than no state.
- **Record decisions** in `docs/DECISIONS.md` (D-N), risks/changes in `docs/RISKS.md`.

## Architecture invariants (see docs/ARCHITECTURE.md)

- Only `atelier-gpu` imports wgpu; only `atelier-ai` imports ort; only `atelier-app` imports egui/winit.
- `atelier-core` (document model, commands, undo) stays free of GPU/UI deps — pure, unit-testable.
- All model mutations go through `Command` objects (undo/redo invariant). UI never mutates the model directly.
- Pixels live in sparse 256² tiles; compositing is dirty-rect driven.
- All color conversions via `atelier-color` (lcms2). No ad-hoc color math outside it.
- CPU reference compositor is the blend-mode source of truth; GPU must match within 1 LSB (8-bit).

## Build & test

```powershell
cargo build --workspace          # debug build
cargo test --workspace           # all tests
cargo run -p atelier-app         # launch the app
cargo clippy --workspace -- -D warnings   # lint gate (CI-enforced)
```

Windows 11 x64 dev box; rustup-managed stable toolchain. GPU tests that need a real adapter
are `#[ignore]`-gated; CI uses software adapters.

## Conventions

- Crate names `atelier-*`; module-per-feature; no `mod.rs` (use `foo.rs` + `foo/`).
- Errors: `thiserror` per crate, `anyhow` only in `atelier-app`.
- Importers must degrade gracefully and never panic on malformed input (fuzz target eventually).
- Commit style: conventional commits (`feat(raster): …`, `fix(io-psd): …`).
