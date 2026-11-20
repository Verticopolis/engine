# Contributing to the Verticopolis engine

This repository holds the engine crate (`engine-rs/`), its WebAssembly
package, the conformance suite (`conformance/`), the docs site
(`docs/engine-site/`) and the bench (`engine-bench/`). The game that runs the
engine lives in [Verticopolis/web](https://github.com/Verticopolis/web); its
CONTRIBUTING.md covers the game. The README's "Build and test" section lists
the checks a pull request runs.

## Pull requests

The rules came with the engine from the game's repository: American English,
no em-dashes in prose, a 500-line ceiling per source file, no host access in
the simulation (`engine-rs/tests/independence.rs`), and a regenerated
conformance lock in the same PR as any change to how the game plays. Merge
commits only, never squash. Fill in every heading of the pull request
template.

A change to what the crate ships adds a change note under
[`.changes/`](.changes/README.md), named after its branch, with its crate
tier and its changelog entries for people who build on the engine:
**major** for a break in saves, the frame or view layout or the binding,
**minor** for a new API, **patch** for a fix, and `none` for a change that
ships nothing new (a Rust code change under `engine-rs/src/` takes patch or
higher; `none` fits its test files). Notes keep two open pull requests from
editing the same changelog lines. The **Change notes** check requires one
when a pull request changes what the crate or the npm package ships (the
paths are in `.changes/README.md`), and refuses a feature pull request that
edits the crate version or the changelog's version sections. **Cut release** picks the version from the
tiers. Only a release PR edits the version or `engine-rs/CHANGELOG.md`.

### What CI runs on a pull request

Every required job runs on every pull request, so its check always reports,
but the build, test and analysis jobs skip the work they do not need. Each of
them starts with a Scope step that reads the files the pull request changes
(the PR template and change-note checks have no Scope step and always run in
full):

- The Rust jobs (`engine-rs.yml`) skip their steps when no changed file is an
  engine input. `.github/actions/rust-scope` lists what is not: docs, change
  notes, the npm package's TypeScript, repository tooling and other
  workflows. A file on neither of its lists runs everything.
- The pack job (`engine-pack.yml`) and CodeQL (`codeql.yml`) skip theirs when
  every changed file is prose that nothing builds, tests or ships
  (`.github/actions/docs-only`). READMEs, changelogs and licenses ship with
  the package, so they still run the pack.

A skipped job finishes green in seconds. A push to `main` always runs
everything. A new step in one of these jobs needs the same
`if: steps.scope.outputs.run == 'true'` as its neighbors, or it runs with
nothing built.

## Releasing

A release is one merged pull request that raises the crate version.

1. **Cut the release PR.** Run **Cut release**
   (`.github/workflows/cut-release.yml`) from the Actions tab on `main`, as
   the game's repository does. It runs `scripts/cut-release.ts`, which takes
   the highest tier among the change notes in `.changes/`, raises
   `version` in `engine-rs/Cargo.toml`, the three lockfiles that name the
   crate (at the root, in `engine-rs/consumer/` and in `engine-bench/`) and
   `crateVersion` in `engine-rs/src/compat-record.json`, writes the notes'
   entries into a new `## <version> (<date>)` section of
   `engine-rs/CHANGELOG.md` (each led by its tier, the highest first),
   deletes the notes and moves the quoted version pins in
   `docs/engine-site/guides/`. It commits that to `release/<version>`
   and opens the "Release engine <version>" PR. Leave the version empty for
   the tier the notes ask for, or name a later one. It refuses while
   another release PR is open. The PR comes from the workflow token, which
   starts no checks, so start them with the run button on the PR, or close
   and reopen it. The workflow needs "Allow GitHub Actions to create and
   approve pull requests" (Settings, Actions, General). The engine's guides
   and the registry READMEs describe the engine as `main` has it, so a
   release that changes what a host sees also updates their prose in the
   release PR. To cut one by hand instead, run `node scripts/cut-release.ts`
   on a clean `release/<version>` branch off `main`.
2. **Merge it.** The push to `main` that changes the crate version starts
   **Release tags** (`.github/workflows/release-tag.yml`). It builds the
   engine, packs the npm tarball with `scripts/pack-engine.sh`, tags
   `v<version>` on the merge commit and publishes the GitHub release, named
   after the tag and marked Latest, with the changelog section. It refuses a version with no changelog
   section.
3. **Approve the publish.** The same run dispatches **Publish engine to
   registries** (`engine-publish.yml`), whose publish jobs wait on the
   `registry` environment. The owner approves it from the run's page, and the
   crate and the npm package go up (see "The registries").

If a run fails, fix the cause on `main` and run **Release tags** by hand from
the Actions tab with the version `engine-rs/Cargo.toml` holds. A tag or
release that already exists is left alone. To publish a version again after
a registry failure, run **Publish engine to registries** by hand with the
version instead.

`engine-pack.yml` also packs the tarballs on every pull request and every
push to `main`, as workflow artifacts, so a release never packs something no
run has packed before. It creates no release: **Release tags** creates every
release, so a tag pushed by hand gets none. Versions up to 1.57.2 were
tagged `engine-v<version>`; those tags and releases stay as they are.

## The registries

Each release publishes the engine to two registries, with the same version:

- the crate to crates.io as
  [`verticopolis-engine`](https://crates.io/crates/verticopolis-engine);
- the player build to npm as
  [`@verticopolis/engine`](https://www.npmjs.com/package/@verticopolis/engine)
  (the `verticopolis` npm organization).

**Release tags** hands a release to **Publish engine to registries** only
when the repository variable `ENGINE_REGISTRY_PUBLISH` is `on`, which it has
been since the first publish. Setting it to anything else ships GitHub
releases and leaves the registries alone.

The publish run checks the version, the tag and the tarball's checksum
without any credential. Its two publish jobs then wait on the `registry`
environment, whose only reviewer is the owner and which deploys only from
`main`, so nothing reaches a registry until the owner approves the run. Each
job gets a short-lived token through trusted publishing: crates.io and npm
trust this repository, the workflow file `engine-publish.yml` and the
`registry` environment, and exchange GitHub's OIDC token for a publish token.
No registry token is stored anywhere. The crate is published from the
release tag's `engine-rs/`. npm gets the tarball from the GitHub release, so
the bytes on npm are the bytes attached there, with an npm provenance
statement. A version a registry already holds is left alone there, so a
re-run is safe.

The npm package requires two-factor authentication and refuses tokens, so a
leaked token cannot publish it. The crate's `include` list in
`engine-rs/Cargo.toml` names what crates.io receives, and the `consumer` job
in `engine-rs.yml` packages the crate on every engine change and builds the
consumer crate against the packaged copy, so a file the library needs from
outside `engine-rs/` fails there. The conformance suite is not in the crate
yet ([#47](https://github.com/Verticopolis/engine/issues/47)); it ships in
the npm package as `@verticopolis/engine/conformance/`.

**The first version of each** went up by hand, because both registries set
up trusted publishing only for a package that already exists. The owner
published 1.57.1 on 2026-11-19 with `scripts/first-publish.ps1`, which checks
out the release tag in a temporary folder, runs the dry runs, asks before
each publish and prints the trusted-publisher settings to add afterward. A
new package published from this repository needs the same first step once.
1.57.1 carries no npm provenance statement, since it was not published from
the workflow; every later version does.

Versions released before the registries were published only to GitHub
Packages, as `@maniator/verticopolis-engine`, and stay there.
