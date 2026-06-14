# Plan review — risks, issues, mitigations

Honest review of the goal and plan (requested in the original prompt). Updated as risks
materialize or retire.

## R-01 · Scope is enormous — the defining risk
Photoshop + Illustrator core parity is decades of engineering. **Mitigation:** strict priority
tiers (REQUIREMENTS), sequential phases with verify gates (ROADMAP), and v1 success criteria
defined as six concrete scenarios (VISION) rather than "parity". The plan treats "all core
features of latest PS/AI" as a *direction* (P1/P2 tiers), not a v1 promise. Anything else
would be planning fiction.

## R-02 · PSD/AI format fidelity
PSD is sprawling (effects, smart filters, text engine data); perfect fidelity is unattainable.
.ai is a private format — only its PDF compatibility stream is readable, and some files are
saved without it; complex appearances arrive flattened or as outlines.
**Mitigation:** define a documented supported subset (FMT-1/FMT-3); always import *something*
sane (flattened composite fallback — PSD stores one); show the user an explicit degradation
report; build a fixture corpus early; never round-trip silently lossy.

## R-03 · egui for a pro graphics UI
Immediate-mode UI may strain at deep panel customization, native menus, IME, accessibility.
**Mitigation:** canvas/compositor is pure wgpu and UI-agnostic by architecture (only
`atelier-app` touches egui), so a later migration (e.g., to Slint/custom retained UI) is
contained. Re-evaluate at end of Phase 5; decision recorded in DECISIONS.

## R-04 · Color-managed GPU compositing correctness
Blend-mode math in wrong space (linear vs perceptual, premultiplied alpha mistakes) produces
subtly wrong images that look "off" vs Photoshop. **Mitigation:** CPU reference compositor +
golden tests comparing GPU vs CPU vs Photoshop-rendered reference PNGs (tolerance documented);
lcms2 for all profile conversions — never hand-rolled matrices.

## R-05 · CMYK is a P1 trap
True CMYK documents (native 4-channel editing, not just export conversion) ripple through
every tool. **Mitigation:** v1 ships RGB/Gray native documents + managed CMYK *export* and
soft proofing; native CMYK editing mode is gated behind its own spec and may slip to post-v1
without breaking the v1 criteria. (Deviation from REQUIREMENTS DOC-9/P1 accepted and noted.)

## R-06 · ONNX EP/driver matrix
CUDA versions, DirectML quirks, CoreML conversion failures. **Mitigation:** CPU EP is always
the correctness baseline; EP selection at runtime with graceful fallback + user override;
models chosen for known-good ONNX exports (MobileSAM, U²-Net, LaMa have established exports).
Model licenses verified before bundling URLs (SAM: Apache-2.0; U²-Net: Apache-2.0;
LaMa: Apache-2.0; Real-ESRGAN: BSD-3) — record in `atelier-ai/MODELS.md` when implemented.

## R-07 · Single-developer-agent bandwidth / context limits
The build proceeds across many sessions; context is lost between them. **Mitigation:** the
harness itself — SESSION-STATE.md is the resumable context snapshot (kept current as part of
the per-phase working agreement), specs carry per-feature state, ROADMAP carries global state.
Any session can cold-start from CLAUDE.md → SESSION-STATE.md → active spec.

## R-08 · 32-bit Windows (original prompt asked for Win x86)
wgpu/DX12 tooling, ONNX Runtime, and modern GPU drivers have effectively abandoned 32-bit;
supporting it would forfeit the GPU/AI pillars. **Decision:** dropped, per the prompt's own
fallback clause ("if not feasible… Windows AMD64 + CUDA"). Tier table in VISION reflects this.

## R-09 · Tablet input
Pressure (Windows Ink/WinTab, macOS, X11/Wayland tablets) is fiddly and hardware-dependent.
**Mitigation:** brush engine consumes a normalized input abstraction from day one; pressure is
P1 and mouse-only paths are never blocked by it.

## R-10 · Performance targets vs correctness-first build order
Naive layer-tree walks will miss 60 fps early. **Mitigation:** tiled dirty-rect architecture
from Phase 2 (not retrofitted); perf budget measured per phase gate, but optimization passes
deferred to Phase 15 unless a gate fails outright.

## R-11 · Text engine (CJK, RTL, OpenType)
Typography rabbit hole. **Mitigation:** cosmic-text carries shaping/fallback; v1 commits to
point/area text with correct shaping, not Illustrator-grade typography (TXT-3 is P2).

## R-12 · Windows-only dev environment (current)
This machine is Win11 x64 → Tier-2/3 claims are untested locally. **Mitigation:** CI matrix
(GitHub Actions: windows/macos/ubuntu runners) from Phase 0–1 so cross-platform drift is
caught continuously even though local dev is Windows.

## R-13 · GPU compositor lacks adjustment-layer + offset-shift execution parity
The CPU compositor (which drives the canvas) applies adjustment layers (spec 0009) and
samples offset layers directly; the GPU compositor treats `CompositeOp::Adjust` as a no-op
and relies on CPU-extracted shifted tiles. Golden parity fixtures therefore exclude
adjustment layers. **Mitigation:** GPU is parity-validation only today — the canvas never
uses it — so users see correct output. When the GPU compositor is wired to the canvas (perf
slice), adjustment math must be ported to WGSL and golden fixtures extended to cover
adjustment layers before that switch flips. Tracked; do not wire GPU→canvas until closed.

## R-14 · Session-only data not yet persisted in `.atl` — CLOSED (spec 0053)
Several non-JSON payloads were `#[serde(skip)]` and not written to the `.atl` container.
**Update (spec 0048):** layer masks persisted as `.atl` v2 binary parts. **Update (spec
0053):** smart-object embedded sub-document pixels + embedded masks now persist as v3
dotted-chain parts (`tiles/<a>.<b>/…`). All known session-only payloads are now persisted —
**risk closed.** Residual note: any *new* `#[serde(skip)]` payload must follow the same
binary-part pattern (and bump the schema) before the Phase-7 format freeze.

## Plan-review verdict
Goal is sound and achievable **as tiered**: the original prompt's flexibility clauses
(fallback platform floor, "would be great" phrasing on AI/3D/parity items, explicit
save-state-and-resume instruction) are load-bearing and have been encoded as P-tiers and
phase gates. The two failure modes to police continuously: (1) letting "core parity" creep
into any single phase, (2) skipping verify gates to feel fast. The harness exists to prevent
both.
