/**
 * Where the glue suites find what they drive and replay: the tooling build's
 * Node glue (`wasm/pkg/`, written by `npm run wasm:build`), the conformance
 * data at the repository root, and the TypeScript layer's JSON fixtures.
 */
import { resolve } from "node:path";

/** The repository root. */
const REPO = resolve(import.meta.dirname, "../../..");
/** The tooling build for Node: the glue, the module and its CommonJS marker. */
export const TOOLING_NODE_DIR = resolve(REPO, "wasm/pkg");
/** The conformance suite: scenarios, the lock, fixtures and the fuzz corpus. */
export const CONFORMANCE_ROOT = resolve(REPO, "conformance");
/** The layer's fixtures written from the engine (the catalogs, the view layout). */
export const LAYER_FIXTURES = resolve(REPO, "packages/verticopolis-engine/src/fixtures");

/** A file a scenario or a fuzz entry names, relative to the repository root
 *  (their data names fixtures as `conformance/...`). */
export function conformanceFile(path: string): string {
  return resolve(REPO, path);
}
