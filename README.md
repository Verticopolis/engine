# Verticopolis engine

The simulation engine of [Verticopolis](https://verticopolis.com), a tower
building game inspired by SimTower (1994). The engine is the whole
simulation: every rule, price, build cap, command, save format and gameplay
event, in Rust. The web game runs it as WebAssembly, and any other frontend
can link it as a Rust library or load the same WebAssembly package.

Frontends may differ. Simulations may not. The conformance suite in
`conformance/` is the referee: a host that replays every scenario and matches
every pinned checkpoint hash is running the real simulation.

## The Verticopolis repositories

| Repository | What it holds |
| --- | --- |
| **[Verticopolis/engine](https://github.com/Verticopolis/engine)** (this one) | The Rust engine (the crate at the root), its WebAssembly package, the conformance lock, the engine docs site and the bench |
| [Verticopolis/components](https://github.com/Verticopolis/components) | The TypeScript pieces a frontend draws and plays with: the renderer and Excalibur scene, the Win95 windows and menus, the procedural sprites and audio. They read a render model the game fills, and know nothing of the engine |
| [Verticopolis/web](https://github.com/Verticopolis/web) | The web game at [verticopolis.com](https://verticopolis.com) and its player's manual |

The game depends on the engine and the components, and joins the two
itself.

## Use it

### Rust

```sh
cargo add verticopolis-engine
```

The crate needs the Rust release named by `rust-version` in
`Cargo.toml` and nothing else: no Node.js, no build script, no
system library.

### JavaScript

```sh
npm install @verticopolis/engine
```

`node/` is the build for Node and `web/` the build for the browser. The
[JavaScript guide](https://docs.verticopolis.com/javascript/) shows how to
load it and drive a tower.

Both are on the registries:
[`verticopolis-engine`](https://crates.io/crates/verticopolis-engine) on
crates.io and [`@verticopolis/engine`](https://www.npmjs.com/package/@verticopolis/engine)
on npm. The first version, 1.57.1, was published by hand; every later
version comes from this repository's release workflow. Each [`v<version>` release](https://github.com/Verticopolis/engine/releases)
(`engine-v<version>` up to 1.57.2)
also carries the npm tarball with its SHA-256 checksum. The package holds the
tooling build (`@verticopolis/engine/tooling`) and the conformance suite
(`@verticopolis/engine/conformance/`) beside the player build; a game that
does not import them does not bundle them.

## Documentation

[ARCHITECTURE.md](ARCHITECTURE.md) is a tour of the engine in diagrams: what
this repository builds, the boundary with a host, a step of time, Classic vs
Modern, undo, what a host reads, saves and the conformance lock.

The guides live on [docs.verticopolis.com](https://docs.verticopolis.com),
built from `docs/engine-site/`:

- [Rust crate](https://docs.verticopolis.com/crate/): a quick start, how the
  engine works, features, saves and versioning.
- [Host API guide](https://docs.verticopolis.com/api/): every call a frontend
  makes. [`API.md`](API.md) is the same list in the repo.
- [Rust API reference](https://docs.verticopolis.com/rust/verticopolis_engine/).
- [Conformance suite](https://docs.verticopolis.com/conformance/).
- [Architecture](https://docs.verticopolis.com/architecture/) and the
  [module map](https://docs.verticopolis.com/engine/), with the diagrams drawn.
- [Classic parity checklist](https://docs.verticopolis.com/parity/) and the
  [Classic canon records](https://docs.verticopolis.com/canon/).
- [Changelog](CHANGELOG.md).

## Layout

| Path | What it is |
| --- | --- |
| `Cargo.toml`, `src/`, `tests/` | The `verticopolis-engine` crate. [`docs/engine.md`](docs/engine.md) maps its modules |
| `consumer/` | A small frontend-free host, built against the public API only |
| `conformance/` | The scenarios, the pinned hashes and the `.TDT` cases the referee checks |
| `engine-bench/` | The step-cost bench |
| `docs/engine-site/` | The docs site |
| `docs/tdt/` | The 1994 `.TDT` save format: what its values mean and a map of every byte |
| `docs/parity.md` | How a Classic tower matches the 1994 game, one line per mechanic |
| `docs/canon/` | Dated audits of Classic rules and the runs of the original that settle them |
| `docs/performance/` | Measurements of the engine package |
| `packages/verticopolis-engine/` | The `@verticopolis/engine` npm package: its `package.json`, README, TypeScript layer and browser types |
| `Cargo.lock` | The lock the crate builds and packages with |
| `package.json` | The repository's build and check tooling (private, never published) |
| `history/commit-map.txt` | Each commit here and the `maniator/verticopolis` commit it came from |

## Build and test

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo run --release --features conformance --bin conformance   # the referee
(cd consumer && cargo test --locked)
```

The TypeScript around the engine (the packer, the event rules, the
TypeScript layer, the build scripts, the docs site's scripts) has the game's
checks:

```sh
npm run typecheck
npm run lint            # ESLint, the game's rules
npm run format:check    # Prettier; `npm run format` rewrites the files
npm test
```

CI also runs the referee compiled to WebAssembly, under Node's WASI
(`scripts/conformance-wasi.mts` has the two commands), so the checkpoints hold
for the engine built the way a browser runs it.

`docs/engine.md` lists the other tools: the checkpoint dump, the `.TDT`
replay and coverage.

## Contributing

The engine came out of the game's repository, and its rules came with it:
American English, no em-dashes in prose, a 500-line ceiling per source file,
no host access in the simulation (`tests/independence.rs`), and a
regenerated conformance lock in the same PR as any change to how the game
plays. [CONTRIBUTING.md](CONTRIBUTING.md) covers pull requests, releases
and the registries.

Pull requests and issues for the engine open here. The game takes a change
when a pull request in Verticopolis/web raises its `@verticopolis/engine`
pin.

## History

This repository was extracted from `maniator/verticopolis`, now
Verticopolis/web, with every past path of every file kept, so `git log` and
`git blame` reach back to each line's first commit with the original authors,
dates and messages. `#123` in an old commit message was rewritten to
`Verticopolis/web#123`, the issue or pull request it meant.

## License

MIT, see [LICENSE](LICENSE).
