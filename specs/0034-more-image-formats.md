# Spec 0034 — Formats: TIFF / WebP / GIF / BMP import & export

- **Status:** ☑ done (2026-06-13)
- **Phase:** 7 groundwork (completes FMT-4 raster codec set, sans ICC)
- **Requirements:** FMT-4 (PNG/JPEG/TIFF/WebP/GIF/BMP)
- **Depends on:** 0032, 0033

## Goal
Place and Export support TIFF, WebP, GIF, and BMP in addition to PNG/JPEG — no new dependency,
just `image` crate features and dialog filters.

## Scope
- Enable `image` features `tiff`, `webp`, `gif`, `bmp` (added to png/jpeg).
- `IMPORT_EXTENSIONS` / `EXPORT_EXTENSIONS` constants in `atelier-io`; Place and Export dialogs
  use them. `decode_image` already format-agnostic (`load_from_memory`); `save_image` infers
  format from the extension.

## Out of scope
- ICC profile read/write (Phase 6); animated GIF/WebP frames (first frame only); 16-bit/HDR
  (EXR is FMT-8/P3); EXR/TIFF-float.

## Verification checklist
- [x] `cargo test -p atelier-io` — lossless TIFF and BMP save→load round-trip pixel-identical
      (plus existing PNG round trips)
- [x] workspace + clippy `--all-targets -D warnings` clean; smoke run

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-13 | `cargo test -p atelier-io` | PASS | `lossless_formats_round_trip` (TIFF + BMP pixel-identical); io 14 tests |
| 2026-06-13 | workspace + clippy + smoke | PASS | full suite green, clippy clean, app runs 5s no crash |

## Notes / surprises
- One extension list drives both dialogs and keeps Place/Export in sync.
- GIF/WebP are tested only indirectly (decode is format-agnostic); explicit round-trip tests
  use the lossless formats (TIFF/BMP) to assert exact pixels. JPEG/GIF are lossy/palette so a
  pixel-exact assertion wouldn't hold.