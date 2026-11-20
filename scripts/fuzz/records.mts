/**
 * The seeded differential's compare: one run's checkpoints and details against
 * another's, naming the first field that differs. Both sides are records the
 * fuzzer writes (`<id>.rs.json` and `<id>.rs.detail.json`): the native build
 * against the WebAssembly build (`scripts/fuzz-compare.mts`), or a corpus
 * entry's native run against its pinned golden (`scripts/fuzz-corpus.mts`).
 * Ported from the game repository's `src/tests/conformance/fuzzCompare.ts`.
 */
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";

export interface Checkpoint {
  label: string;
  state: string;
  crowd: string;
  events: string;
}

/** What the fuzzer records at a checkpoint beyond the hashes
 *  (`engine-rs/src/scenario_detail.rs`): the money, the log, a digest of each
 *  top-level save field and the cosmetic fx slots. */
export interface CheckpointDetail {
  label: string;
  money: number;
  log: { seq: number; count: number; digest: string };
  save: Record<string, string>;
  fx: Record<string, number>;
}

export interface RunRecord {
  checkpoints: Checkpoint[];
  details: CheckpointDetail[];
}

/** Which part of a checkpoint differed first, in the order the compare looks:
 *  the label, the three hashes, then the detail (money, log, save field, fx slots). */
export type DivergenceKind = "count" | "label" | "state" | "crowd" | "events" | "money" | "log" | "save" | "fx";

/** What the native run is compared with. */
export type Leg = "WASM" | "golden";

export interface Divergence {
  /** The checkpoint index, or the shorter run's length for `count`. */
  index: number;
  label: string;
  kind: DivergenceKind;
  /** The log part (`seq`, `count`, `digest`), the save field or the fx slot. */
  field?: string;
  /** The compared leg's value. */
  other: string;
  rust: string;
}

const HASHES = ["state", "crowd", "events"] as const;

const isObject = (v: unknown): v is Record<string, unknown> => typeof v === "object" && v !== null && !Array.isArray(v);

/** Read the two record files of `id` from `dir`, or `null` when either is missing. */
export function readRecord(dir: string, id: string): RunRecord | null {
  const files = [join(dir, `${id}.rs.json`), join(dir, `${id}.rs.detail.json`)];
  if (!files.every((f) => existsSync(f))) return null;
  const [checkpoints, details] = files.map((f) => JSON.parse(readFileSync(f, "utf8")) as unknown);
  return { checkpoints: checkpoints as Checkpoint[], details: details as CheckpointDetail[] };
}

/** Why a run record cannot be compared, or `null` when it can. A record that is
 *  short, relabeled or of the wrong shape must never read as agreement, so the
 *  callers report it as a harness failure and not a divergence. */
export function recordProblem(run: RunRecord, what: string): string | null {
  if (!Array.isArray(run.checkpoints) || !Array.isArray(run.details)) return `${what}: the records are not lists`;
  if (run.details.length !== run.checkpoints.length) return `${what}: ${run.details.length} detail records for ${run.checkpoints.length} checkpoints`;
  for (let i = 0; i < run.checkpoints.length; i++) {
    const c: unknown = run.checkpoints[i];
    const d: unknown = run.details[i];
    if (!isObject(c) || !["label", "state", "crowd", "events"].every((k) => typeof c[k] === "string")) return `${what}: checkpoint ${i} is not a label with state, crowd and events hashes`;
    if (!isObject(d) || d.label !== c.label || typeof d.money !== "number" || !isObject(d.log) || !isObject(d.save) || !isObject(d.fx)) return `${what}: detail ${i} does not match checkpoint ${i} (${String(c.label)})`;
  }
  return null;
}

/** The first detail difference at one checkpoint, if any. */
function detailDivergence(a: CheckpointDetail, b: CheckpointDetail): Pick<Divergence, "kind" | "field" | "other" | "rust"> | null {
  if (a.money !== b.money) return { kind: "money", other: String(a.money), rust: String(b.money) };
  for (const part of ["seq", "count", "digest"] as const) {
    if (a.log[part] !== b.log[part]) return { kind: "log", field: part, other: String(a.log[part]), rust: String(b.log[part]) };
  }
  for (const key of [...new Set([...Object.keys(a.save), ...Object.keys(b.save)])].sort()) {
    if (a.save[key] !== b.save[key]) return { kind: "save", field: key, other: String(a.save[key] ?? "(absent)"), rust: String(b.save[key] ?? "(absent)") };
  }
  for (const key of [...new Set([...Object.keys(a.fx), ...Object.keys(b.fx)])].sort()) {
    if (a.fx[key] !== b.fx[key]) return { kind: "fx", field: key, other: String(a.fx[key] ?? "(absent)"), rust: String(b.fx[key] ?? "(absent)") };
  }
  return null;
}

/** The first divergence between two runs of one scenario, or `null` when they
 *  agree. A hash difference also names the first thing the detail records show
 *  differing, so the report points at the cause as well as the digest. Throws
 *  when a record cannot be compared. */
export function compareRuns(run: RunRecord, rust: RunRecord, leg: Leg = "WASM"): Divergence | null {
  const problem = recordProblem(run, leg) ?? recordProblem(rust, "Rust");
  if (problem) throw new Error(problem);
  const n = Math.min(run.checkpoints.length, rust.checkpoints.length);
  for (let i = 0; i < n; i++) {
    const a = run.checkpoints[i];
    const b = rust.checkpoints[i];
    if (a.label !== b.label) return { index: i, label: a.label, kind: "label", other: a.label, rust: b.label };
    const detail = detailDivergence(run.details[i], rust.details[i]);
    for (const h of HASHES) {
      if (a[h] !== b[h]) {
        return { index: i, label: a.label, kind: h, ...(detail ? { field: detail.field ?? detail.kind } : {}), other: a[h], rust: b[h] };
      }
    }
    if (detail) return { index: i, label: a.label, ...detail };
  }
  if (run.checkpoints.length !== rust.checkpoints.length) {
    return { index: n, label: "(count)", kind: "count", other: String(run.checkpoints.length), rust: String(rust.checkpoints.length) };
  }
  return null;
}

/** The kind of a divergence, as the shrinker matches it (`state:units`, `money`). */
export function signature(d: Divergence): string {
  return d.field ? `${d.kind}:${d.field}` : d.kind;
}

/** One line for a log or a job summary. */
export function describeDivergence(d: Divergence, leg: Leg = "WASM"): string {
  const where = d.kind === "count" ? "checkpoint count" : `checkpoint ${d.index} (${d.label}): ${d.kind}${d.field ? ` field ${d.field}` : ""}`;
  return `DIVERGED at ${where}, ${leg} ${d.other} vs Rust ${d.rust}`;
}
