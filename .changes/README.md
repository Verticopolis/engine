# Change notes

A pull request does not edit `CHANGELOG.md` or a version. It adds
one note here, named after its branch (`claude/project-thread-abc123` becomes
`claude-project-thread-abc123.md`), so two open pull requests never touch the
same file:

```
---
crate: minor    # major | minor | patch | none
---
- the binding's `towerSummary` reports the lobby's floor.
- `Tower::lobby_floor` returns it to Rust hosts.
```

- `crate` is the tier from CONTRIBUTING.md: **major** for a break in saves,
  the frame or view layout or the binding, **minor** for a new API,
  **patch** for a fix. `none` says the pull request changes nothing the crate
  or the npm package ships; such a note has no entries.
- Each entry is one `- ` bullet, written for people who build on the engine
  and starting where the text after `- **<tier>**: ` would, with any further
  lines indented two spaces. The release leads each entry with the note's
  tier, so leave the tier out of the entry.
- A pull request that changes what the crate or the npm package ships needs
  a note: `src/`, `Cargo.toml`, `conformance/`,
  `packages/verticopolis-engine/` (its `package.json` is the package's
  manifest) or the packaging scripts (`scripts/wasm-build.ts`,
  `scripts/build-engine-wasm.sh`, `scripts/pack-engine.sh`,
  `scripts/engine-package-files.sh`). A change to Rust code under
  `src/` other than its test files needs a tier of patch or higher;
  `crate: none` fits only test files or a change that ships nothing new.
  Docs, the bench and CI changes alone need no note, and neither do
  Dependabot's branches, which cannot write one.
- Notes sit directly in `.changes/`, never in a subfolder.
- A feature pull request leaves the crate version and the version sections
  of `CHANGELOG.md` alone; the release PR writes them.
- A note that is already on `main` belongs to the pull request that merged
  it. A follow-up pull request from the same branch name adds
  `<branch-slug>-2.md` instead of editing it; only the release PR removes a
  pending note.

**Cut release** (`.github/workflows/cut-release.yml`, CONTRIBUTING.md
"Releasing") raises the crate version by the highest pending tier, writes
every note's entries into the new version's section of
`CHANGELOG.md` (the highest tier first), and deletes the notes.
The **Change notes** check (`.github/workflows/change-notes.yml`) runs
`node scripts/change-notes-check.ts --base origin/main` on every pull
request; the rules are in `tools/changeNotes.ts`.
