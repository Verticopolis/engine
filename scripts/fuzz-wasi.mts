/**
 * Replays one scenario on the fuzzer built for WebAssembly (wasm32-wasip1)
 * under Node's WASI, the way `fuzz --replay` runs it natively, so the records
 * of the two builds can be compared (`scripts/fuzz-compare.mts`). The guest
 * sees three directories and nothing else: the scenario's own as `/in`, the
 * output as `/out` and the conformance suite as `/conformance`.
 *
 *   cargo build --locked --release --features conformance --bin fuzz --target wasm32-wasip1
 *   node --no-turbo-fast-api-calls scripts/fuzz-wasi.mts \
 *     engine-rs/target/wasm32-wasip1/release/fuzz.wasm <scenario.json> <out-dir> [conformance-dir]
 *
 * Exits 0 when the scenario ran, 6 when the engine refused it (the fuzzer's
 * own status 1), 4 when the build trapped (a Rust panic aborts with an
 * unreachable, which shows as a WebAssembly RuntimeError), and 5 when the run
 * failed for any other reason: a missing or unreadable .wasm, a folder it
 * cannot create, bad input (the fuzzer's 2). Node itself exits 1 when this
 * script crashes, so no finding uses 1 or 2, and a broken harness can never
 * read as the WebAssembly build refusing a scenario. Run Node with
 * `--no-turbo-fast-api-calls`, as for `conformance-wasi.mts`.
 */
import { mkdirSync, readFileSync } from "node:fs";
import { basename, dirname, resolve } from "node:path";
import { WASI } from "node:wasi";

const [wasmPath, scenario, out, suite = "conformance"] = process.argv.slice(2);
if (!wasmPath || !scenario || !out) {
  console.error("usage: fuzz-wasi.mts <fuzz.wasm> <scenario.json> <out-dir> [conformance-dir]");
  process.exit(5);
}
/** The run failed on its own: never a finding. */
const HARNESS = 5;
/** The engine refused the scenario: the fuzzer's status 1, moved off 1. */
const REFUSED = 6;
let instance: WebAssembly.Instance;
let wasi: WASI;
try {
  mkdirSync(out, { recursive: true });
  wasi = new WASI({
    version: "preview1",
    args: ["fuzz", "--replay", `/in/${basename(scenario)}`, "/out", "/conformance"],
    env: {},
    preopens: { "/in": resolve(dirname(scenario)), "/out": resolve(out), "/conformance": resolve(suite) },
    returnOnExit: true,
  });
  const module = await WebAssembly.compile(readFileSync(wasmPath));
  instance = await WebAssembly.instantiate(module, wasi.getImportObject() as WebAssembly.Imports);
} catch (e) {
  console.error(`${scenario}: cannot start the WebAssembly fuzzer: ${(e as Error).message}`);
  process.exit(HARNESS);
}
try {
  const status = wasi.start(instance);
  process.exitCode = status === 0 ? 0 : status === 1 ? REFUSED : HARNESS;
} catch (e) {
  if (e instanceof WebAssembly.RuntimeError) {
    console.error(`${scenario}: the WebAssembly build trapped: ${e.message}`);
    process.exitCode = 4;
  } else {
    console.error(`${scenario}: the WASI run failed: ${(e as Error).message}`);
    process.exitCode = HARNESS;
  }
}
