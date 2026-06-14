# Roadmap

Top-down, sequential. One phase = one or more specs in `specs/`. A phase is **done** only when
its verification checklist passes (build green, tests pass, manual checks recorded in the
spec's Verification Log). Do not start phase N+1 with phase N red.

Status legend: ☐ not started · ◐ in progress · ☑ done

Specs written so far: Phase 0 → 0001 ☑ · Phase 1 → 0002 ☑ · Phase 2 → 0003 ☑ + 0004 ☑ + 0005 ☑ + 0006 ☑ (gate met). Phase 3 → 0007 ☑ + 0008 ☑ + 0009 ☑ + 0010 ☑ + 0011 ☑ — **Phase 3 complete**. Deferred: quick-mask, interactive transform handles, magnetic lasso, refine-edge.
Phase 4 → 0012 ☑ (path model + tessellation) + 0013 ☑ (canvas render) + 0014 ☑ (rect/ellipse tools) + 0015 ☑ (polygon/star tools) + 0016 ☑ (pen tool) + 0017 ☑ (direct-select anchor move) + 0018 ☑ (add/remove anchors) + 0019 ☑ (segment-click insert) + 0020 ☑ (bezier handle model) + 0021 ☑ (handle-drag UI — interactive path editing complete) + 0022 ☑ (align vector layer to canvas, no-dep VEC-6 subset). + 0023 ☑ (rasterize vector layer, INT-2) + 0024 ☑ (compound paths, VEC-8) + 0025 ☑ (anti-aliased vector rasterize) + 0026 ☑ (align/distribute shapes in a layer) + 0027 ☑ (layer duplicate) + 0028 ☑ (multi-select + group/ungroup) + 0029 ☑ (cross-layer align/distribute — VEC-6 complete) + 0030 ☑ (copy/paste layers) + 0031 ☑ (boolean path ops / Pathfinder via i_overlay — VEC-5). **Phase 4 vector engine COMPLETE.** Next: Phase 5 — focus modes & raster↔vector interop (place image INT-3, smart objects, z-interleaved compositing), or Phase 6 color management.

| # | Phase | Contents (req IDs) | Verify gate | Status |
|---|-------|--------------------|-------------|--------|
| 0 | Bootstrap | Workspace scaffold; window + wgpu surface + egui docked panels; CI (build+test, Win x64); empty canvas pan/zoom (SH-1..3) | `cargo build` + `cargo test` green; window opens; panels dock; canvas pans/zooms at 60 fps | ☑ |
| 1 | Document model | Layer tree, groups, node kinds (stubs ok), command/undo, history panel, `.atl` save/load v0, layers panel UI, layer duplicate (spec 0027) (DOC-1..4,6,7) | Unit tests for tree ops + undo invariants; save→load→identical; manipulate layers in UI | ☑ |
| 2 | Raster engine | Tile store, GPU compositor w/ full blend-mode set, CPU reference, brush/eraser, move, canvas resize, region recomposite (RAS-1,2,4,5 subset; DOC-3,8; re-scoped per D-12) | Golden CPU=GPU blend tests; paint+undo correct; 60 fps target doc | ☑ |
| 3 | Selections & adjustments | Selection tools+combine ops, marching ants, magic wand, feather/grow/shrink/invert/all; core adjustments destructive + adjustment layers; transform, crop, image resample (RAS-3,4,5,6; DOC-1) | Mask op unit tests; per-tool kittest; adjustment layer re-render correctness | ☑ |
| 4 | Vector engine | Path model, pen/direct-select, shapes, fill/stroke, booleans, align, tessellated GPU render (VEC-1..7) | Boolean op test corpus; crisp zoom; editing checklist | ☑ |
| 5 | Focus modes & interop | New-doc focus chooser, workspace presets, rasterize vector layer, place image, cross-paste (INT-1..4) | Interop checklist both directions | ◐ (INT-1 groundwork, INT-2 ☑ spec 0023, INT-3 ☑ spec 0032; z-interleaved vector compositing ☑ spec 0051) |
| 6 | Color management | lcms2 integration, working spaces, assign/convert, display profile, color picker/swatches (COL-1,2,5,6) | Round-trip ΔE tests vs reference values; visual proof on wide-gamut display path | ☐ |
| 7 | Formats I | PNG/JPEG/TIFF/WebP/BMP/GIF with ICC; SVG import; `.atl` v1 freeze + spec doc (FMT-4,5; DOC-7) | Fixture corpus round-trips; fuzz smoke; degradation report UI | ◐ (FMT-4 raster import/export done specs 0032-0034, sans ICC; SVG + ICC remain) |
| 8 | PSD | PSD import (P0 subset) then export; degradation reporting (FMT-1,2) | Real-world PSD corpus renders within tolerance vs reference PNGs; PS-opens-our-export check | ☐ |
| 9 | AI vector formats | .ai import via PDF-compat stream; PDF export; SVG export (FMT-3,6; COL-3,4 soft proof) | AI fixture corpus → editable paths; exported PDF/SVG validates in 3rd-party tools | ☐ |
| 10 | Smart objects & effects | Smart objects embedded/linked, layer effects, clipping masks (DOC-5,10; FMT-9 prep) | Non-destructive transform checklist; edit-source-updates-instances test | ◐ (clipping masks ☑ spec 0046; smart-object embed & composite ☑ spec 0052; persist embedded docs ☑ spec 0053 — edit-contents & transform next) |
| 11 | Text | Point/area text, styles, outline conversion (TXT-1,2) | Shaping tests (Latin+CJK sample), outline conversion correctness | ☐ |
| 12 | AI assist | ort + EP selection, model manager, Select Subject, BG removal, cloud endpoint settings (AI-1,2,3,7,8) | Model download+checksum; mask quality eval on fixture set; EP fallback matrix test | ☐ |
| 13 | 3D texture tools | Normal/bump/height generation, strength controls, lit preview (3D-1,2,5) | Known-input → known-normal-map golden tests; visual preview check | ☐ |
| 14 | Configuration | Settings dialog, keymap editor + presets, modifier options (CFG-1..4) | Rebind/conflict/persist checklist; settings migration test | ☐ |
| 15 | Hardening & v1 | Autosave/recovery, perf pass vs §12 targets, packaging (MSI/dmg/AppImage), Tier-2 platform builds (SH-5) | Perf numbers recorded; installers smoke-tested; v1 success criteria (VISION) all pass | ☐ |

Post-v1 (P2/P3 backlog): content-aware fill (AI-4), image trace (VEC-9, AI-5), upscale (AI-6),
healing tools, gradients/patterns, artboards++, PDF/X-4, EXR, AO/roughness/tiling (3D-3,4),
text-on-path, smart-object PSD round-trip, localization.

## Working agreement per phase

1. Write/refresh the spec (`specs/NNNN-*.md`) from REQUIREMENTS — includes scope, design
   notes, out-of-scope, and a concrete verification checklist.
2. Implement smallest vertical slice first; keep `main` (trunk) green.
3. Run the checklist; record results in the spec's Verification Log with date.
4. Update ROADMAP status + SESSION-STATE.md before ending a session.
