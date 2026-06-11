# Architecture

## Stack decision

**Language: Rust (stable).** Memory safety for a large long-lived codebase, first-class
cross-compilation (Windows x64/ARM64, macOS x64/aarch64, Linux), zero-cost FFI to C libraries
we need (Little CMS, ONNX Runtime), and the best current cross-platform GPU abstraction.

**GPU: wgpu.** One API → Vulkan (Win/Linux), DX12 (Win), Metal (macOS), GL fallback.
Compute shaders (WGSL) for filters/adjustments; render pipelines for compositing and vector
tessellation output. Satisfies "GPU hardware acceleration" on every target without
per-platform renderer code. CUDA/MPS specifically are *AI inference* concerns, handled by
ONNX Runtime execution providers — not by hand-written kernels.

**UI: winit + egui (egui-wgpu, egui_dock).** Immediate-mode keeps panel/tool UI velocity high
and shares the wgpu device with the canvas. Risk and possible later migration documented in
RISKS R-03. Canvas is our own wgpu render target embedded in the UI, not an egui widget tree.

**Key libraries**

| Concern | Crate | Notes |
|---------|-------|-------|
| Geometry | `kurbo` | Béziers, affine transforms |
| Tessellation | `lyon` | fill/stroke → triangles for GPU |
| Path booleans | `i_overlay` | unite/subtract/intersect/xor |
| Color management | `lcms2` | Little CMS bindings; ICC, intents, soft proof |
| Text shaping | `cosmic-text` (`rustybuzz`, `swash`) | shaping, fallback, layout |
| Image codecs | `image`, `zune-jpeg`, `tiff` | + ICC chunk handling |
| PSD | `psd` (read bootstrap) → custom `atelier-psd` reader/writer | spec: Adobe PSD file format docs; writer must be ours |
| AI (.ai) | `pdf` / custom parser of PDF-compat stream | text → outlines fallback |
| SVG | `usvg` (import), custom export | |
| PDF export | `pdf-writer` | |
| AI inference | `ort` (ONNX Runtime) | EPs: CUDA, DirectML, CoreML, CPU |
| Settings | `serde` + `toml`, `directories` | versioned config |
| Undo | custom command pattern | in `atelier-core` |

## Workspace layout (cargo)

```
photo-illustration-shop/
├─ Cargo.toml                 # [workspace]
├─ crates/
│  ├─ atelier-app/            # binary: shell, panels, tools wiring, event loop
│  ├─ atelier-core/           # document model, layer tree, commands/undo, selection model
│  ├─ atelier-raster/         # tile store, brush engine, CPU reference compositor
│  ├─ atelier-vector/         # path model, editing ops, booleans, tessellation
│  ├─ atelier-gpu/            # wgpu device mgmt, tile compositor, WGSL shaders, filters
│  ├─ atelier-color/          # lcms2 wrapper, profiles, conversions, soft proof
│  ├─ atelier-text/           # text layout/shaping → vector outlines
│  ├─ atelier-io/             # native .atl format + common codecs glue
│  ├─ atelier-io-psd/         # PSD read/write
│  ├─ atelier-io-ai/          # .ai (PDF-compat) import, PDF/SVG export
│  ├─ atelier-ai/             # ort runtime, model manager, cloud client, mask post-processing
│  ├─ atelier-3d/             # normal/bump/AO/roughness generation, lit preview
│  └─ atelier-settings/       # config schema, keymap, migration
└─ assets/                    # icons, default keymaps, test fixtures
```

Dependency rule: arrows point inward to `atelier-core`. `atelier-app` is the only crate that
knows about all others. No crate except `atelier-gpu` touches wgpu; no crate except
`atelier-ai` touches ort. `atelier-core` has **no** GPU/UI dependencies → fully unit-testable.

## Core model sketch

```rust
Document { id, size, color: (Mode, IccProfile, Depth), root: Group, artboards, history }
enum Node { Raster(RasterLayer), Vector(VectorLayer), Group(Group), Adjustment(AdjLayer),
            Text(TextLayer), Smart(SmartObject), Fill(FillLayer) }
common: { name, visible, locked, opacity, blend: BlendMode, mask: Option<Mask>, clip: bool }
RasterLayer { tiles: TileMap /* sparse 256² */, depth }
VectorLayer { shapes: Vec<Shape { path: kurbo::BezPath, fill, stroke }> }
SmartObject { source: Embedded(DocumentId) | Linked(PathBuf), transform: Affine, cache: TileMap }
```

- **Edits**: every mutation is a `Command { apply, revert }` recorded in per-document history.
  Tools produce commands; UI never mutates the model directly.
- **Compositing**: dirty-rect propagation up the tree; GPU compositor walks the tree per
  256² tile, blending in document color space (linear-light for relevant modes); result
  color-converted to display profile in the final present pass.
- **Selections** are 8-bit masks (raster) and/or paths (vector) with lossless path→mask and
  approximate mask→path conversion (INT-5).
- **CPU reference compositor** in `atelier-raster` mirrors GPU semantics; golden-image tests
  compare both (tolerance ≤ 1 LSB 8-bit) — this is the correctness anchor for blend modes.

## AI subsystem

- `ort` with runtime EP selection: try CUDA → DirectML (Windows) / CoreML (macOS) → CPU.
  Setting override in CFG. Models stored under app data dir; manifest with URL + sha256.
- Cloud: trait `VisionBackend { segment(image, prompt) -> Mask; remove_bg; inpaint }` with
  `LocalOnnx` and `HttpEndpoint` implementations; endpoint config = base URL, key, model id.
- All AI outputs land as ordinary selections/masks/layers — fully editable, undoable.

## Native format `.atl`

ZIP container: `manifest.json` (schema-versioned tree + metadata), `tiles/<layer>/<x>_<y>.bin`
(lz4-compressed pixel tiles), `paths/<layer>.json`, embedded ICC, embedded smart-object
sub-documents as nested `.atl`. Open spec maintained in `docs/FORMAT-ATL.md` (written with
Phase 1).

## Testing strategy

- Unit tests per crate (model, booleans, color math).
- Golden-image tests: CPU vs GPU compositor; importer fixtures (PSD/AI/SVG corpus under
  `assets/fixtures/`).
- Importer fuzzing (cargo-fuzz, P1) — malformed files must never crash (REQ §12).
- Each spec ships with a verification checklist executed before the feature is "done"
  (see `.claude/skills/implement-spec`).
- Headless GPU tests run on a software adapter (wgpu + llvmpipe / WARP) in CI.

## Platform notes

- Windows: DX12 default backend (broadest driver quality), Vulkan opt-in; DirectML AI
  fallback for non-NVIDIA. Tablet input via Windows Ink, WinTab fallback.
- macOS: Metal; ONNX CoreML EP (MPS-backed). Universal binary later (Tier 2 = AS first).
- Linux: Vulkan; X11+Wayland via winit; CUDA EP when present.
- 32-bit Windows: not supported (RISKS R-08).
