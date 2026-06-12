# .atl native format

Open, versioned container. Spec evolves per phase; loaders must reject
`schema_version` greater than they understand (tested) and migrate older ones.

## Schema v1 (Phase 2, current)

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

- v2 (Phase 4): `paths/<layer-id>.json` — vector shape lists
- v2 (Phase 6): embedded ICC profile part; real `color_mode`
- v2 (Phase 10): embedded smart-object sub-documents as nested `.atl` parts
- Freeze + publish at Phase 7 (ROADMAP); changes after freeze require migrations + tests
