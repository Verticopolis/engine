/**
 * Runs the conformance referee built for WebAssembly (wasm32-wasip1) under
 * Node's WASI, so the suite also holds for the engine compiled the way a
 * browser runs it: 32-bit pointers, no threads, wasm's float rules. The
 * referee sees the conformance directory as `/conformance` and nothing else.
 *
 *   cargo build --locked --release --features conformance --bin conformance --target wasm32-wasip1
 *   node --no-turbo-fast-api-calls scripts/conformance-wasi.ts \
 *     engine-rs/target/wasm32-wasip1/release/conformance.wasm conformance
 *
 * Run Node with `--no-turbo-fast-api-calls`: without it, Node 26 can crash
 * in a garbage collection that starts inside WASI's fd_write fast call.
 * Exits with the referee's own status.
 */
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { WASI } from "node:wasi";

const [wasmPath, root] = process.argv.slice(2);
if (!wasmPath || !root) {
  console.error("usage: conformance-wasi.ts <conformance.wasm> <conformance dir>");
  process.exit(2);
}
const wasi = new WASI({
  version: "preview1",
  args: ["conformance", "/conformance"],
  env: {},
  preopens: { "/conformance": resolve(root) },
  returnOnExit: true,
});
const module = await WebAssembly.compile(await readFile(wasmPath));
const instance = await WebAssembly.instantiate(module, wasi.getImportObject());
process.exitCode = wasi.start(instance);
