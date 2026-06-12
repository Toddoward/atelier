# Spec 0003 — Raster engine I: tile store, blend-mode math, CPU reference compositor

- **Status:** ☑ done (2026-06-12)
- **Phase:** 2 (slice a of 3 — GPU parity is spec 0004, brush/transform tools spec 0005)
- **Requirements:** RAS-1 (tile storage), DOC-3 (blend-mode math), DOC-8 (8-bit first), FMT (atl v1 tile parts)
- **Depends on:** 0002

## Goal
Raster layers hold real pixels in sparse 256² tiles; a pure-CPU compositor flattens any
document subtree to RGBA8 with the full Photoshop blend-mode set, group isolation /
pass-through, and opacity — unit-tested against W3C/PDF compositing formulas. This CPU
path is the **source of truth** the GPU compositor (spec 0004) must match within 1 LSB
(D-9, R-04). `.atl` gains schema v1 with lz4-compressed binary tile parts.

## Scope
- `atelier-core::tile`: `TILE_SIZE=256`, `Tile` (RGBA8, straight alpha), `TileMap`
  (sparse `BTreeMap<(i32,i32), Tile>`), pixel get/set, `fill_rect`, iteration for io/GPU.
  Tile data is `#[serde(skip)]` in the JSON manifest (binary parts instead) but included
  in `PartialEq` so round-trip tests stay honest.
- `atelier-core::node`: `NodeKind::Raster(RasterContent { art: PlaceholderArt, tiles: TileMap })`.
  Placeholder art stays for the egui canvas until spec 0004 renders tiles; new layers
  fill their tiles with the placeholder rect so CPU/GPU output matches the placeholder view.
- `atelier-raster::blend`: all 28 `BlendMode`s as `B(cb, cs)` per-pixel functions —
  separable per W3C compositing / PDF spec, non-separable (Hue/Saturation/Color/
  Luminosity, Darker/LighterColor) via SetLum/SetSat/ClipColor, Dissolve as deterministic
  hash-threshold, HardMix as sum-threshold.
- `atelier-raster::compositor`: `composite_rgba8(doc, w, h) -> Vec<u8>`; f32 straight-alpha
  internal; standard formula `ao = as + ab(1-as)`,
  `Co = (as(1-ab)Cs + as·ab·B(Cb,Cs) + (1-as)ab·Cb)/ao`; groups: isolated buffer +
  Normal/blend-mode composite with group opacity; `PassThrough` at opacity 1 composites
  children directly onto the backdrop (PS semantics), `PassThrough` at opacity <1 falls
  back to isolated-Normal (documented simplification).
- `atelier-io`: `.atl` schema v1 — manifest JSON unchanged shape, plus
  `tiles/<node-id>/<tx>_<ty>.bin` parts (lz4_flex, size-prepended). v0 files still load;
  reject >1. New workspace dep: `lz4_flex`.
- Tests: tile invariants; per-mode math spot-checks against hand-computed W3C values;
  compositing identities (Normal over transparent = source; opacity scaling; group
  isolated vs pass-through divergence; no NaN/out-of-range across mode × alpha grid);
  io v1 round trip with pixels + v0 compat load.

## Out of scope
- GPU compositor + canvas tile rendering (spec 0004); brush/eraser, move/transform,
  crop/resize (spec 0005); clipping-mask & layer-mask compositing (Phase 3); dirty-rect
  incremental recomposite (spec 0004, where it pays); color management (Phase 6 — math is
  sRGB-component-space like classic PS 8-bit); Photoshop golden-image fixtures (R-04 —
  added when a PS-rendered corpus exists; until then W3C-formula hand checks anchor).

## Design notes
- Tile data lives in `atelier-core` as pure bytes (no GPU/UI deps — invariant holds);
  `atelier-raster` owns *operations* (blend, composite). Architecture table amended
  mentally: "tile store" = data in core, engine in raster. Rationale: `Command` undo and
  serde need pixels reachable from `Document`.
- Straight (unassociated) alpha storage like PS; compositor works in f32.
- Dissolve randomness = `hash(x, y, 0x9E3779B9)` threshold — deterministic across
  runs/platforms so golden tests stay stable.
- `RasterContent` keeps `PlaceholderArt` only as canvas stand-in; remove in spec 0004.

## Verification checklist
- [ ] `cargo test -p atelier-core` — tile store invariants
- [ ] `cargo test -p atelier-raster` — blend math spot checks (hand-computed values),
      compositing identities, full mode×alpha sweep finite & in-range
- [ ] `cargo test -p atelier-io` — v1 round trip with tiles, v0-compat load, garbage reject
- [ ] `cargo test --workspace` + `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Compositor output for a placeholder-filled layer equals the placeholder color over
      transparent checker (automated pixel assert)

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-12 | `cargo test -p atelier-core` | PASS | 19 tests (4 new tile tests: transparent reads, cross-border set/get incl. negative coords, fill_rect tile spanning + prune, byte-length validation) |
| 2026-06-12 | `cargo test -p atelier-raster` | PASS | 12 tests: 13 separable modes vs hand-computed W3C values, soft-light spec identities, non-separable lum invariants, full mode×alpha-grid finite/in-range sweep, dissolve determinism + ~50% threshold; compositor: source-over-transparent identity, multiply ±1 LSB, opacity coverage, isolated-vs-passthrough divergence, hidden layers, all-modes deterministic, dissolve all-or-nothing |
| 2026-06-12 | `cargo test -p atelier-io` | PASS | 6 tests: v1 deep-equal round trip incl. pixels, pixel-value spot check after reload, v0-schema migration load, malformed tile part rejection, future-version rejection, garbage rejection |
| 2026-06-12 | full workspace + clippy `--all-targets -D warnings` | PASS | 48 tests total; clippy clean |
| 2026-06-12 | placeholder-filled layer composites to its color over transparency | PASS | `single_layer_over_transparent_is_source` (automated pixel assert) |

## Notes / surprises
- `RasterContent.tiles` must carry `#[serde(skip)]` at the *field* level — skip inside
  `TileMap` still leaves a required `"tiles"` key in the manifest JSON (caught by the
  v0-compat test on first run).
- Model change `Raster(PlaceholderArt)` → `Raster(RasterContent)` required a v0→v1 JSON
  migration in the loader; FORMAT-ATL.md updated.
- Photoshop golden-image corpus still absent (R-04): blend math is anchored to W3C-spec
  hand-computed values for now; revisit when PS-rendered fixtures land (planned Phase 8 prep).
- Dissolve gates alpha through a deterministic xy-hash so CPU/GPU outputs can be compared
  exactly in spec 0004.
