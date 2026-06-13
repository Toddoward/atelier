# Spec 0013 — Vector engine II: render vector layers on the canvas

- **Status:** ☑ done (2026-06-13)
- **Phase:** 4 (slice b of 3)
- **Requirements:** VEC-7 (anti-aliased, resolution-independent vector render), VEC-1/4 (shapes visible)
- **Depends on:** 0012

## Goal
Vector layers become visible: each shape is tessellated (spec 0012) and drawn on the canvas,
transformed by the viewport (pan/zoom), over the raster composite. Fills and strokes show in
their colors with anti-aliasing; zoom stays crisp (geometry re-tessellated in document space,
vertices mapped to screen each frame).

## Scope
- Canvas (`atelier-app`): for each visible vector layer in tree order, tessellate its shapes
  (`atelier_core::atelier_vector::tessellate`) and emit an `egui::epaint::Mesh` whose vertices
  are viewport-mapped (doc→screen) and colored from the shape mesh; paint above the raster
  composite, below the selection ants. Layer opacity scales vertex alpha.
- Tessellation cache keyed by history revision (re-tessellate only on document change; the
  per-frame cost is just the affine vertex map).
- Respect the canvas clip rect; premultiplied-alpha-correct color via `Color32`.

## Out of scope
- A bespoke `atelier-gpu` vector pipeline (egui already renders meshes via wgpu; a dedicated
  pipeline is a later perf option — noted). Interleaving vector & raster by true z-order in
  one compositor (vectors currently draw as an overlay above the raster composite; full
  interop is Phase 5). Per-shape blend modes; gradients; dashes. Pen/shape tools (spec 0014)
  — this slice has no authoring UI, so tests construct vector layers directly.

## Design notes
- egui flat-color mesh: `Vertex { pos, uv: WHITE_UV, color }`, `texture_id: default` (font
  atlas white texel) — solid triangles, AA from egui's feathering at mesh edges is limited;
  acceptable for slice b (true MSAA/analytic AA is a polish item).
- Resolution independence: tessellate in doc space (curves smooth at doc scale), map vertices
  by the viewport affine each frame. At extreme zoom this can facet; re-tessellation at screen
  scale is deferred (noted).
- Cache: `EditorState.vector_cache: Option<(u64 rev, Vec<(NodeId, vector::Mesh)>)>`.

## Verification checklist
- [ ] `cargo test -p atelier-app` — kittest: a doc with a filled-rectangle vector layer
      produces a non-empty painted mesh (assert via a tessellation+map helper returning the
      egui mesh vertex count / bbox); cache invalidates on revision change
- [ ] workspace + clippy `--all-targets -D warnings` clean
- [ ] [manual·non-gating] `cargo run`: add a vector layer (via test fixture / future tool) and
      see a crisp colored shape that pans/zooms with the canvas

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-13 | `cargo test -p atelier-app` (kittest) | PASS | `vector_layer_tessellates_and_caches`: filled-rect vector layer → 1 cached non-empty mesh; cache invalidates on revision bump |
| 2026-06-13 | workspace + clippy `--all-targets -D warnings` | PASS | full suite green (app 22), clippy clean |

## Notes / surprises
- Rendered via `egui::epaint::Mesh` (flat-color triangles, WHITE_UV/font-atlas texel), not a
  bespoke `atelier-gpu` pipeline — egui already renders meshes on wgpu and this keeps the
  "only atelier-gpu imports wgpu" invariant while getting correct paint order (above the
  raster composite, below ants). A dedicated GPU pipeline remains an option if profiling
  demands it.
- Vectors paint as an overlay above the raster composite; true z-interleaving of raster and
  vector layers in one compositor is Phase 5 interop work.
- Tessellation cached in doc space per revision; per-frame cost is just the affine
  vertex→screen map. At extreme zoom geometry can facet (tessellated at doc scale);
  re-tessellation at screen scale deferred.
