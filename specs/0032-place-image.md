# Spec 0032 — Interop II: place image (INT-3)

- **Status:** ☑ done (2026-06-13)
- **Phase:** 5 (raster↔vector interop / import)
- **Requirements:** INT-3 (place raster image into a document), FMT-4 (PNG/JPEG decode)
- **Depends on:** 0002, 0031

## Goal
Place a raster image (PNG/JPEG) from disk into the open document as a new raster layer,
undoably, via File → Place Image….

## Scope
- New workspace dep **`image` 0.25** (png+jpeg, default-features off) on `atelier-io`.
- `atelier-io::image_io`: `DecodedImage { width, height, rgba }`, `decode_image(bytes)` and
  `load_image(path)` (typed `ImageError`, never panics on bad input).
- `TileMap::from_rgba(w, h, &[u8])` (atelier-core) — build tiles from a straight-alpha RGBA8
  buffer (transparent pixels skipped).
- App `place_image(DecodedImage)` inserts a "Placed Image" raster layer above the selection
  (undoable `AddNode`, selected); `place_image_dialog` (rfd pick → `load_image`) on
  File → Place Image….

## Out of scope
- Place as a smart object / linked (DOC-5, later); placing into a chosen position/scale
  (lands at doc origin); TIFF/WebP/GIF/BMP (FMT-4 remainder); drag-and-drop import; ICC from
  the file (Phase 6).

## Verification checklist
- [x] `cargo test -p atelier-io` — PNG encode→`decode_image` round trip; garbage bytes error
- [x] `cargo test -p atelier-app` — `place_image` adds a raster layer with the decoded pixels
      (opaque placed, transparent skipped); undo removes it
- [x] workspace + clippy `--all-targets -D warnings` clean; smoke run

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-13 | `cargo test -p atelier-io` | PASS | `png_round_trip` (3×2 RGBA encode→decode identical), `garbage_bytes_error_not_panic`; io 10 tests |
| 2026-06-13 | `cargo test -p atelier-app` | PASS | `place_image_adds_raster_layer_and_undoes` (2×2 image → raster layer, red placed / transparent skipped, undo removes); app 36 tests |
| 2026-06-13 | workspace + clippy + smoke | PASS | full suite green, clippy clean, app runs 5s no crash |

## Notes / surprises
- `image` decode lives in `atelier-io` (formats crate); `TileMap::from_rgba` is the pure
  core bridge so the app just wires dialog → decode → tiles → `AddNode`.
- Placement is at doc origin, unscaled — interactive place (position/scale handles) and
  smart-object placement are follow-ups.
- Same add-dep → fetch → read-API method as `i_overlay`; `image` 0.25 API was stable
  (`load_from_memory().to_rgba8()`).