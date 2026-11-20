import { describe, it, expect } from "vitest";
import { readFileSync, readdirSync } from "node:fs";
import { resolve } from "node:path";
import {
  CONFORMANCE_DIR,
  firstDivergence,
  loadScenario,
  runScenario,
  type Checkpoint,
  type EngineStart,
} from "./scenario.ts";
import { hasWasmPackage, startWasmEngine, startWasmHandle, wasm, wasmEngine } from "./wasmEngine.ts";
import { CATALOG_VERSION, readCatalog } from "../src/binding/catalog.ts";
import type { PortRecord } from "../src/port/enginePort.ts";
import { catalogLockView } from "./catalogLockView.ts";
import { canonicalJson, digest } from "./canonical.ts";

/**
 * The conformance suite through the published glue: every scenario under
 * `conformance/scenarios` runs on the tooling build through its wasm-bindgen
 * binding in Node and must reproduce the lock's checkpoint hashes
 * (`expected.json`) and catalog digests (`catalog-digests.json`) exactly.
 * The native referee and its wasm32-wasip1 build write and check the same
 * lock; this suite holds the JavaScript binding a frontend loads to it. It
 * never writes the lock. Without a built package (`npm run wasm:build`) the
 * suite skips.
 */

const SCENARIO_DIR = resolve(CONFORMANCE_DIR, "scenarios");
const LOCK = resolve(CONFORMANCE_DIR, "expected.json");
const CATALOG_LOCK = resolve(CONFORMANCE_DIR, "catalog-digests.json");
const CATALOG_MODES = ["classic", "modern"] as const;

type Lock = { scenarios: Record<string, Checkpoint[]> };
type CatalogLock = { catalogs: Record<string, string> };

const lock = JSON.parse(readFileSync(LOCK, "utf8")) as Lock;
const catalogLock = JSON.parse(readFileSync(CATALOG_LOCK, "utf8")) as CatalogLock;
const files = readdirSync(SCENARIO_DIR)
  .filter((f) => f.endsWith(".json"))
  .sort();
/** Whether a port record says its command was done. */
const done = (rec: PortRecord): boolean => rec[0] === 1;
/** A scenario's pinned checkpoints; throws for one the lock lacks. */
function pinned(id: string): Checkpoint[] {
  const want = lock.scenarios[id];
  if (!want) throw new Error(`${id} is not in the lock; regenerate the lock with the native referee`);
  return want;
}

describe.skipIf(!hasWasmPackage())("engine conformance through the WASM binding", () => {
  it("has a scenario for every lock entry and a unique id per file", () => {
    const ids = files.map((f) => loadScenario(resolve(SCENARIO_DIR, f)).id);
    expect(ids).toEqual(files.map((f) => f.replace(/\.json$/, "")));
    expect(Object.keys(lock.scenarios).sort()).toEqual([...ids].sort());
  });

  for (const file of files) {
    const scenario = loadScenario(resolve(SCENARIO_DIR, file));
    it(`${scenario.id} matches its pinned checkpoints`, () => {
      const got = runScenario(scenario, startWasmEngine);
      const want = pinned(scenario.id);
      // Report the first divergent checkpoint, the useful fact for a port.
      expect(firstDivergence(got, want)).toBeNull();
    }, 60_000);
  }

  it("runs a new game and a loaded save the same way twice in one process", () => {
    for (const id of ["starter-classic", "fixture-split-tower"]) {
      const s = loadScenario(resolve(SCENARIO_DIR, `${id}.json`));
      expect(runScenario(s, startWasmEngine)).toEqual(runScenario(s, startWasmEngine));
    }
  }, 60_000);

  // The emergency dialog answers through the port's `answerChoice`. The runner
  // answers through `resolveChoice`, so the two scenarios that answer a choice
  // run again with the answer given through the port, and must land on the
  // same checkpoints.
  for (const id of ["fire-rescue-paid-modern", "ransom-paid-modern"]) {
    it(`${id} matches its pinned checkpoints when answered through answerChoice`, () => {
      const scenario = loadScenario(resolve(SCENARIO_DIR, `${id}.json`));
      const answers = scenario.commands.filter((c) => c.op === "resolveChoice").length;
      expect(answers, `${id} no longer answers a choice`).toBeGreaterThan(0);
      expect(
        scenario.commands.some((c) => c.op === "reload"),
        `${id} reloads, which would drop the override`,
      ).toBe(false);
      let answered = 0;
      const throughPort: EngineStart = (start) => {
        const handle = startWasmHandle(start);
        const resolveChoice = (accept: boolean): void => {
          answered++;
          expect(done(handle.answerChoice(accept)), `answerChoice(${accept}): ${handle.lastReason()}`).toBe(true);
        };
        return { ...wasmEngine(handle), resolveChoice };
      };
      const want = pinned(id);
      expect(firstDivergence(runScenario(scenario, throughPort), want)).toBeNull();
      expect(answered, `${id}: every answer went through answerChoice`).toBe(answers);
    }, 60_000);
  }

  // The catalog the binding hands a frontend is the one the lock pins, and its
  // text is the canonical form the lock hashes. The lock view leaves out the
  // wedding hall's footprint, so it is pinned here.
  for (const mode of CATALOG_MODES) {
    it(`catalog(${mode}) is canonical and matches the lock`, () => {
      const { catalog } = wasm();
      if (typeof catalog !== "function") throw new Error("the tooling build has no catalog(mode)");
      const text = catalog(mode);
      const got = readCatalog(wasm(), mode);
      expect(canonicalJson(catalogLockView(JSON.parse(text)))).toBe(canonicalJson(catalogLockView(got)));
      const hash = digest(catalogLockView(got));
      expect(hash).toBe(catalogLock.catalogs[mode]);
      const hall = got.facilities.find((f) => f.key === "weddingHall");
      expect(hall).toMatchObject({ width: 28, floors: 5 });
    });
  }

  it("refuses a catalog for an unknown mode, or of a shape version it does not read", () => {
    expect(() => readCatalog(wasm(), "arcade" as "classic")).toThrow(/classic or modern/);
    expect(() => readCatalog(wasm(), "classic", CATALOG_VERSION + 1)).toThrow(/expected version/);
  });

  // Each refusal must surface as the binding's own error with its message; a
  // Rust panic would trap (a RuntimeError) and poison the one module instance
  // for every scenario after it.
  it("refuses a scenario start it cannot honor with an error that leaves the module usable", () => {
    expect(() => startWasmEngine({ newGame: { seed: 1, mode: "arcade" as "classic" } })).toThrow(/classic or modern/);
    expect(() => startWasmEngine({ fixture: "conformance/README.md" })).toThrow(/not a VCTOWER1 file/);
    expect(() =>
      startWasmEngine({
        fixture: "conformance/fixtures/sixseven-december.vctower",
        mode: "arcade" as "classic",
      }),
    ).toThrow(/classic or modern/);
    // The module still answers after the refusals: a fresh game founds and
    // hashes the same as the lock's starter.
    const want = pinned("starter-classic")[0];
    const fresh = startWasmEngine(loadScenario(resolve(SCENARIO_DIR, "starter-classic.json")).start);
    try {
      expect(fresh.stateDigest()).toBe(want.state);
    } finally {
      fresh.free?.();
    }
  });
});
