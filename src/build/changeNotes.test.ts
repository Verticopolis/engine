import { describe, expect, it } from "vitest";
import { highestTier, isNoteFile, isRustCode, isShipped, parseNote, sectionBody } from "./changeNotes";

const note = (front: string, body: string) => `---\n${front}\n---\n${body}`;

describe("parseNote", () => {
  it("reads the tier and the entries with their continuation lines", () => {
    const n = parseNote("a.md", note("crate: minor    # a comment", "- a new API.\n  More about it.\n\n- a second entry.\n"));
    expect(n).toEqual({ name: "a.md", tier: "minor", entries: ["- a new API.\n  More about it.", "- a second entry."] });
  });

  it("takes a none note with no entries", () => {
    expect(parseNote("n.md", note("crate: none", "")).entries).toEqual([]);
  });

  it("refuses a malformed note", () => {
    expect(() => parseNote("x.md", "- no front matter.\n")).toThrow(/front matter/);
    expect(() => parseNote("x.md", "---\ncrate: patch\n- never closed.\n")).toThrow(/closing/);
    expect(() => parseNote("x.md", note("crate: huge", "- x."))).toThrow(/not one of/);
    expect(() => parseNote("x.md", note("app: patch", "- x."))).toThrow(/"crate:" line/);
    expect(() => parseNote("x.md", note("", "- x."))).toThrow(/names no/);
    expect(() => parseNote("x.md", note("crate: patch\ncrate: minor", "- x."))).toThrow(/twice/);
    expect(() => parseNote("x.md", note("crate: patch", ""))).toThrow(/at least one entry/);
    expect(() => parseNote("x.md", note("crate: none", "- x."))).toThrow(/has no entries/);
    expect(() => parseNote("x.md", note("crate: patch", "Some prose.\n"))).toThrow(/every line/);
    expect(() => parseNote("x.md", note("crate: patch", "- **patch**: x."))).toThrow(/leave the tier/);
  });
});

describe("the release", () => {
  const notes = [
    parseNote("b.md", note("crate: patch", "- a fix.")),
    parseNote("c.md", note("crate: none", "")),
    parseNote("a.md", note("crate: minor", "- a new API.\n  Two lines.")),
    parseNote("d.md", note("crate: minor", "- another API.")),
  ];

  it("takes the highest tier, and none when nothing ships", () => {
    expect(highestTier(notes)).toBe("minor");
    expect(highestTier([notes[1]])).toBeUndefined();
    expect(highestTier([])).toBeUndefined();
  });

  it("leads each entry with its tier, highest tier first, then by file name", () => {
    expect(sectionBody(notes)).toBe("- **minor**: a new API.\n  Two lines.\n- **minor**: another API.\n- **patch**: a fix.");
  });

  it("reads note files only", () => {
    expect(isNoteFile("claude-project-thread-abc.md")).toBe(true);
    expect(isNoteFile("README.md")).toBe(false);
    expect(isNoteFile(".gitkeep")).toBe(false);
  });
});

describe("the paths the check reads", () => {
  it("counts every path that reaches the crate or the npm package as shipped", () => {
    for (const path of [
      "engine-rs/src/sim.rs",
      "engine-rs/Cargo.toml",
      "src/events/gameplayCatalog.ts",
      "conformance/expected.json",
      "packages/verticopolis-engine/web.d.ts",
      "package.json",
      "scripts/pack-engine.sh",
      "scripts/engine-package-files.sh",
      "scripts/build-engine-wasm.sh",
      "scripts/wasm-build.ts",
    ]) {
      expect(isShipped(path), path).toBe(true);
    }
  });

  it("leaves docs, tests of the tooling, the bench and CI out", () => {
    for (const path of [
      "README.md",
      "engine-rs/CHANGELOG.md",
      "engine-rs/tests/independence.rs",
      "engine-bench/src/main.rs",
      "src/build/releaseCut.ts",
      "scripts/cut-release.ts",
      "package-lock.json",
      ".github/workflows/engine-rs.yml",
      "docs/engine-site/guides/crate.md",
    ]) {
      expect(isShipped(path), path).toBe(false);
    }
  });

  it("tells Rust code from the test files under engine-rs/src/", () => {
    for (const path of ["engine-rs/src/sim.rs", "engine-rs/src/panels/mod.rs", "engine-rs/src/contest.rs", "engine-rs/src/latest.rs"]) {
      expect(isRustCode(path), path).toBe(true);
    }
    for (const path of [
      "engine-rs/src/catalog/tests.rs",
      "engine-rs/src/panels/tests_gates.rs",
      "engine-rs/src/crowd/spawn_tests.rs",
      "engine-rs/src/rule_tests/attendance.rs",
      "engine-rs/src/tdt/tests/roundtrip.rs",
      "engine-rs/tests/independence.rs",
      "engine-rs/src/catalog/mod.ts",
    ]) {
      expect(isRustCode(path), path).toBe(false);
    }
  });
});
