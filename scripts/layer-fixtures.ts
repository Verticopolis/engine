/**
 * Write the TypeScript layer's JSON fixtures from the engine itself:
 * packages/verticopolis-engine/src/fixtures/catalog.classic.json and
 * catalog.modern.json (the binding's `catalog(mode)`) and viewLayout.json (a
 * fresh engine's `viewLayout()`), pretty-printed. The package ships them under
 * `fixtures/`, and the test helpers under `testing/` read them.
 *
 *   node scripts/layer-fixtures.ts           write them
 *   node scripts/layer-fixtures.ts --check   fail if any differs from the engine
 *
 * Needs the tooling build (npm run wasm:build). Engine CI runs --check after
 * the build, so a catalog or layout change in Rust lands with its fixtures.
 */
import { readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const dir = resolve(root, "packages/verticopolis-engine/src/fixtures");
const check = process.argv.includes("--check");

const engine = createRequire(import.meta.url)(resolve(root, "engine-rs/pkg/verticopolis_engine.js")) as {
  catalog(mode: string): string;
  Engine: { newGame(seed: number, mode: string): { viewLayout(): string; free(): void } };
};

const pretty = (json: string): string => `${JSON.stringify(JSON.parse(json), null, 2)}\n`;
const game = engine.Engine.newGame(1, "classic");
const files: Record<string, string> = {
  "catalog.classic.json": pretty(engine.catalog("classic")),
  "catalog.modern.json": pretty(engine.catalog("modern")),
  "viewLayout.json": pretty(game.viewLayout()),
};
game.free();

let stale = 0;
for (const [name, text] of Object.entries(files)) {
  const file = resolve(dir, name);
  if (!check) {
    writeFileSync(file, text);
    continue;
  }
  let current = "";
  try {
    current = readFileSync(file, "utf8");
  } catch {
    // A missing fixture is stale too.
  }
  if (current !== text) {
    console.error(
      `::error file=packages/verticopolis-engine/src/fixtures/${name}::${name} differs from the engine's answer; run node scripts/layer-fixtures.ts and commit it.`,
    );
    stale++;
  }
}
if (stale > 0) process.exit(1);
console.log(check ? "layer fixtures: match the engine" : "layer fixtures: written");
