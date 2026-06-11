---
name: implement-spec
description: Workflow for implementing one Atelier feature spec from specs/ — scoping, vertical-slice implementation, verification, and state handoff. Use whenever starting, resuming, or finishing work on a spec or roadmap phase.
---

# Implement a spec

The unit of work in this repo is one spec file (`specs/NNNN-*.md`). Phases in
`docs/ROADMAP.md` map to one or more specs.

## 1. Orient
- Read `docs/SESSION-STATE.md`, then the target spec. If no spec exists for the work,
  write one first from `specs/0000-TEMPLATE.md` + the relevant REQUIREMENTS rows. Set spec
  status to ◐ and update ROADMAP.

## 2. Scope check
- Confirm scope fits the phase: if you discover required work outside the spec, either
  (a) add it to the spec's Scope with a note, or (b) file it as a new spec / backlog line in
  ROADMAP. Never silently expand.
- Check `docs/RISKS.md` for a relevant risk; honor recorded mitigations and decisions (D-N).

## 3. Implement vertical slices
- Smallest end-to-end slice first (model → render → UI reachable), then widen.
- Respect architecture invariants in `CLAUDE.md` (crate boundaries, Command-pattern edits,
  tiles, lcms2-only color).
- Write tests alongside: unit tests in the owning crate; golden-image tests when pixels are
  involved; fixture files under `assets/fixtures/`.
- Keep `cargo build --workspace && cargo test --workspace` green between slices.

## 4. Verify (gate — never skip)
- Execute every line of the spec's Verification checklist. Automate what can be automated;
  run the app for the manual lines (`cargo run -p atelier-app`).
- Record results in the spec's **Verification Log** with date and pass/fail per item.
  A failed item means the spec stays ◐ — fix or explicitly descope with a note.

## 5. Hand off
- Update spec status (☑ when all checks pass), ROADMAP phase row, and
  `docs/SESSION-STATE.md` (done / in-flight / next / surprises).
- Add any new decisions to `docs/DECISIONS.md`, new risks to `docs/RISKS.md`.
- Commit with conventional-commit message scoped to the spec (e.g. `feat(vector): booleans per spec 0007`).
