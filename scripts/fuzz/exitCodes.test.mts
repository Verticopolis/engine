import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import type { RunRecord } from "./records.mts";

// The fuzz scripts' exit codes are a contract with engine-fuzz.yml and
// fuzz-corpus.mts: a harness failure must never land on a code that means a
// finding (6 refused or diverged, 4 trapped), nor on 1, which Node uses when
// a script crashes. The refused (6) and trap (4) paths of fuzz-wasi.mts need
// the WebAssembly build, so the corpus check and the nightly cover them.
const SCRIPTS = resolve(import.meta.dirname, "..");
const run = (script: string, ...args: string[]) =>
  spawnSync(process.execPath, [join(SCRIPTS, script), ...args], { encoding: "utf8" }).status;

const record = (): RunRecord => ({
  checkpoints: [{ label: "start", state: "s0", crowd: "c0", events: "e0" }],
  details: [{ label: "start", money: 2000000, log: { seq: 0, count: 0, digest: "l0" }, save: { clock: "k0" }, fx: { bombs: 0 } }],
});
const write = (dir: string, id: string, r: RunRecord) => {
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, `${id}.rs.json`), JSON.stringify(r.checkpoints));
  writeFileSync(join(dir, `${id}.rs.detail.json`), JSON.stringify(r.details));
};

describe("fuzz script exit codes", () => {
  let dir: string;
  beforeEach(() => {
    dir = mkdtempSync(join(tmpdir(), "fuzz-codes-"));
  });
  afterEach(() => rmSync(dir, { recursive: true, force: true }));

  it("fuzz-wasi exits 5 without its arguments", () => {
    expect(run("fuzz-wasi.mts")).toBe(5);
  });

  it("fuzz-wasi exits 5 when the .wasm is missing", () => {
    writeFileSync(join(dir, "s.json"), "{}");
    expect(run("fuzz-wasi.mts", join(dir, "absent.wasm"), join(dir, "s.json"), join(dir, "out"))).toBe(5);
  });

  it("fuzz-compare exits 0 on identical records", () => {
    write(join(dir, "native"), "a", record());
    write(join(dir, "wasm"), "a", record());
    expect(run("fuzz-compare.mts", join(dir, "native"), join(dir, "wasm"), "a")).toBe(0);
  });

  it("fuzz-compare exits 6 when one hash differs", () => {
    const other = record();
    other.checkpoints[0].state = "x";
    write(join(dir, "native"), "a", record());
    write(join(dir, "wasm"), "a", other);
    expect(run("fuzz-compare.mts", join(dir, "native"), join(dir, "wasm"), "a")).toBe(6);
  });

  it("fuzz-compare exits 3 when a record is missing or unreadable", () => {
    write(join(dir, "native"), "a", record());
    mkdirSync(join(dir, "wasm"));
    expect(run("fuzz-compare.mts", join(dir, "native"), join(dir, "wasm"), "a")).toBe(3);
    writeFileSync(join(dir, "wasm", "a.rs.json"), "not json");
    writeFileSync(join(dir, "wasm", "a.rs.detail.json"), "[]");
    expect(run("fuzz-compare.mts", join(dir, "native"), join(dir, "wasm"), "a")).toBe(3);
  });

  it("fuzz-compare exits 3 without its arguments", () => {
    expect(run("fuzz-compare.mts")).toBe(3);
  });
});
