/**
 * Shrink a diverging fuzzer scenario to a 1-minimal command list (no single
 * command can be dropped) that still diverges the same way, and write it to
 * the corpus. Delta debugging does not promise the globally shortest list, and
 * a run cut off by `--max-tests` may not be 1-minimal; the entry's description
 * says so. Usage:
 *   node scripts/fuzz-shrink.mts <fuzz-out>/fuzz-<seed>.json <name> [--max-tests 4000]
 * The candidates replay on the native fuzzer and on the fuzzer built for
 * WebAssembly under Node's WASI, so both builds must exist
 * (`cargo build --release --features conformance --bin fuzz`, in engine-rs/,
 * once as is and once with `--target wasm32-wasip1`).
 * A candidate counts as still failing only when the native build runs it
 * cleanly and the WebAssembly build shows the same kind of divergence as the
 * original (`state:units`, `money`, `refused`, `trap`), so a smaller scenario
 * that merely breaks differently is not accepted. The result goes to
 * conformance/fuzz-corpus/<name>.json; once the fix is in, pin its golden with
 * `node scripts/fuzz-corpus.mts --update` and commit both with the fix, where
 * the corpus job then holds the entry to its golden and to agreement. Exit 0
 * written, 1 the original does not diverge (or the Rust engine refuses it),
 * 2 bad arguments, an unreadable input or a taken name, 3 the harness failed
 * (a build missing, a timeout, a malformed record). The default limit is 4000 tests: a
 * tall night needs about 1300.
 */
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { ddmin } from "./fuzz/ddmin.mts";
import { compareRuns, readRecord, signature } from "./fuzz/records.mts";
import { buildProblem, CONFORMANCE_DIR, nativeReplay, rustMessage, wasmReplay } from "./fuzz/run.mts";

/** A corpus file name: lowercase words and digits joined by dashes. */
const CORPUS_NAME = /^[a-z0-9]+(-[a-z0-9]+)*$/;
const corpusDir = (): string => resolve(CONFORMANCE_DIR, "fuzz-corpus");

/** What running a candidate natively and on the WASM build showed. Only `diverged` can be
 *  shrunk toward; the others tell the caller why it cannot. */
type Shown =
  | { kind: "diverged"; signature: string }
  | { kind: "converged" }
  | { kind: "refused"; message: string }
  | { kind: "harness"; message: string };

/** The flags after the file and name: `--max-tests <n>`, at most once.
 *  Anything else (a typo, or the retired `--wasm`) is refused. */
function parseOptions(args: readonly string[]): { maxTests: number } | null {
  let maxTests = 4000;
  const seen = new Set<string>();
  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    if (seen.has(arg)) return null;
    seen.add(arg);
    if (arg === "--max-tests") {
      maxTests = Number(args[++i]);
      if (!Number.isInteger(maxTests) || maxTests < 1) return null;
    } else return null;
  }
  return { maxTests };
}

function main(): number {
  const [file, name, ...rest] = process.argv.slice(2);
  const options = parseOptions(rest);
  if (!file || !name || !CORPUS_NAME.test(name) || !options) {
    console.error("usage: fuzz-shrink <scenario.json> <name: lowercase words and digits joined by dashes> [--max-tests <n>]");
    return 2;
  }
  const { maxTests } = options;
  const problem = buildProblem(true);
  if (problem) {
    console.error(problem);
    return 3;
  }
  // Check the name before any work, so a taken one fails fast and no work
  // directory exists yet to leak.
  const out = resolve(corpusDir(), `${name}.json`);
  if (existsSync(out)) {
    console.error(`${out} already exists; pick another name`);
    return 2;
  }

  let original: { id: string; description: string; start: unknown; commands: unknown[] };
  try {
    original = JSON.parse(readFileSync(file, "utf8"));
    if (!original || typeof original.id !== "string" || typeof original.description !== "string" || !Array.isArray(original.commands)) throw new Error("not a scenario (needs id, description and commands)");
  } catch (e) {
    console.error(`${file}: cannot read the scenario: ${(e as Error).message}`);
    return 2;
  }
  const work = mkdtempSync(join(tmpdir(), "vc-shrink-"));
  try {
    /** Run a candidate: the native build must run it cleanly (a scenario it
     *  refuses is not a valid shrink) and the WebAssembly build must then differ. */
    const show = (commands: readonly unknown[], id = "candidate"): Shown => {
      const path = join(work, `${id}.json`);
      const nativeDir = join(work, "native");
      const wasmDir = join(work, "wasm");
      rmSync(nativeDir, { recursive: true, force: true });
      rmSync(wasmDir, { recursive: true, force: true });
      writeFileSync(path, JSON.stringify({ ...original, id, commands }));
      const rust = nativeReplay(path, nativeDir);
      if (rust.code === 1) return { kind: "refused", message: `the native build refuses it: ${rustMessage(rust.stderr)}` };
      if (rust.code !== 0) return { kind: "harness", message: `the native replay failed (${rust.code}): ${rustMessage(rust.stderr)}` };
      const wasm = wasmReplay(path, wasmDir);
      if (wasm.code === 6) return { kind: "diverged", signature: "refused" };
      if (wasm.code === 4) return { kind: "diverged", signature: "trap" };
      if (wasm.code !== 0) return { kind: "harness", message: `the WebAssembly replay failed (${wasm.code}): ${rustMessage(wasm.stderr)}` };
      const a = readRecord(nativeDir, id);
      const b = readRecord(wasmDir, id);
      if (!a || !b) return { kind: "harness", message: `missing records for ${id}` };
      try {
        const d = compareRuns(b, a, "WASM");
        return d ? { kind: "diverged", signature: signature(d) } : { kind: "converged" };
      } catch (e) {
        return { kind: "harness", message: (e as Error).message };
      }
    };

    const first = show(original.commands, "original");
    if (first.kind === "harness") {
      console.error(`${file}: ${first.message}`);
      return 3;
    }
    if (first.kind !== "diverged") {
      console.log(`${file}: ${first.kind === "refused" ? first.message : "does not diverge with the fuzzer's current build"}, nothing to shrink`);
      return 1;
    }
    const target = first.signature;
    // A harness failure on a candidate is not "still failing": the shrink
    // goes on without it, and the final result is still a verified divergence.
    const r = ddmin(original.commands, (c) => {
      const shown = show(c);
      return shown.kind === "diverged" && shown.signature === target;
    }, maxTests);
    const note = r.exhausted ? " The search stopped at its test limit, so the list may not be 1-minimal." : "";
    const shrunk = {
      id: name,
      description: `Shrunk from ${original.id} (${original.commands.length} to ${r.kept.length} commands), which diverged as ${target} on the WASM leg.${note} ${original.description}`,
      start: original.start,
      commands: r.kept,
    };
    mkdirSync(corpusDir(), { recursive: true });
    writeFileSync(out, `${JSON.stringify(shrunk, null, 2)}\n`);
    console.log(`${original.commands.length} commands to ${r.kept.length} in ${r.tests} tests${r.exhausted ? " (stopped at the test limit, may not be 1-minimal)" : ""}; signature ${target}; wrote ${out}`);
    return 0;
  } finally {
    rmSync(work, { recursive: true, force: true });
  }
}

process.exit(main());
