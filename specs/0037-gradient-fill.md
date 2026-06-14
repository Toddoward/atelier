# Spec 0037 — Linear gradient fill

- **Status:** ☑ done (2026-06-13)
- **Phase:** 2 follow-up (RAS-9 gradient — completes the fill set bar patterns)
- **Requirements:** RAS-9 (gradients)
- **Depends on:** 0036 (fill), 0035 (brush color)

## Goal
A Gradient tool (key `G`): drag an axis on the canvas to fill the selection (or whole layer)
of the selected raster layer with a foreground→transparent linear gradient, undoably.

## Scope
- `atelier-raster::fill::gradient_region(tiles, c0, c1, p0, p1, offset, region, mask?)` —
  two-stop linear gradient (projection onto the p0→p1 axis, clamped), straight-alpha src-over,
  mask + offset aware.
- `ActiveTool::Gradient` + Tools-panel entry + `G` (plain; `Ctrl+G` stays Group). Canvas drag
  rubber-bands the axis (live line preview); on release `apply_gradient` fills from
  `brush.color` to a transparent copy of it, capturing tiles and committing one `PaintTiles`.

## Out of scope
- Multi-stop gradient editor; radial / angular / reflected gradients; gradient on vector fills;
  dithering; gradient between two arbitrary picked colors (foreground→transparent for now).

## Verification checklist
- [x] `cargo test -p atelier-raster` — gradient interpolates monotonically along the axis
      (start opaque, end transparent)
- [x] `cargo test -p atelier-app` — drag a horizontal gradient → left more opaque than right;
      undo clears
- [x] workspace + clippy `--all-targets -D warnings` clean; smoke run

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-13 | `cargo test -p atelier-raster` | PASS | `gradient_interpolates_along_axis` (a0>230, a9<60, monotonic) |
| 2026-06-13 | `cargo test -p atelier-app` | PASS | `gradient_fill_via_pointer_and_undo` (left>right opacity, undo clears); app 40 tests |
| 2026-06-13 | workspace + clippy + smoke | PASS | full suite green, clippy clean, app runs 5s no crash |

## Notes / surprises
- Reuses the shape/selection drag plumbing (`select_drag`) for the axis and `PaintTiles` for
  undo — only the fill kernel is new.
- Foreground→transparent is the single-color default; a two-color or multi-stop gradient is a
  natural follow-up once a gradient UI exists.