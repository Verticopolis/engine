<!--
Thanks for contributing to the Verticopolis engine! Fill in the sections below.
Keep the simulation free of host access (tests/independence.rs), use
American English, and write no em-dashes.
Keep every ## heading and write something under each one: "None" or
"n/a: <reason>" where a section does not apply. The PR template check
(.github/workflows/pr-template.yml) fails a description that drops a heading.
Opening a PR through the API or `gh pr create --body` skips this template, so
start that body from this file.
-->

## What & why

<!-- What does this change do, and what problem does it solve? Link any related issue. -->

Closes #

## Type of change

- [ ] Bug fix (non-breaking change that fixes an issue)
- [ ] Classic-canon fix (matches the 1994 original for all towers)
- [ ] Modern-mode rule (opt-in, "what the original couldn't do")
- [ ] New API (a call, a field, an event or a catalog value)
- [ ] Refactor / internal-only (no change to how the game plays or to the API)
- [ ] Docs / tooling

## Game mode impact

<!--
Towers are founded once as "classic" or "modern" (immutable for the tower's life).
Classic must stay faithful to SimTower 1994; Modern is the opt-in "what the original
couldn't do" layer. Keep behavior the two modes disagree on in a single mode-resolved
rule-set (resolved once at founding) rather than branching on the mode ad hoc or
smearing mode checks through the simulation.
If the change cannot affect gameplay (e.g. docs/tooling), write "n/a: <reason>"
instead of ticking a box.
-->

- [ ] This change is **classic-canon** (affects all towers, stays faithful to the 1994 original)
- [ ] This change is **modern-only** (gated behind Modern mode, does not alter Classic behavior)
- [ ] This change is **mode-agnostic** (identical in both modes)
- [ ] Any mode-divergent behavior is isolated in the mode rule-set (resolved once at founding), not smeared into the simulation with ad-hoc mode checks

## Quality gates

<!-- All must be green before merge. -->

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --locked --all-targets -- -D warnings`
- [ ] `cargo test --locked`
- [ ] `cargo run --release --features conformance --bin conformance` (the referee)
- [ ] `(cd consumer && cargo test --locked)`
- [ ] `npm run typecheck`, `npm run lint`, `npm run format:check` and `npm test`, **or** this PR changes no TypeScript, JavaScript or CSS

## Versioning

<!--
Only a release PR edits the version or CHANGELOG.md (CONTRIBUTING.md,
"Releasing"). Every other change to the crate adds a change note under
.changes/ named after its branch, with its crate tier (major / minor / patch,
or none) and its changelog entries (.changes/README.md). A change to how the
game plays regenerates the conformance lock in the same PR.
-->

- [ ] Added a change note with its crate tier under `.changes/`, **or** this PR changes nothing the crate ships
- [ ] Regenerated the conformance lock, **or** this PR does not change how the game plays
- [ ] Updated the guides on docs.verticopolis.com and the READMEs (`CRATE_README.md`, `packages/verticopolis-engine/README.md`) that went stale, **or** read them and none did

## Complexity

<!--
A change to a hot path (the engine step and its build, the WASM binding, saves
and the other surfaces in section 1 of
https://github.com/Verticopolis/web/blob/main/docs/performance/hot-path-review.md)
fills in the note from section 5 of that page: hot paths and callers, sizes,
complexity before and after, remaining scans and why, the caches, indexes and
counters the engine has there and whether each is kept (HP-8), invalidation
evidence, measurements. A significant change (section 5) gives every field.
Any other change may replace the first line below with "n/a: <reason>". The
second line links the section 6 review record once a review has posted it on
this PR, or says "no hot-path surface".
-->

Hot paths and call frequency (tier), sizes, complexity before and after, remaining scans and why, caches kept or replaced (HP-8), invalidation evidence, measurements:

Hot-path record (section 6): link

## Notes for reviewers

<!-- Canon references, trade-offs, follow-ups, anything that helps the review. "None" if nothing. -->
