# Roadmap

Top-down, sequential. One phase = one or more specs in `specs/`. A phase is **done** only when
its verification checklist passes (build green, tests pass, manual checks recorded in the
spec's Verification Log). Do not start phase N+1 with phase N red.

Status legend: ☐ not started · ◐ in progress · ☑ done

Specs written so far: Phase 0 → `specs/0001-bootstrap-shell.md` · Phase 1 → `specs/0002-document-model.md` · Phase 2 → `specs/0003-raster-tiles-cpu-compositor.md` (slice a; slices b/c = GPU parity 0004, brush/tools 0005)

| # | Phase | Contents (req IDs) | Verify gate | Status |
|---|-------|--------------------|-------------|--------|
| 0 | Bootstrap | Workspace scaffold; window + wgpu surface + egui docked panels; CI (build+test, Win x64); empty canvas pan/zoom (SH-1..3) | `cargo build` + `cargo test` green; window opens; panels dock; canvas pans/zooms at 60 fps | ☑ |
| 1 | Document model | Layer tree, groups, node kinds (stubs ok), command/undo, history panel, `.atl` save/load v0, layers panel UI (DOC-1..4,6,7) | Unit tests for tree ops + undo invariants; save→load→identical; manipulate layers in UI | ☑ |
| 2 | Raster engine | Tile store, GPU compositor w/ full blend-mode set, CPU reference, brush/eraser, move/transform, crop/resize (RAS-1,2,4,5; DOC-3,8) | Golden CPU=GPU blend tests; paint+undo correct; 60 fps target doc | ◐ |
| 3 | Selections & adjustments | Selection tools+combine ops, quick mask, marching ants; core adjustments destructive + adjustment layers (RAS-3,6; DOC-1) | Mask op unit tests; visual checklist per tool; adjustment layer re-render correctness | ☐ |
| 4 | Vector engine | Path model, pen/direct-select, shapes, fill/stroke, booleans, align, tessellated GPU render (VEC-1..7) | Boolean op test corpus; crisp zoom; editing checklist | ☐ |
| 5 | Focus modes & interop | New-doc focus chooser, workspace presets, rasterize vector layer, place image, cross-paste (INT-1..4) | Interop checklist both directions | ☐ |
| 6 | Color management | lcms2 integration, working spaces, assign/convert, display profile, color picker/swatches (COL-1,2,5,6) | Round-trip ΔE tests vs reference values; visual proof on wide-gamut display path | ☐ |
| 7 | Formats I | PNG/JPEG/TIFF/WebP/BMP/GIF with ICC; SVG import; `.atl` v1 freeze + spec doc (FMT-4,5; DOC-7) | Fixture corpus round-trips; fuzz smoke; degradation report UI | ☐ |
| 8 | PSD | PSD import (P0 subset) then export; degradation reporting (FMT-1,2) | Real-world PSD corpus renders within tolerance vs reference PNGs; PS-opens-our-export check | ☐ |
| 9 | AI vector formats | .ai import via PDF-compat stream; PDF export; SVG export (FMT-3,6; COL-3,4 soft proof) | AI fixture corpus → editable paths; exported PDF/SVG validates in 3rd-party tools | ☐ |
| 10 | Smart objects & effects | Smart objects embedded/linked, layer effects, clipping masks (DOC-5,10; FMT-9 prep) | Non-destructive transform checklist; edit-source-updates-instances test | ☐ |
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
