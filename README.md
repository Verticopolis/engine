# Verticopolis engine

The Rust simulation engine behind [Verticopolis](https://verticopolis.com): the
`verticopolis-engine` crate and its WASM package, the conformance lock and its
fixtures, the engine docs site and the engine bench.

- The crate: the root `Cargo.toml` is a workspace whose member is
  `engine-rs/` (`verticopolis-engine` on crates.io).
- The npm package: the root `package.json` defines `@verticopolis/engine`,
  the WASM build for Node and the browser.

## Preview

This repository holds history extracted from `maniator/verticopolis` at commit
`658661a5c` on 2026-11-19. Every past path of every file was kept, so
`git log` and `git blame` reach back to each line's first commit, with the
original authors, dates and messages.

Development still happens in `maniator/verticopolis`. This repository is
extracted again when the split happens after the 4.x line ships, which may
rewrite this history, so please don't open pull requests or build on it yet.

- `#123` references in commit messages were rewritten to
  `Verticopolis/web#123`, the name `maniator/verticopolis` takes when it moves
  into the org.
- `history/commit-map.txt` maps each commit here to the commit it came from.
- The workflows came over with their history. `engine-rs.yml` runs the Rust
  checks here now; the others start only by hand until the cut.

## License

MIT, see [LICENSE](LICENSE).
