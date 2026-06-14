# Spec 0033 — Formats: export flattened document to PNG / JPEG

- **Status:** ☑ done (2026-06-13)
- **Phase:** 7 groundwork (FMT-4 export side; lands now that the compositor + image dep exist)
- **Requirements:** FMT-4 (PNG/JPEG export)
- **Depends on:** 0032 (image dep), 0006 (compositor)

## Goal
Export the flattened document to a PNG or JPEG file via File → Export Image…, choosing the
format by file extension.

## Scope
- `atelier-io::encode_png(w, h, rgba)` and `save_image(path, w, h, rgba)` — RGBA8 in; PNG
  keeps alpha, JPEG (.jpg/.jpeg) flattens to RGB. Buffer-size validation (`ImageError::Buffer`).
- App `export_to(path)` composites the document (`atelier_raster::composite_rgba8`) and writes
  it; `export_image_dialog` (rfd save, PNG/JPEG filters) on File → Export Image….

## Out of scope
- ICC profile embedding (Phase 6); TIFF/WebP/GIF/BMP (FMT-4 remainder — add `image` features);
  per-layer / artboard export; export scale/region; PSD/SVG/PDF export (later format phases).

## Verification checklist
- [x] `cargo test -p atelier-io` — encode_png → decode round trip; save_image writes a
      readable PNG; encode rejects a mismatched buffer
- [x] `cargo test -p atelier-app` — export a doc with a placed red square → reload the PNG →
      correct size, red where drawn, transparent elsewhere
- [x] workspace + clippy `--all-targets -D warnings` clean; smoke run

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-13 | `cargo test -p atelier-io` | PASS | `encode_png_round_trips_through_decode`, `save_image_writes_a_readable_png`, `encode_png_rejects_mismatched_buffer`; io 13 tests |
| 2026-06-13 | `cargo test -p atelier-app` | PASS | `export_document_to_png` (64×64 out, red where placed, transparent outside); app 37 tests |
| 2026-06-13 | workspace + clippy + smoke | PASS | full suite green, clippy clean, app runs 5s no crash |

## Notes / surprises
- Export = `composite_rgba8` (the CPU compositor, which already handles all layer/blend/
  adjustment-layer cases) → `save_image`. The flatten path is fully shared with the canvas.
- JPEG drops alpha (RGB) by design; PNG preserves it.
- ICC-tagged export waits for the color-management phase (lcms2).