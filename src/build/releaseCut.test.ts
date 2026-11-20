import { describe, expect, it } from "vitest";
import {
  bumpVersion,
  crateVersion,
  movePins,
  nextVersion,
  renameUnreleased,
  setCrateVersion,
  setLockedVersion,
  setRecordVersion,
  unreleasedTier,
} from "./releaseCut";

const changelog = (body: string) => `# Engine changelog\n\nIntro.\n\n## Unreleased\n\n${body}\n## 1.58.0 (2026-11-19)\n\n- **major**: an old entry.\n`;

describe("unreleasedTier", () => {
  it("takes the highest tier among the Unreleased entries", () => {
    expect(unreleasedTier(changelog("- **patch**: a fix.\n  More text.\n\n- **minor**: a new API.\n"))).toBe("minor");
    expect(unreleasedTier(changelog("- **patch**: a fix.\n"))).toBe("patch");
    expect(unreleasedTier(changelog("- **minor**: a.\n- **major**: b.\n- **patch**: c.\n"))).toBe("major");
  });

  it("reads only the Unreleased section", () => {
    expect(unreleasedTier(changelog("- **patch**: a fix.\n"))).not.toBe("major");
  });

  it("refuses an entry with no tier, an empty section and a missing one", () => {
    expect(() => unreleasedTier(changelog("- **patch**: a fix.\n- a line with no tier.\n"))).toThrow(/names no tier/);
    expect(() => unreleasedTier(changelog(""))).toThrow(/no entries/);
    expect(() => unreleasedTier("# Engine changelog\n\n## 1.0.0 (2026-01-01)\n\n- **patch**: x.\n")).toThrow(/no "## Unreleased"/);
  });
});

describe("nextVersion", () => {
  it("bumps by tier", () => {
    expect(bumpVersion("1.58.3", "patch")).toBe("1.58.4");
    expect(bumpVersion("1.58.3", "minor")).toBe("1.59.0");
    expect(bumpVersion("1.58.3", "major")).toBe("2.0.0");
  });

  it("takes a named version only at or past the bump", () => {
    expect(nextVersion("1.58.0", "minor")).toBe("1.59.0");
    expect(nextVersion("1.58.0", "minor", "")).toBe("1.59.0");
    expect(nextVersion("1.58.0", "minor", "2.0.0")).toBe("2.0.0");
    expect(nextVersion("1.58.0", "minor", "1.59.0")).toBe("1.59.0");
    expect(() => nextVersion("1.58.0", "minor", "1.58.1")).toThrow(/not 1\.59\.0 or later/);
    expect(() => nextVersion("1.58.0", "patch", "v1.59")).toThrow(/not a version/);
  });
});

describe("renameUnreleased", () => {
  it("renames the heading and keeps the rest", () => {
    const out = renameUnreleased(changelog("- **minor**: a.\n"), "1.59.0", "2026-11-20");
    expect(out).toContain("## 1.59.0 (2026-11-20)\n\n- **minor**: a.\n");
    expect(out).not.toContain("## Unreleased");
    expect(out).toContain("## 1.58.0 (2026-11-19)");
  });

  it("refuses a version the changelog already has", () => {
    expect(() => renameUnreleased(changelog("- **patch**: a.\n"), "1.58.0", "2026-11-20")).toThrow(/already has a 1\.58\.0/);
  });
});

describe("Cargo files", () => {
  const toml = `[package]\nname = "verticopolis-engine"\nversion = "1.58.0"\n\n[dependencies]\nserde = { version = "1.0.1" }\n`;

  it("reads and sets the package version only", () => {
    expect(crateVersion(toml)).toBe("1.58.0");
    const out = setCrateVersion(toml, "1.59.0");
    expect(crateVersion(out)).toBe("1.59.0");
    expect(out).toContain('serde = { version = "1.0.1" }');
  });

  it("moves the one locked engine entry", () => {
    const lock = `[[package]]\nname = "serde"\nversion = "1.58.0"\n\n[[package]]\nname = "verticopolis-engine"\nversion = "1.58.0"\ndependencies = []\n`;
    const out = setLockedVersion(lock, "1.58.0", "1.59.0");
    expect(out).toContain('name = "verticopolis-engine"\nversion = "1.59.0"');
    expect(out).toContain('name = "serde"\nversion = "1.58.0"');
    expect(() => setLockedVersion(lock, "1.57.0", "1.59.0")).toThrow(/has verticopolis-engine 1\.58\.0/);
    expect(() => setLockedVersion("", "1.58.0", "1.59.0")).toThrow(/found 0/);
  });

  it("moves the compatibility record's crate version", () => {
    const record = `{\n  "crateVersion": "1.58.0",\n  "saveVersion": 13\n}\n`;
    expect(setRecordVersion(record, "1.58.0", "1.59.0")).toContain('"crateVersion": "1.59.0"');
    expect(() => setRecordVersion(record, "1.57.0", "1.59.0")).toThrow(/does not hold/);
  });
});

describe("movePins", () => {
  it("moves quoted pins and leaves history", () => {
    const text = 'from 1.58.0 the package\nverticopolis-engine = "=1.58.0"\n(`"1.58.0"`, not `"^1.58.0"`)\n"1.58.01"\n';
    const { text: out, moved } = movePins(text, "1.58.0", "1.59.0");
    expect(moved).toBe(3);
    expect(out).toBe('from 1.58.0 the package\nverticopolis-engine = "=1.59.0"\n(`"1.59.0"`, not `"^1.59.0"`)\n"1.58.01"\n');
  });
});
