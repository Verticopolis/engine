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
| **[Verticopolis/engine](https://github.com/Verticopolis/engine)** (this one) | The Rust engine (`engine-rs/`), its WebAssembly package, the conformance lock, the engine docs site and the bench |
| [Verticopolis/components](https://github.com/Verticopolis/components) | The TypeScript pieces a frontend builds on: the renderer and Excalibur scene, the Win95 windows and menus, audio, and the engine binding and host loader |
| [Verticopolis/web](https://github.com/Verticopolis/web) | The web game at [verticopolis.com](https://verticopolis.com) and its player's manual |

The game depends on the engine and the components, and the components load
the engine's WebAssembly package at run time.

## Use it

### Rust

```sh
cargo add verticopolis-engine
```

The crate needs the Rust release named by `rust-version` in
`engine-rs/Cargo.toml` and nothing else: no Node.js, no build script, no
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
also carries the npm tarball, the tooling build and the conformance suite as
tarballs, each with its SHA-256 checksum.

## Documentation

The guides live on [docs.verticopolis.com](https://docs.verticopolis.com),
built from `docs/engine-site/`:

- [Rust crate](https://docs.verticopolis.com/crate/): a quick start, how the
  engine works, features, saves and versioning.
- [Host API guide](https://docs.verticopolis.com/api/): every call a frontend
  makes. [`engine-rs/API.md`](engine-rs/API.md) is the same list in the repo.
- [Rust API reference](https://docs.verticopolis.com/rust/verticopolis_engine/).
- [Conformance suite](https://docs.verticopolis.com/conformance/).
- [Changelog](engine-rs/CHANGELOG.md).

## Layout

| Path | What it is |
| --- | --- |
| `engine-rs/` | The `verticopolis-engine` crate. [`engine-rs/README.md`](engine-rs/README.md) maps its modules |
| `engine-rs/consumer/` | A small frontend-free host, built against the public API only |
| `conformance/` | The scenarios, the pinned hashes and the `.TDT` cases the referee checks |
| `engine-bench/` | The step-cost bench |
| `docs/engine-site/` | The docs site |
| `packages/verticopolis-engine/` | The npm package's README and browser types |
| `Cargo.toml`, `Cargo.lock` | The workspace root (member `engine-rs`) |
| `package.json` | The `@verticopolis/engine` package definition |
| `history/commit-map.txt` | Each commit here and the `maniator/verticopolis` commit it came from |

## Build and test

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo run --release --features conformance --bin conformance   # the referee
(cd engine-rs/consumer && cargo test --locked)
```

`engine-rs/README.md` lists the other tools: the checkpoint dump, the `.TDT`
replay and coverage.

## Contributing

The engine came out of the game's repository, and its rules came with it:
American English, no em-dashes in prose, a 500-line ceiling per source file,
no host access in the simulation (`engine-rs/tests/independence.rs`), and a
regenerated conformance lock in the same PR as any change to how the game
plays. [CONTRIBUTING.md](CONTRIBUTING.md) covers pull requests, releases
and the registries.

The engine lives here now. Pull requests open here once the game builds
from this repository's packages, which takes a few days. Until then, engine
changes are on hold everywhere, Verticopolis/web included. Issues are
welcome here now.

## History

This repository was extracted from `maniator/verticopolis`, now
Verticopolis/web, with every past path of every file kept, so `git log` and
`git blame` reach back to each line's first commit with the original authors,
dates and messages. `#123` in an old commit message was rewritten to
`Verticopolis/web#123`, the issue or pull request it meant.

## License

MIT, see [LICENSE](LICENSE).
