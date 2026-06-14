# Spec 0053 — Persist embedded smart-object documents (.atl v3)

- **Status:** ☑ done (2026-06-14)
- **Phase:** 1/10 (`.atl` format ⇄ smart objects DOC-5)
- **Requirements:** DOC-5 (smart objects), FMT (.atl), closes last R-14 item
- **Depends on:** 0052 (smart objects embed & composite), 0048 (.atl v2 mask parts)

## Goal
Embedded smart-object pixels (and embedded layer masks) survive `.atl` save→load. The embedded
document *structure* already serializes inline in the manifest (spec 0052); this adds the
binary pixel/mask parts for nodes that live inside embedded documents, at any nesting depth.

## Scope
- `.atl` schema **v3**: tile/mask part keys become a **dotted node-id chain** addressing a node
  through nested embedded docs — `tiles/<a>.<b>.<c>/<tx>_<ty>.bin`, `masks/<a>.<b>.bin`, where
  `a`,`b` are the smart-object node ids descended into and the final id is the raster node in
  the deepest embedded doc. Top-level nodes keep the old single-id key (`tiles/<id>/…`), so the
  chain is just `[id]`.
- `save_atl`: recurse — for `Raster` write its tiles+mask under the current key; for `Smart`
  recurse into `content.doc` with the key prefix extended by the smart node's id.
- `load_atl`: parse the dotted chain, resolve it to the target `RasterContent` by descending
  `Smart(content).doc` for each non-final id, and reattach. v1/v2 files (single-id keys) resolve
  as a one-element chain → unchanged behaviour (no migration needed).

## Out of scope
- Linked (external-file) smart objects — embedded only.
- De-duplicating identical embedded docs (each instance stores its own parts).

## Verification checklist
- [x] `cargo test -p atelier-io` — a doc containing a smart object with embedded pixels **and**
      an embedded layer mask round-trips deep-equal; nested (smart-in-smart) pixels survive
- [x] existing v0/v1/v2 round-trip + malformed-part + version tests still pass (back-compat)
- [x] `cargo build/test --workspace`; clippy `--all-targets -D warnings` clean

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-14 | `cargo test -p atelier-io` | PASS | `round_trips_embedded_smart_object` (pixels+mask deep-equal), `round_trips_nested_smart_object` (smart-in-smart); io 17 tests |
| 2026-06-14 | back-compat | PASS | `loads_v0_schema_files`, `rejects_malformed_tile_part`, `round_trips_layer_mask`, vector/adjustment round-trips all still green |
| 2026-06-14 | workspace + clippy | PASS | full suite green; clippy `--all-targets -D warnings` clean |

## Notes / surprises
- Each embedded `Document` has its own id space (ids restart at 0), so a flat `tiles/<id>/`
  key would collide across nesting levels — the dotted chain disambiguates by path, not id.
