import { readFileSync } from "node:fs";
import type { Catalog } from "../catalog/catalogTypes.ts";
import { readCatalog } from "../binding/catalog.ts";
import type { GameMode } from "../model/types.ts";

/**
 * The Rust engine's catalog for each mode, as its `catalog(mode)` export
 * answers it, for tests that price, size or lay out without an engine. The
 * catalog is the engine's published rule table (CONTRIBUTING.md, "What every
 * test must do": a pinned value comes from the catalog), so the fixtures
 * (`fixtures/catalog.<mode>.json`) are written from the engine on purpose, as
 * the catalog digests are: `npm run layer:fixtures` writes them, and engine CI
 * regenerates them and fails on a diff. Node only.
 * Each read passes the binding's own shape check (`readCatalog`).
 */

const FIXTURES = new URL("../fixtures/", import.meta.url);
const read = new Map<GameMode, Catalog>();

/** The fixture file of `mode`'s catalog. */
export const catalogFixturePath = (mode: GameMode): URL => new URL(`catalog.${mode}.json`, FIXTURES);

/** `mode`'s catalog, read once. */
export function catalogFor(mode: GameMode): Catalog {
  let c = read.get(mode);
  if (!c) {
    const text = JSON.stringify(JSON.parse(readFileSync(catalogFixturePath(mode), "utf8")));
    c = readCatalog({ catalog: () => text }, mode);
    read.set(mode, c);
  }
  return c;
}
