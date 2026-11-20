/**
 * Run the fuzzer's replay mode (`fuzz --replay <scenario> <out-dir>`) on the
 * native build and on the WebAssembly build under Node's WASI
 * (`scripts/fuzz-wasi.mts`). The corpus check and the shrinker both go
 * through here; the nightly workflow calls the two directly.
 */
import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

export const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
export const CONFORMANCE_DIR = resolve(REPO_ROOT, "conformance");

/** The release build of the fuzzer, or `VC_FUZZ_BIN`. */
export const FUZZ_BIN = process.env.VC_FUZZ_BIN ?? resolve(REPO_ROOT, "engine-rs/target/release/fuzz");
/** The fuzzer built for wasm32-wasip1, or `VC_FUZZ_WASM`. */
export const FUZZ_WASM = process.env.VC_FUZZ_WASM ?? resolve(REPO_ROOT, "engine-rs/target/wasm32-wasip1/release/fuzz.wasm");

export interface Replay {
  /** 0 ran; 1 the engine refused the scenario; 2 bad input; 3 timed out;
   *  4 the WebAssembly build trapped; 101 a native Rust panic. */
  code: number;
  stderr: string;
}

/** What is missing before a replay can run, or `null`. */
export function buildProblem(wasm: boolean): string | null {
  if (!existsSync(FUZZ_BIN)) return `no fuzzer at ${FUZZ_BIN} (cargo build --release --features conformance --bin fuzz, in engine-rs/)`;
  if (wasm && !existsSync(FUZZ_WASM)) return `no WebAssembly fuzzer at ${FUZZ_WASM} (add --target wasm32-wasip1 to that build)`;
  return null;
}

/** `timeoutMs` caps the run (default 120 s); a run that hits it reports code 3. */
export function nativeReplay(scenarioFile: string, outDir: string, timeoutMs = 120_000): Replay {
  const r = spawnSync(FUZZ_BIN, ["--replay", scenarioFile, outDir, CONFORMANCE_DIR], { encoding: "utf8", timeout: timeoutMs });
  return { code: r.status ?? 3, stderr: r.stderr ?? "" };
}

/** The same replay on the WebAssembly build, in a fresh Node process. */
export function wasmReplay(scenarioFile: string, outDir: string, timeoutMs = 120_000): Replay {
  const r = spawnSync(
    process.execPath,
    ["--no-turbo-fast-api-calls", "--no-warnings", resolve(REPO_ROOT, "scripts/fuzz-wasi.mts"), FUZZ_WASM, scenarioFile, outDir, CONFORMANCE_DIR],
    { encoding: "utf8", timeout: timeoutMs },
  );
  return { code: r.status ?? 3, stderr: r.stderr ?? "" };
}

/** The line of stderr worth showing: the panic message (Rust prints
 *  `panicked at <place>:` and then the message) or else the last line that is
 *  not the `note: run with RUST_BACKTRACE` hint. */
export function rustMessage(stderr: string): string {
  const lines = stderr.split("\n").map((l) => l.trim()).filter((l) => l !== "");
  const at = lines.findIndex((l) => l.includes("panicked at"));
  if (at >= 0) return lines.slice(at, at + 2).filter((l) => !l.startsWith("note:")).join(" ");
  return lines.filter((l) => !l.startsWith("note:")).pop() ?? "no output";
}
