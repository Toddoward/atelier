# Requirements

Priorities: **P0** = v1 cannot ship without it · **P1** = v1 target · **P2** = post-v1 ·
**P3** = aspirational. Each shipped requirement must have a spec in `specs/` before
implementation and a verification checklist before it is marked done (see CLAUDE.md workflow).

## 1. Application shell & platform

| ID | Requirement | Pri |
|----|-------------|-----|
| SH-1 | Native desktop app; windowed UI with dockable panels (tools, layers, properties, color, history) | P0 |
| SH-2 | GPU-rendered canvas via wgpu (Vulkan/DX12/Metal/GL fallback); CPU reference path for tests | P0 |
| SH-3 | Tier-1: Windows x86-64. Tier-2: macOS Apple Silicon, Linux x86-64. Tier-3: Win ARM64, macOS Intel. No 32-bit | P0 |
| SH-4 | Multiple open documents (tabbed), per-document undo history | P0 |
| SH-5 | Crash-safe: autosave/recovery journal | P1 |
| SH-6 | Localization scaffold (English first; strings externalized) | P2 |

## 2. Document model & layers

| ID | Requirement | Pri |
|----|-------------|-----|
| DOC-1 | Document = layer tree. Node kinds: raster layer, vector layer, group, adjustment layer, text layer, smart object, fill layer | P0 |
| DOC-2 | Groups: unlimited nesting, group opacity/blend/mask, pass-through blend mode | P0 |
| DOC-3 | Per-layer: opacity, blend mode (full PS set: normal, multiply, screen, overlay, soft/hard light, color dodge/burn, darken/lighten, difference, exclusion, hue/sat/color/luminosity, etc.), visibility, lock flags | P0 |
| DOC-4 | Layer masks (raster) and vector masks; clipping masks | P0 |
| DOC-5 | Smart objects: embedded sub-document, non-destructive transform, open-source-and-update; linked (file-on-disk) variant | P1 |
| DOC-6 | Command-pattern edits; unlimited (configurable) undo/redo; history panel | P0 |
| DOC-7 | Native file format `.atl`: documented, versioned, zip-of-parts (manifest JSON + binary tile/path data), forward-compatible | P0 |
| DOC-8 | Color depth 8/16-bit integer per channel; 32-bit float | P0 (8/16), P1 (32f) |
| DOC-9 | Document color modes: RGB, Grayscale; CMYK | P0 (RGB/Gray), P1 (CMYK) |
| DOC-10 | Layer effects (drop shadow, stroke, outer/inner glow, color/gradient overlay, bevel) | P1 |
| DOC-11 | Artboards (multiple per document, vector-focus default) | P1 |

## 3. Raster editing

| ID | Requirement | Pri |
|----|-------------|-----|
| RAS-1 | Tiled raster storage (256² tiles), sparse, GPU-composited | P0 |
| RAS-2 | Brush + eraser: size/hardness/opacity/flow/spacing, smoothing; tablet pressure (WinTab/Ink) | P0 (pressure P1) |
| RAS-3 | Selections: rectangle, ellipse, lasso, polygonal, magic wand; add/subtract/intersect; feather, grow/shrink, invert; quick-mask; marching ants | P0 |
| RAS-4 | Transform: move, free transform (scale/rotate/skew), interpolation choices | P0 |
| RAS-5 | Crop tool, canvas resize, image resample | P0 |
| RAS-6 | Adjustments (destructive + as adjustment layers): levels, curves, brightness/contrast, hue/saturation, color balance, black & white, vibrance, invert, posterize, threshold | P0 core set, P1 full |
| RAS-7 | Filters: gaussian/box/motion blur, sharpen/unsharp mask, noise add/reduce, median; GPU compute where possible | P1 |
| RAS-8 | Clone stamp, healing-brush (classic algorithmic) | P1 |
| RAS-9 | Gradients (linear/radial/angle) + fill bucket, patterns | P1 |
| RAS-10 | Content-aware fill (AI inpaint-restoration, see AI-3) | P2 |

## 4. Vector editing

| ID | Requirement | Pri |
|----|-------------|-----|
| VEC-1 | Path model: cubic Béziers, multiple subpaths, even-odd/non-zero fill rules | P0 |
| VEC-2 | Pen tool (add/remove/convert anchors), direct selection (move anchors/handles) | P0 |
| VEC-3 | Shape primitives: rect, rounded rect, ellipse, polygon, star, line | P0 |
| VEC-4 | Fill & stroke: solid, gradient; stroke width/cap/join/dash, stroke alignment | P0 |
| VEC-5 | Boolean ops: unite, subtract, intersect, exclude (Pathfinder) | P0 |
| VEC-6 | Align/distribute, group/ungroup, arrange order | P0 |
| VEC-7 | GPU rendering via tessellation; anti-aliased, resolution-independent zoom | P0 |
| VEC-8 | Compound paths, clipping paths | P1 |
| VEC-9 | Image trace (raster → vector), classic + AI-assisted (AI-5) | P2 |
| VEC-10 | Effects on vectors: corner rounding, offset path, simplify path | P2 |

## 5. Raster ↔ vector interop

| ID | Requirement | Pri |
|----|-------------|-----|
| INT-1 | Per-project focus chooser (raster/vector) at New Document; switchable later; affects workspace preset + defaults only | P0 |
| INT-2 | Rasterize vector layer (at document or chosen resolution) | P0 |
| INT-3 | Place raster image into vector-focus doc (as raster layer / smart object) | P0 |
| INT-4 | Copy/paste paths and pixels across documents of either focus | P0 |
| INT-5 | Vector selection → raster selection (path to mask) and back (mask outline to path) | P1 |

## 6. Text

| ID | Requirement | Pri |
|----|-------------|-----|
| TXT-1 | Point text + area text; system font enumeration; size/leading/tracking/kerning; fill/stroke | P1 |
| TXT-2 | Text stays editable; rasterize/outline-to-path conversions | P1 |
| TXT-3 | OpenType features, text on a path | P2 |

## 7. Color management & print

| ID | Requirement | Pri |
|----|-------------|-----|
| COL-1 | ICC profile support via Little CMS: load, embed, assign vs convert | P0 |
| COL-2 | Working spaces: sRGB, Adobe RGB, Display P3, ProPhoto RGB, Gray gamma 2.2, common CMYK (FOGRA39/51, GRACoL2013, Japan Color 2011) | P0 RGB, P1 CMYK |
| COL-3 | Rendering intents (perceptual, rel. colorimetric + BPC, saturation, abs. colorimetric) | P1 |
| COL-4 | Soft proofing + gamut warning | P1 |
| COL-5 | Display profile aware (OS monitor profile) | P1 |
| COL-6 | Color picker (HSB/RGB/Lab/CMYK/hex), swatches panel, eyedropper | P0 |
| COL-7 | PDF/X-compatible export path (PDF/X-4) | P2 |

## 8. File formats

| ID | Requirement | Pri |
|----|-------------|-----|
| FMT-1 | Import PSD: layer tree, groups, raster pixels (8/16-bit, RGB/Gray), opacity/blend/visibility, masks; graceful degradation list reported to user | P0 |
| FMT-2 | Export PSD: round-trip of the subset above; everything else flattened with warning | P1 |
| FMT-3 | Import AI: parse the PDF compatibility stream → vector layers (paths, fills, strokes, text-as-outlines fallback) | P1 |
| FMT-4 | Import/export PNG, JPEG, TIFF (w/ ICC), WebP, BMP, GIF | P0 |
| FMT-5 | Import/export SVG (vector layers) | P0 import, P1 export fidelity pass |
| FMT-6 | Export PDF (vector + raster, embedded ICC) | P1 |
| FMT-7 | Import PDF pages as documents | P2 |
| FMT-8 | EXR / HDR import | P3 |
| FMT-9 | Smart-object aware PSD round-trip | P2 |

## 9. AI assist (non-generative)

| ID | Requirement | Pri |
|----|-------------|-----|
| AI-1 | ONNX Runtime integration; execution providers: CUDA (Win/Linux), DirectML (Win fallback), CoreML (macOS), CPU (always) | P1 |
| AI-2 | Select Subject / click-to-select (MobileSAM-class) producing editable masks | P1 |
| AI-3 | Background removal (U²-Net/RMBG-class) | P1 |
| AI-4 | Inpainting restoration for content-aware fill (LaMa-class) | P2 |
| AI-5 | AI-assisted trace/edge refine | P2 |
| AI-6 | Super-resolution upscale (Real-ESRGAN-class) | P2 |
| AI-7 | Model manager: download on demand from configurable URLs, checksum verify, local cache, offline-friendly | P1 |
| AI-8 | Cloud fallback: Settings page for base URL + API key + model name; OpenAI-compatible image endpoints where applicable; explicit per-call user consent indicator | P1 |
| AI-9 | No generative (diffusion/text-to-image) features | P0 (constraint) |

## 10. 3D-asset texture tools

| ID | Requirement | Pri |
|----|-------------|-----|
| 3D-1 | Generate normal map from image (Sobel/Scharr height-to-normal, strength/invert controls) | P1 |
| 3D-2 | Generate bump/height map (luminance-based + AI depth optional later) | P1 |
| 3D-3 | Ambient occlusion + roughness approximation from height | P2 |
| 3D-4 | Tiling preview (offset wrap) and seamless-tile assist | P2 |
| 3D-5 | Live lit preview of normal map (simple lambert/phong sphere or plane) | P2 |

## 11. Configuration

| ID | Requirement | Pri |
|----|-------------|-----|
| CFG-1 | Settings dialog: performance (GPU adapter, tile cache size, undo limit), UI (theme, language), tools, AI endpoints | P0 |
| CFG-2 | Keymap editor: every command rebindable, conflict detection, import/export, presets ("Atelier default", "Photoshop-like", "Illustrator-like") | P1 (P0: static default keymap) |
| CFG-3 | Modifier-key behavior options (e.g., space=pan, alt=eyedropper/clone-source) | P1 |
| CFG-4 | Settings stored as versioned TOML in OS config dir; safe migration | P0 |

## 12. Performance targets (non-functional)

- 60 fps pan/zoom on 8k×8k document with 50 layers, mid-range discrete GPU (Tier-1).
- Brush latency < 25 ms paint-to-screen at 1000-px brush on 4k canvas.
- Cold start < 3 s (excluding first-run shader compile).
- Memory: tile cache bounded by user setting; documents larger than RAM via tile eviction (P2).
- All format importers fuzz-tolerant: malformed files must error, never crash (P0).
