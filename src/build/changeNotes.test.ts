import { describe, expect, it } from "vitest";
import { highestTier, isNoteFile, parseNote, sectionBody } from "./changeNotes";

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
