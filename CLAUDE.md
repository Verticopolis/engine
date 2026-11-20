# CLAUDE.md

This repository is the Verticopolis engine: the Rust crate (the repository
root: `Cargo.toml`, `src/`, `tests/`),
its WebAssembly and npm package (`@verticopolis/engine`, with the TypeScript
layer in `packages/verticopolis-engine/`), the conformance suite
(`conformance/`), the docs site and the bench. Every simulation rule, price,
cap and tuning value lives here. [README.md](README.md) has the layout and
the checks; [ARCHITECTURE.md](ARCHITECTURE.md) has the design in diagrams;
[CONTRIBUTING.md](CONTRIBUTING.md) has pull requests, change notes, releases
and the registries. Read all three. How a Classic tower matches the 1994
game is [docs/parity.md](docs/parity.md).

## The BMAD setup

This repository carries its own BMAD install: the BMAD/BMGD skills in
`.agents/skills/` (`.claude/skills` links to it), the workflow engine in
`_bmad/`, this repository's overrides in `_bmad/custom/`, and its own
outputs in `_bmad-output/`: the project context
(`_bmad-output/project-context.md`), the backlog
(`_bmad-output/implementation-artifacts/backlog.md`), specs, plans, party
findings and party memory. Run the skills from this repository's root so
they read its config and write to its backlog. Run the review skill on every
non-trivial change here: `/gds-code-review` for simulation, binding, frame
and save work, `/bmad-code-review` for CI, packaging and tooling. The
hot-path checklist (`docs/performance/hot-path-review.md`) stays one page in
[Verticopolis/web](https://github.com/Verticopolis/web) and governs this
repository too; the review overrides read it from there. Keeping the install
in step with the other repositories is in CONTRIBUTING.md, "The BMAD setup".

## Non-negotiables

- **The engine is the one simulation.** A frontend never adds, copies or
  patches a rule; a rule change lands here, with a regenerated conformance
  lock in the same PR. Ask for only what any frontend could use.
- **Change notes, never a version edit.** A PR that changes what the crate
  or the npm package ships adds `.changes/<branch-slug>.md` with its `crate:`
  tier (major, minor, patch or none). Any change to Rust code under
  `src/` other than its test files takes patch or higher; a
  test-only change may use `crate: none`. Only the release PR raises the
  version and writes `CHANGELOG.md`.
- **Quality gates before pushing:** `cargo fmt --all -- --check`,
  `cargo clippy --locked --all-targets -- -D warnings`, `cargo test --locked`,
  the conformance referee, and for the TypeScript `npm run typecheck`,
  `npm run lint`, `npm run format:check` and `npm test`. Prettier owns the
  TypeScript layout; run `npm run format` before committing.
- **No guard tests.** Never add a test that reads source, CSS, workflow
  or configuration files. Check workflows with actionlint and real CI runs.
- **No host access in the simulation** (`tests/independence.rs`).
- **A 500-line ceiling per source file.** Test files and generated files
  are exempt. Nothing checks it yet, and some files are already over: don't
  make one longer, and split it when you change it.
- **American English everywhere. No em-dashes in prose** (docs, comments,
  commit and PR text); use commas, colons, parentheses or separate
  sentences. Skip the "X, not Y" emphatic pattern and marketing vocabulary.
- **An independent GO before every merge.** The review comes from a
  separate thread or person, never the PR's author, on the PR's head
  commit, with every review thread resolved.
- **Merge commits only** to `main`, never a squash, and never force-push
  `main`. Force-pushing your own PR branch is fine. Keep a branch's own
  history readable before it merges.
- **PR descriptions follow `.github/PULL_REQUEST_TEMPLATE.md`:** keep every
  `## ` heading and fill each one.
