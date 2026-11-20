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
 * Exits with the fuzzer's own status (0 ran, 1 the engine refused the
 * scenario, 2 bad input), or 4 when the build traps (a Rust panic aborts with
 * an unreachable, which shows as a WebAssembly RuntimeError). Run Node with
 * `--no-turbo-fast-api-calls`, as for `conformance-wasi.mts`.
 */
import { mkdirSync, readFileSync } from "node:fs";
import { basename, dirname, resolve } from "node:path";
import { WASI } from "node:wasi";

const [wasmPath, scenario, out, suite = "conformance"] = process.argv.slice(2);
if (!wasmPath || !scenario || !out) {
  console.error("usage: fuzz-wasi.mts <fuzz.wasm> <scenario.json> <out-dir> [conformance-dir]");
  process.exit(2);
}
mkdirSync(out, { recursive: true });
const wasi = new WASI({
  version: "preview1",
  args: ["fuzz", "--replay", `/in/${basename(scenario)}`, "/out", "/conformance"],
  env: {},
  preopens: { "/in": resolve(dirname(scenario)), "/out": resolve(out), "/conformance": resolve(suite) },
  returnOnExit: true,
});
try {
  const module = await WebAssembly.compile(readFileSync(wasmPath));
  const instance = await WebAssembly.instantiate(module, wasi.getImportObject() as WebAssembly.Imports);
  process.exitCode = wasi.start(instance);
} catch (e) {
  if (e instanceof WebAssembly.RuntimeError) {
    console.error(`${scenario}: the WebAssembly build trapped: ${e.message}`);
    process.exitCode = 4;
  } else throw e;
}
