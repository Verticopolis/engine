---
crate: minor
---
- the npm package carries the engine's TypeScript layer, moved here with its
  history from the game and the components (#89): `model/*`, `catalog/*`,
  `binding/*`, `port/*`, `host/*` and `testing/*` as compiled ES modules with
  declarations, the JSON fixtures under `fixtures/`, and
  `events/gameplayCatalog` and `events/gameplayEventRules` compiled beside the
  generated `.ts`. The package is now `"type": "module"` with an exports map
  (the `node/` and `tooling/node/` builds stay CommonJS). The exports map
  covers every path the game and the components import; `CHANGELOG.md` and
  the `.ts` and `.d.ts` files under `events/` are read by file path, not
  imported. `port/frameReader` (`FrameReader`) reads a frame from the port
  with nothing but the engine; `testing/mockPort` takes the test runner's spy
  factory (`mockPort(vi, answers)`); `host/loadEngineModule` always checks the
  expected package identity, and `loadEngineUnchecked` is the explicit
  development path. No rule, save, layout or catalog changes.
