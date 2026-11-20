# @verticopolis/engine

The [Verticopolis](https://verticopolis.com) simulation engine, compiled to
WebAssembly: every rule, price, cap, command, save format and gameplay event
of the tower game, with builds for Node (`node/`) and the browser (`web/`).
Your code draws the tower and handles input; the engine runs it.

## Install

```sh
npm install @verticopolis/engine
```

```js
const { Engine } = require("@verticopolis/engine");

const engine = Engine.newGame(42, "classic");
for (let minute = 0; minute < 60; minute++) engine.tick(1); // one game hour
console.log(engine.money());
engine.free();
```

## Test builds and the conformance suite

The package also holds `tooling/`, the same engine with the test and tooling
calls (`require("@verticopolis/engine/tooling")`, for test suites and
harnesses only, never shipped to players), and `conformance/`, the
conformance suite's files. A game that imports neither bundles nothing from
them. See
[Test builds and the conformance suite](https://docs.verticopolis.com/javascript/#test-builds-and-the-conformance-suite).

## The TypeScript layer

The package also carries the TypeScript a frontend builds on, as ES modules
with declarations, one subpath per module: `model/*`, `catalog/*`,
`binding/*`, `port/*` (with `port/frameReader`, which reads a frame from the
engine alone), `host/*`, `testing/*` (`mockPort(vi, answers)` and the catalog
fixtures) and `events/*`. None of them imports anything but the engine. See
[The TypeScript layer](https://docs.verticopolis.com/javascript/#the-typescript-layer).

## Documentation

The full guide lives on the engine docs site,
[docs.verticopolis.com](https://docs.verticopolis.com):

- [JavaScript package](https://docs.verticopolis.com/javascript/): install,
  the Node and browser quick starts, every call, saves and versioning.
- [Host API guide](https://docs.verticopolis.com/api/) and the
  [Rust crate](https://docs.verticopolis.com/crate/)
  ([`verticopolis-engine`](https://crates.io/crates/verticopolis-engine) on
  crates.io), for a host written in Rust.
- [Conformance suite](https://docs.verticopolis.com/conformance/): how a host
  proves it runs the real simulation.
- [Changelog](./CHANGELOG.md), one section per engine version.

The Rust source is in
[`src/`](https://github.com/Verticopolis/engine/tree/main/src)
of the [Verticopolis engine repository](https://github.com/Verticopolis/engine).

## License

MIT. See [`LICENSE`](./LICENSE). The licenses of the Rust crates compiled
into the WebAssembly, and of fflate, whose deflate the engine ports, are in
`web/THIRD-PARTY.json`.
