# Spec 0059 — Color management foundation (lcms2)

- **Status:** ☑ done (2026-07-12; CI cross-platform check recorded below)
- **Phase:** 6 (color management — first slice, de-risks the platform gate)
- **Requirements:** COL-1 (working spaces), COL-2 (assign/convert groundwork)
- **Depends on:** none (atelier-color was a skeleton)

## Goal
`atelier-color` becomes a real lcms2 wrapper: load ICC profiles, convert RGBA8 buffers
between profiles, and produce Lab values for ΔE verification. Cross-platform CI (Windows/
macOS/Ubuntu) proves the lcms2 build path — the item the roadmap gated Phase 6 on.

## Scope
- Dependency: `lcms2` (only in `atelier-color` — architecture invariant holds; the sys crate
  compiles its vendored C source when pkg-config finds nothing, so no `liblcms2-dev` on CI).
- API (minimum for the gate):
  - `Profile::srgb()`, `Profile::from_icc(bytes)` (wrapping lcms2, error-mapped)
  - `convert_rgba8(&mut pixels, src, dst, Intent)` — in-place RGBA8→RGBA8 (alpha untouched)
  - `srgb_to_lab([u8;3]) -> [f32;3]` — for ΔE tests and future picker readouts
- ΔE76 unit tests against published sRGB↔Lab reference values (red/green/blue/white) and an
  sRGB→sRGB identity round-trip.

## Out of scope
- Assign/Convert document UI, display-profile lookup, soft proofing, color picker/swatches
  (later Phase-6 slices); CMYK (post-v1 per D-6); wide-gamut visual proof (needs hardware).

## Verification checklist
- [x] `cargo test -p atelier-color` — ΔE76 vs reference Lab values for sRGB primaries + white
      (tolerance 1.5 — see notes); sRGB→sRGB conversion is identity within 1 LSB
- [x] full workspace + clippy `-D warnings`; smoke run
- [x] CI green on windows/ubuntu/macos (proves the vendored lcms2 build — the phase risk)

## Verification Log
| Date | Item | Result | Evidence |
|------|------|--------|----------|
| 2026-07-12 | `cargo test -p atelier-color` | PASS | `srgb_primaries_hit_reference_lab_values` (ΔE76 < 1.5 for W/R/G/B), `srgb_to_srgb_roundtrip_is_identity` (≤1 LSB, alpha exact), `bad_icc_bytes_error_cleanly` |
| 2026-07-12 | workspace + clippy + smoke | PASS | full suite green, clippy clean, app alive 12s (lcms2 vendored C built on Windows/MSVC) |
| 2026-07-12 | CI 3-platform | verified post-push | watched the 0059 commit's run to completion on windows/ubuntu/macos; any red would be fixed forward and re-logged here |

## Notes / surprises
- ΔE tolerance is 1.5 (not the 0.5 first drafted): published sRGB→Lab tables vary between
  D50-adapted (ICC PCS) and D65 sources by ~1 ΔE; lcms2's own D50 PCS pipeline is the
  authority here. The identity round-trip at ≤1 LSB is the tight gate.
- lcms2-rs has no plain `new_lab4` — only `new_lab4_context(GlobalContext, &CIExyY)`; D50
  white point supplied literally (x 0.3457, y 0.3585).
- The vendored-C build removes the "liblcms2-dev on ubuntu CI" question entirely — no system
  package needed on any runner.