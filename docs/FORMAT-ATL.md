# .atl native format

Open, versioned container. Spec evolves per phase; loaders must reject
`schema_version` greater than they understand (tested) and migrate older ones.

## Schema v3 (current)

Adds, on top of v2: persistence of pixels/masks that live **inside embedded smart-object
documents** (spec 0053). Part keys become a **dotted node-id chain** addressing a node through
nested embedded documents:

| Part | Content |
|------|---------|
| `tiles/<a>.<b>.<c>/<tx>_<ty>.bin` | a tile of the raster node reached by descending smart-object nodes `a`, `b` and selecting raster node `c` in the deepest embedded doc |
| `masks/<a>.<b>.bin` | the layer mask of that nested raster node (same header+coverage layout as v2) |

Top-level nodes keep the single-id key (`tiles/<id>/…`, `masks/<id>.bin`) — i.e. a one-element
chain — so v1/v2 files load unchanged (no migration needed). The embedded document *structure*
serializes inline in the manifest (`Smart(SmartContent{ doc, offset })`); only its pixel/mask
bytes ride in these parts. An unresolvable chain (or non-numeric segment) is a hard
`BadTilePart` load error, never a panic.

## Schema v2 (superseded)

Adds, on top of v1:

| Part | Content |
|------|---------|
| `masks/<node-id>.bin` | one per raster layer that has a layer mask: lz4 (size-prepended) of a 16-byte header (`x0,y0,w,h` as i32 LE) + `w·h` coverage bytes over the mask's tight bounds (spec 0048) |

v1 files load unchanged (no mask parts → masks stay `None`); no JSON migration needed.

## Schema v1 (Phase 2)

ZIP archive:

| Part | Content |
|------|---------|
| `manifest.json` | `{ "schema_version": 1, "document": <Document JSON> }` |
| `tiles/<node-id>/<tx>_<ty>.bin` | one per existing 256² tile of each raster layer: lz4 block compression with prepended size (`lz4_flex`), decompressed = 256·256·4 RGBA8 straight-alpha bytes, stored uncompressed in the zip |

Raster payload in JSON is `RasterContent { art: PlaceholderArt|null }` — the `tiles` field
never appears in JSON; the loader reattaches binary parts after deserialization. Malformed
tile part names/bytes are a hard load error (`BadTilePart`), never a panic.

Migration v0→v1 (in `load_atl`): raster `kind` payload was a bare `PlaceholderArt`; it is
rewrapped as `{ "art": <old> }`. v0 files carry no pixels.

## Schema v0 (Phase 1, superseded)

ZIP archive containing exactly:

| Part | Content |
|------|---------|
| `manifest.json` | `{ "schema_version": 0, "document": <Document JSON> }` |

`Document` JSON (serde-derived from `atelier-core`):
- `size`: `[w, h]` document pixels
- `focus`: `"Raster" \| "Vector"` (workspace preset, INT-1)
- `color_mode`: string tag (stub until Phase 6; currently `"RGB8"`)
- `nodes`: map `NodeId → Node`; `root`; `next_id` (monotonic id counter, ids never reused)
- `Node`: `{ props: { name, visible, locked, opacity, blend, clip }, kind, parent, children }`
  - `children` is top-of-layer-panel first
  - `kind`: `Group{expanded}` | `Raster(PlaceholderArt)` | `Vector(PlaceholderArt)` |
    `Adjustment` | `Text` | `Smart` | `Fill` (stubs until their phases)
  - `PlaceholderArt`: `{ bounds: [x,y,w,h], color: [r,g,b,a] }` — Phase-1 stand-in,
    removed when real payloads land

## Planned (bump `schema_version` and add a migration each time)

- (Phase 6): embedded ICC profile part; real `color_mode`
- (Phase 10): linked (external-file) smart objects; embedded-doc de-duplication
- Freeze + publish at Phase 7 (ROADMAP); changes after freeze require migrations + tests
