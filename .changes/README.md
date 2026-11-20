# Change notes

A pull request does not edit `engine-rs/CHANGELOG.md` or a version. It adds
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
- A pull request that changes `engine-rs/src/`, `engine-rs/Cargo.toml` or
  `packages/verticopolis-engine/` needs a note, a `crate: none` one included.
  Docs, tests, the bench and CI changes alone need none, and neither do
  Dependabot's branches, which cannot write one.
- A note that is already on `main` belongs to the pull request that merged
  it. A follow-up pull request from the same branch name adds
  `<branch-slug>-2.md` instead of editing it; only the release PR removes a
  pending note.

**Cut release** (`.github/workflows/cut-release.yml`, CONTRIBUTING.md
"Releasing") raises the crate version by the highest pending tier, writes
every note's entries into the new version's section of
`engine-rs/CHANGELOG.md` (the highest tier first), and deletes the notes.
The **Change notes** check (`.github/workflows/change-notes.yml`) runs
`node scripts/change-notes-check.ts --base origin/main` on every pull
request; the rules are in `src/build/changeNotes.ts`.
