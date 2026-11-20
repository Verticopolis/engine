# CLAUDE.md

This repository is the Verticopolis engine: the Rust crate (`engine-rs/`),
its WebAssembly and npm package (`@verticopolis/engine`, with the TypeScript
layer in `packages/verticopolis-engine/`), the conformance suite
(`conformance/`), the docs site and the bench. Every simulation rule, price,
cap and tuning value lives here. [README.md](README.md) has the layout and
the checks; [CONTRIBUTING.md](CONTRIBUTING.md) has pull requests, change
notes, releases and the registries. Read both.

## Load the game's repository too

The agent workflow is not copied here. The BMAD/BMGD skills, the review
skills (`/gds-code-review`, `/bmad-code-review`), the hot-path checklist
(`docs/performance/hot-path-review.md`) and the agent rules
(`AGENTS.md`, `_bmad-output/project-context.md`) live in
[Verticopolis/web](https://github.com/Verticopolis/web). Load it in the same
session, and run its review skill on every non-trivial change here:
`/gds-code-review` for simulation, binding, frame and save work,
`/bmad-code-review` for CI, packaging and tooling.

## Non-negotiables

- **The engine is the one simulation.** A frontend never adds, copies or
  patches a rule; a rule change lands here, with a regenerated conformance
  lock in the same PR. Ask for only what any frontend could use.
- **Change notes, never a version edit.** A PR that changes what the crate
  or the npm package ships adds `.changes/<branch-slug>.md` with its `crate:`
  tier (major, minor, patch or none). Any change to Rust code under
  `engine-rs/src/` takes patch or higher. Only the release PR raises the
  version and writes `engine-rs/CHANGELOG.md`.
- **Quality gates before pushing:** `cargo fmt --all -- --check`,
  `cargo clippy --locked --all-targets -- -D warnings`, `cargo test --locked`,
  the conformance referee, and for the TypeScript `npm run typecheck`,
  `npm run lint`, `npm run format:check` and `npm test`. Prettier owns the
  TypeScript layout; run `npm run format` before committing.
- **No guard tests.** Never add a test that reads source, workflow or
  configuration files. Check workflows with actionlint and real CI runs.
- **No host access in the simulation** (`engine-rs/tests/independence.rs`),
  and a 500-line ceiling per source file.
- **American English everywhere. No em-dashes in prose** (docs, comments,
  commit and PR text); use commas, colons, parentheses or separate
  sentences. Skip the "X, not Y" emphatic pattern and marketing vocabulary.
- **Merge commits only** to `main`, never a squash. Keep a branch's own
  history readable before it merges.
- **PR descriptions follow `.github/PULL_REQUEST_TEMPLATE.md`:** keep every
  `## ` heading and fill each one.
