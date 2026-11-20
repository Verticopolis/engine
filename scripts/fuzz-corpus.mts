/**
 * Replay every entry of conformance/fuzz-corpus/ on the native fuzzer and on
 * the fuzzer built for WebAssembly (under Node's WASI), and fail when the
 * native run leaves the entry's golden (conformance/fuzz-corpus/goldens/) or
 * the WebAssembly run leaves the native one. The `wasm-referee` job of
 * engine-rs.yml runs it on every pull request and on main. Usage:
 *   (cd engine-rs && cargo build --release --features conformance --bin fuzz \
 *     && cargo build --release --features conformance --bin fuzz --target wasm32-wasip1)
 *   node scripts/fuzz-corpus.mts [--budget <seconds>] [--update]
 * --update pins each entry's native records as its golden instead of checking
 * them (local only, never in CI), and still checks the WebAssembly leg; commit
 * the goldens with the change that moves them and say why.
 * Exit 0 all entries agree, 1 a divergence, a refusal, a trap or the budget
 * ran out (default 240 s; a replay still running at the deadline is cut off),
 * 2 bad arguments, 3 the harness failed (a build missing, an unreadable entry
 * or folder, an empty corpus, a missing or malformed record).
 */
import { copyFileSync, mkdirSync, mkdtempSync, readdirSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, join, resolve } from "node:path";
import { compareRuns, describeDivergence, readRecord, type Leg } from "./fuzz/records.mts";
import { buildProblem, CONFORMANCE_DIR, nativeReplay, rustMessage, wasmReplay } from "./fuzz/run.mts";

const USAGE = "usage: fuzz-corpus [--budget <seconds>] [--update]";
const CORPUS = resolve(CONFORMANCE_DIR, "fuzz-corpus");
const GOLDENS = resolve(CORPUS, "goldens");
/** The records `fuzz --replay` writes are not entries. */
const RECORD = /\.rs(\.detail)?\.json$/;

const args = process.argv.slice(2);
const update = args.includes("--update");
const flag = args.indexOf("--budget");
const budget = flag >= 0 ? Number(args[flag + 1]) : 240;
const known = args.filter((a, i) => a === "--update" || a === "--budget" || (flag >= 0 && i === flag + 1));
if (!Number.isFinite(budget) || budget <= 0 || known.length !== args.length) {
  console.error(USAGE);
  process.exit(2);
}
if (update && process.env.CI) {
  console.error("fuzz-corpus --update is a local regeneration switch and never runs in CI");
  process.exit(2);
}
const problem = buildProblem(true);
if (problem) {
  console.error(`HARNESS: ${problem}`);
  process.exit(3);
}
let files: string[] = [];
try {
  files = readdirSync(CORPUS)
    .filter((f) => f.endsWith(".json") && !RECORD.test(f))
    .sort()
    .map((f) => resolve(CORPUS, f));
} catch (e) {
  console.error(`HARNESS: cannot read conformance/fuzz-corpus/: ${(e as Error).message}`);
  process.exit(3);
}
if (files.length === 0) {
  console.error("HARNESS: conformance/fuzz-corpus/ holds no entries, so the job would prove nothing");
  process.exit(3);
}

/** Compare `got` (from `gotDir`) with the native records; a string on failure. */
function agree(id: string, nativeDir: string, gotDir: string, leg: Leg): { ok: true; checkpoints: number } | { ok: false; harness: boolean; message: string } {
  const rust = readRecord(nativeDir, id);
  const other = readRecord(gotDir, id);
  if (!rust || !other) return { ok: false, harness: true, message: `missing ${leg === "golden" ? "golden" : "records"} for ${id}` };
  try {
    const d = compareRuns(other, rust, leg);
    return d ? { ok: false, harness: false, message: describeDivergence(d, leg) } : { ok: true, checkpoints: rust.checkpoints.length };
  } catch (e) {
    return { ok: false, harness: true, message: (e as Error).message };
  }
}

const work = mkdtempSync(join(tmpdir(), "vc-corpus-"));
const nativeDir = join(work, "native");
const wasmDir = join(work, "wasm");
const started = Date.now();
const failed: string[] = [];
let harness = false;
/** With --update, the entries whose fresh records wait to be pinned. Nothing is
 *  written to the goldens until every entry has passed, so a failed or
 *  over-budget refresh leaves the committed goldens as they were. */
const toPin: string[] = [];
const remaining = () => budget * 1000 - (Date.now() - started);
try {
  for (const file of files) {
    const name = basename(file);
    const id = name.replace(/\.json$/, "");
    if (remaining() <= 0) {
      console.log(`${name}: not run, the ${budget} s budget is spent`);
      failed.push(`${name} (over budget)`);
      continue;
    }
    const rust = nativeReplay(file, nativeDir, Math.min(120_000, remaining()));
    if (rust.code !== 0) {
      console.log(`${name}: the native run ${rust.code === 1 ? "stopped" : rust.code === 101 ? "panicked" : rust.code === 3 ? "timed out" : `failed (${rust.code})`}: ${rustMessage(rust.stderr)}`);
      if (rust.code === 1 || rust.code === 101 || rust.code === 3) failed.push(name);
      else harness = true;
      continue;
    }
    if (update) toPin.push(id);
    else {
      const g = agree(id, nativeDir, GOLDENS, "golden");
      if (!g.ok) {
        console.log(`${name}: native run against its golden: ${g.message}`);
        if (g.harness) harness = true;
        else failed.push(name);
        continue;
      }
    }
    const wasm = wasmReplay(file, wasmDir, Math.min(120_000, Math.max(1, remaining())));
    if (wasm.code !== 0) {
      const what = wasm.code === 6 ? "refused what the native build ran" : wasm.code === 4 ? "trapped" : wasm.code === 3 ? "timed out" : `failed to run (${wasm.code})`;
      console.log(`${name}: the WebAssembly build ${what}: ${rustMessage(wasm.stderr)}`);
      if ([3, 4, 6].includes(wasm.code)) failed.push(name);
      else harness = true;
      continue;
    }
    const w = agree(id, nativeDir, wasmDir, "WASM");
    if (w.ok) console.log(`${name}: ok (${w.checkpoints} checkpoints)`);
    else {
      console.log(`${name}: ${w.message}`);
      if (w.harness) harness = true;
      else failed.push(name);
    }
  }
  if (remaining() < 0 && !failed.some((f) => f.endsWith("(over budget)"))) failed.push("(the last entry ran past the budget)");
  if (update) {
    if (failed.length === 0 && !harness) {
      mkdirSync(GOLDENS, { recursive: true });
      for (const id of toPin) for (const record of [`${id}.rs.json`, `${id}.rs.detail.json`]) copyFileSync(join(nativeDir, record), join(GOLDENS, record));
      console.log(`${toPin.length} goldens pinned`);
    } else console.log("no golden written: the refresh did not pass on every entry");
  }
} finally {
  rmSync(work, { recursive: true, force: true });
}
const seconds = ((Date.now() - started) / 1000).toFixed(1);
console.log(`${files.length} corpus entries in ${seconds} s${failed.length ? `; failed: ${failed.join(", ")}` : ""}`);
process.exit(failed.length ? 1 : harness ? 3 : 0);
