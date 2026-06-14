# Spec 0039 — Radial gradient

- **Status:** ☑ done (2026-06-13)
- **Phase:** 2 follow-up (RAS-9 gradient polish)
- **Requirements:** RAS-9 (gradients)
- **Depends on:** 0037 (linear gradient)

## Goal
The Gradient tool gains a Radial mode (Tools-panel checkbox): the drag start is the center,
the drag end sets the radius; fills foreground→transparent radially.

## Scope
- `atelier-raster::fill::gradient_region_radial` — same contract as `gradient_region` but
  `t = clamp(dist(p, center)/radius)`.
- `BrushSettings.gradient_radial` toggle (Tools panel, shown for the Gradient tool);
  `apply_gradient` dispatches linear vs. radial.

## Out of scope
- Angular/reflected/diamond gradients; multi-stop editor; per-stop color UI.

## Verification checklist
- [x] `cargo test -p atelier-raster` — radial gradient brightest at center, monotone outward
- [x] `cargo test -p atelier-app` — radial fill: center alpha > edge alpha, center near-opaque
- [x] workspace + clippy `--all-targets -D warnings` clean; smoke run

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-06-13 | `cargo test -p atelier-raster` | PASS | `radial_gradient_is_brightest_at_center` (center>mid>edge) |
| 2026-06-13 | `cargo test -p atelier-app` | PASS | `radial_gradient_center_brighter_than_edge`; app 42 tests |
| 2026-06-13 | workspace + clippy + smoke | PASS | full suite green, clippy clean, app runs 5s no crash |

## Notes / surprises
- Shares the gradient tool's drag plumbing + `PaintTiles` undo; only the kernel and a toggle
  are new. RAS-9 now covers solid / linear / radial / flood fills (patterns remain).