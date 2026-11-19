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

The source is in
[`engine-rs/`](https://github.com/Verticopolis/engine/tree/main/engine-rs)
of the [Verticopolis engine repository](https://github.com/Verticopolis/engine).

## License

MIT. See [`LICENSE`](./LICENSE). The licenses of the Rust crates compiled
into the WebAssembly, and of fflate, whose deflate the engine ports, are in
`web/THIRD-PARTY.json`.
