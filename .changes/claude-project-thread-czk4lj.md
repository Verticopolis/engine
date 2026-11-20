---
crate: patch
---
- the repository's layout changed: the crate sits at the repository root and
  the npm package's manifest is `packages/verticopolis-engine/package.json`.
  What the crate and the npm package ship is unchanged, apart from the
  generated `events/` declarations' header comments naming the new paths.
- `engineSourceHash` (`@verticopolis/engine/host/packageHash`) follows a
  crate's `include` list at its top level, so hashing a crate that sits at a
  repository root reads only the folders and files the crate ships. It throws
  on an `include` entry with a glob character it cannot follow.
