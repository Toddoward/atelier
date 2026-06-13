# Spec 0012 — Vector engine I: path model + fill/stroke tessellation

- **Status:** ☑ done (2026-06-13)
- **Phase:** 4 (slice a of 3 — GPU render is 0013, pen/shape tools are 0014)
- **Requirements:** VEC-1 (path model: cubic Béziers, subpaths, fill rules), VEC-4 (fill & stroke params), VEC-7 prep (tessellation for GPU)
- **Depends on:** 0011

## Goal
`atelier-vector` holds a real, serde-able vector shape model — cubic-Bézier subpaths with
fill rule, solid fill, and stroke (width/cap/join) — and tessellates shapes to triangle
meshes (positions + indices) via `lyon`, ready for the GPU renderer (spec 0013). Pure crate,
no GPU/UI deps, fully unit-tested. `NodeKind::Vector` is upgraded from the `PlaceholderArt`
rect stub to a `VectorContent` shape list (with a `.atl` migration).

## Scope
- `atelier-vector::path`: `Path { subpaths: Vec<SubPath> }`, `SubPath { start, segs: Vec<Seg>,
  closed }`, `Seg::{Line(p), Cubic(c1,c2,p)}`, `FillRule::{NonZero, EvenOdd}`; builder helpers
  (move/line/cubic/close) and `rect`/`ellipse` constructors; bounds().
- `atelier-vector::shape`: `Shape { path, fill: Option<[f32;4]>, stroke: Option<Stroke> }`,
  `Stroke { color, width, cap, join }`. serde-derive throughout.
- `atelier-vector::tessellate`: `Mesh { vertices: Vec<Vertex{pos:[f32;2], color:[f32;4]}>,
  indices: Vec<u32> }`; `tessellate(&Shape) -> Mesh` filling then stroking via lyon
  (`FillTessellator`/`StrokeTessellator`), honoring fill rule + stroke params.
- `atelier-core`: `NodeKind::Vector(VectorContent)` where `VectorContent { shapes: Vec<Shape> }`
  (re-export the vector types or hold them via a thin core mirror — see design); `.atl` v1→
  still v1 (vector shapes are JSON in the manifest, no binary parts); migration from the old
  `Vector(PlaceholderArt)` shape.
- Tests: path bounds; rect/ellipse construction; fill tessellation of a square (≥2 tris,
  covers area); stroke tessellation produces geometry; even-odd vs non-zero differ on a
  self-overlapping path; serde round-trip of a Shape; `.atl` migration of an old vector node.

## Out of scope
- GPU rendering of meshes (spec 0013); pen/shape/edit tools (spec 0014); booleans
  (Phase 4 later slice, i_overlay); text-as-vector (Phase 11); per-shape blend/opacity
  beyond layer-level; gradients/dashes (stroke dash is a later polish).

## Design notes
- To respect the architecture invariant (`atelier-core` stays dependency-light and the
  document model is the shared vocabulary), the vector shape types live in `atelier-vector`,
  and `atelier-core` depends on `atelier-vector` for `VectorContent` (vector crate is pure:
  serde + lyon + kurbo only, no GPU/UI — allowed inward dep, mirrors how core will reference
  other pure model crates). Record as D-14.
- Tessellation tolerance ~0.1px; mesh color carried per-vertex so the GPU pipeline (0013)
  is a single flat-color triangle shader to start.
- `.atl` migration: an old `Vector(PlaceholderArt{bounds,color})` becomes a `VectorContent`
  with one filled rectangle Shape of that bounds+color.

## Verification checklist
- [ ] `cargo test -p atelier-vector` — path bounds, rect/ellipse, fill/stroke tessellation,
      even-odd vs non-zero, serde round-trip
- [ ] `cargo test -p atelier-core` — VectorContent in tree; node kind name
- [ ] `cargo test -p atelier-io` — `.atl` round-trips a vector shape; old-vector migration
- [ ] workspace + clippy `--all-targets -D warnings` clean
- [ ] (no app/GPU surface this slice — canvas still draws vector layers as their bounds rect
      until 0013; verify build/run unaffected via smoke)

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-13 | `cargo test -p atelier-vector` | PASS | path: rect corners+bounds, ellipse bounds, serde; tessellate: fill square covers ≈100px², stroke produces geometry, even-odd vs non-zero differ on overlap, shape serde |
| 2026-06-13 | `cargo test -p atelier-core` | PASS | VectorContent in NodeKind; tree ops unaffected |
| 2026-06-13 | `cargo test -p atelier-io` | PASS | `.atl` round-trips a vector shape; legacy `Vector{bounds,color}` migrates to one filled rect |
| 2026-06-13 | workspace + clippy `--all-targets -D warnings` + smoke | PASS | clean |

## Notes / surprises
- D-14: `atelier-core` now depends on `atelier-vector` (pure: serde+lyon+kurbo) so
  `NodeKind::Vector(VectorContent)` holds real shapes. core re-exports `VectorContent` +
  `atelier_vector`.
- Test geometry gotcha: even-odd vs non-zero only diverge when the two subpaths share
  winding direction; my first inner loop wound opposite (both rules holed it → equal areas).
- Canvas does not yet render vector shapes (spec 0013 GPU pass) — vector layers are present
  in the model and `.atl` but invisible on canvas until then.
- lyon path builder uses `begin/line_to/cubic_bezier_to/end`; fill rule via
  `FillOptions::with_fill_rule`. Per-vertex color in the mesh keeps the future GPU shader a
  single flat-color triangle pass.
