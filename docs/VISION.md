# Vision — "Atelier" (working codename, repo: photo-illustration-shop)

> This document is the refined, canonical statement of the project goal. It supersedes the
> original free-form prompt, which is preserved verbatim in `docs/ORIGINAL-PROMPT.md`.

## One-sentence goal

A single cross-platform, GPU-accelerated desktop application that unifies raster image editing
(Photoshop-class) and vector illustration (Illustrator-class) in one document model, with
PSD/AI interchange, professional color management, and locally-run (or user-configured cloud)
computer-vision AI assist tools — **without** generative AI.

## Product pillars

1. **One document model, two focuses.** At project creation the user picks a *focus*:
   - **Raster focus** — Photoshop-like workspace: pixel layers, brushes, selections, filters,
     adjustment layers.
   - **Vector focus** — Illustrator-like workspace: artboards, paths, shapes, strokes/fills,
     boolean path operations, typography.

   The focus selects workspace layout, default tools, and document defaults — it does **not**
   wall off capability. Every document can contain both raster and vector layers, mirroring how
   classic Photoshop hosted shape/path layers and classic Illustrator could place and
   rasterize images. Interop is first-class: rasterize a vector layer, embed a raster image in
   a vector project, copy/paste both ways, shared color and transform systems.

2. **Layer system as the spine.** Flexible layer management is the core differentiator:
   - Unlimited nesting of **groups**, group-level masks/opacity/blend modes.
   - **Smart objects**: embedded or linked sub-documents with non-destructive transforms;
     editing the source re-renders all instances.
   - Adjustment layers, clipping masks, layer/vector masks, layer effects.
   - Fast reordering, multi-select operations, search/filter in the layer panel.

3. **Professional color & print.** ICC color management end to end (Little CMS):
   document profiles (sRGB, Adobe RGB, Display P3, ProPhoto, gray, CMYK such as
   FOGRA39/51, GRACoL, Japan Color), assign vs. convert, rendering intents, soft proofing
   with gamut warning, 8/16/32-bit channels, PDF/X-ready export path.

4. **Interchange, not lock-in.** High compatibility with the existing ecosystem:
   - **Import**: PSD (layered), AI (via its PDF compatibility stream), SVG, PDF, PNG, JPEG,
     TIFF, WebP, GIF, BMP, EXR (stretch).
   - **Export**: PSD (layered subset), SVG, PDF, PNG, JPEG, TIFF, WebP.
   - Native format is open and documented (`.atl`), designed so other tools can read it.

5. **AI assist, not AI generate.** Classic CV models, run locally via ONNX Runtime
   (CUDA / DirectML / CoreML execution providers) or via a user-configured cloud endpoint
   (base URL + API key in Settings). Target tools:
   - Select Subject / one-click object selection (SAM-family, MobileSAM baseline)
   - Background removal (U²-Net / RMBG-class)
   - Content-aware fill via inpainting-restoration models (LaMa-class)
   - Edge-aware refine mask, auto trace assist (raster → vector)
   - Optional: super-resolution upscale (Real-ESRGAN-class)
   No text-to-image, no diffusion generation. Models are downloaded on demand by a model
   manager; nothing phones home without explicit configuration.

6. **3D-asset texture tooling** (classic-Photoshop style): generate normal maps, bump/height
   maps, ambient occlusion approximations, roughness maps from images; tiling/offset preview;
   live 3D-lit preview of a normal map.

7. **GPU-first.** Rendering and compositing run on the GPU (wgpu → Vulkan/DX12/Metal/GL),
   tiled canvas, compute-shader filters. CPU fallback must exist for correctness tests.

8. **Deep configurability.** Every command rebindable (keymap editor with conflict detection,
   Photoshop/Illustrator preset keymaps), modifier-key behavior settings, tool options,
   theming, performance settings (GPU selection, tile size, undo limits, scratch disk).

## Platform targets

| Tier | Platform | GPU compute / AI backend | Commitment |
|------|----------|--------------------------|------------|
| 1 | Windows x86-64 | wgpu (DX12/Vulkan); ONNX Runtime CUDA → DirectML fallback | Primary, always green |
| 2 | macOS Apple Silicon | wgpu (Metal); ONNX Runtime CoreML (MPS-backed) | Build + test regularly |
| 2 | Linux x86-64 | wgpu (Vulkan); ONNX Runtime CUDA → CPU fallback | Build + test regularly |
| 3 | Windows ARM64, macOS Intel | wgpu native; ONNX CPU | Best-effort, no release gate |
| — | Windows x86 (32-bit) | — | **Not supported** (modern GPU/AI stacks have effectively dropped 32-bit; documented trade-off, see RISKS R-08) |

Per the original prompt: if full breadth proves infeasible, the floor is **Windows x86-64 +
CUDA** — the architecture is chosen so the other platforms are compile-target work, not
rewrites.

## Explicit non-goals

- Generative AI (text-to-image, generative fill/expand). **Rationale: scope, not ideology.**
  It's a large subsystem (model weights/hosting, prompt UX, safety, licensing) orthogonal to
  the app's core job of precise raster+vector editing — including it would be
  over-engineering for these goals. Classic CV *assist* (selection, masking) stays, because
  it directly accelerates editing. (See D-13.)
- Full Photoshop/Illustrator feature parity at v1 — we target the *core* feature set
  (see REQUIREMENTS.md priority tiers); parity grows by roadmap phase.
- 3D scene editing (only 2D texture-map generation for 3D pipelines).
- Animation/video timelines.
- Cloud documents, collaboration, accounts.

## Success criteria (v1)

1. Open a real-world layered PSD; layer tree, blend modes, and composite visually match
   Photoshop output within documented tolerances; re-export preserves the layer structure.
2. Open an AI file's PDF-compat content as editable-where-possible vector layers.
3. Create a vector-focus project, draw/boolean/edit paths, export clean SVG and PDF.
4. Create a raster-focus project, paint with pressure, select subject via local AI model,
   apply adjustment layer, export print-ready CMYK TIFF with embedded ICC profile.
5. Generate a normal map + height map from a photo texture and preview it lit.
6. All of the above with the GPU compositor active at 60 fps pan/zoom on a 4k-layer document
   (mid-range discrete GPU), and every shipped feature covered by its spec's verification
   checklist.
