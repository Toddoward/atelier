# Spec 0048 — Persist layer masks in `.atl` (schema v2)

- **Status:** ☑ done (2026-06-13)
- **Phase:** 7 (format — closes the mask half of R-14)
- **Requirements:** DOC-7 (native format), DOC-4 (masks persist)
- **Depends on:** 0047 (layer masks)

## Goal
Layer masks survive save/load: `.atl` schema bumps to v2, writing a binary mask part per
masked raster layer; older files still load.

## Scope
- `Mask::to_region_bytes` / `from_region_bytes` — dump/rebuild a mask over its tight bounds.
- `.atl` `SCHEMA_VERSION = 2`; save writes `masks/<node-id>.bin` = lz4(size-prepended) of a
  16-byte header (`x0,y0,w,h` i32 LE) + coverage; load reattaches them. v1/v0 files load
  unchanged (no mask parts). Malformed mask parts error (reuse `BadTilePart`), never panic.

## Out of scope
- Embedded smart-object sub-documents (the other half of R-14 — done with smart objects);
  vector masks; mask compression beyond lz4.

## Verification checklist
- [x] `cargo test -p atelier-io` — a doc with a layer mask round-trips (coverage preserved
      in/out); existing v0/v1-compat + future-version-reject tests still pass
- [x] workspace + clippy `--all-targets -D warnings` clean; smoke run

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-13 | `cargo test -p atelier-io` | PASS | `round_trips_layer_mask` (coverage 200 in / 0 out preserved); io 15 tests incl. v0/v1 compat |
| 2026-06-13 | workspace + clippy + smoke | PASS | full suite green, clippy clean, app runs 5s no crash |

## Notes / surprises
- R-14 is now closed for masks; embedded smart-object docs remain (will need their own nested
  parts when smart objects land).
- v2 needs no JSON migration — mask parts are purely additive; absence = no mask.