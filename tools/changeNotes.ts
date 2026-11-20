/**
 * Change notes: one file per pull request under `.changes/`
 * (`.changes/README.md`), so two open pull requests never edit the same
 * lines of `engine-rs/CHANGELOG.md`. A note names the crate tier in its front
 * matter and holds the changelog entries:
 *
 *   ---
 *   crate: patch
 *   ---
 *   - the binding's `foo` refuses a negative floor.
 *
 * `scripts/cut-release.ts` turns the pending notes into the version's
 * changelog section and deletes them; `scripts/change-notes-check.ts` checks
 * them on every pull request.
 *
 * Node only, and ERASABLE so plain `node` runs the scripts that import it.
 */

import type { Tier } from "./releaseCut.ts";

/** The front matter key that names the tier. */
export const TIER_KEY = "crate";

/** A note's tier: a release tier, or `none` for a change that ships nothing. */
export type NoteTier = Tier | "none";

export interface ChangeNote {
  name: string;
  tier: NoteTier;
  /** The changelog entries, each one `- ` bullet with its continuation lines. */
  entries: string[];
}

/** Paths whose change reaches the crate or the npm package. */
const SHIPPED_PREFIXES: readonly string[] = [
  "engine-rs/src/",
  "engine-rs/Cargo.toml",
  "src/events/",
  "conformance/",
  "packages/verticopolis-engine/",
  "package.json",
  "scripts/pack-engine.sh",
  "scripts/engine-package-files.sh",
  "scripts/build-engine-wasm.sh",
  "scripts/wasm-build.ts",
];
export const isShipped = (path: string) =>
  SHIPPED_PREFIXES.some((p) => path === p || (p.endsWith("/") && path.startsWith(p)));

/**
 * Rust code under engine-rs/src/ that is not a test file. A change to it,
 * a deletion included, needs a crate tier of patch or higher.
 */
const TEST_FILE = /(^|\/)(tests?(_[a-z0-9_]+)?|[a-z0-9_]+_tests)\.rs$|\/(rule_tests|tests)\//;
export const isRustCode = (path: string) =>
  path.startsWith("engine-rs/src/") && path.endsWith(".rs") && !TEST_FILE.test(path);

const NOTE_TIERS: readonly NoteTier[] = ["none", "patch", "minor", "major"];
const TIER_PREFIX = /^- \*\*(major|minor|patch|none)\*\*/;

/** True for a file name under `.changes/` that is a note (the README is not). */
export function isNoteFile(name: string): boolean {
  return name.endsWith(".md") && name !== "README.md";
}

/** Parses one note. `name` is its file name, for the error messages. */
export function parseNote(name: string, text: string): ChangeNote {
  const lines = text.replace(/\r\n/g, "\n").split("\n");
  if (lines[0]?.trimEnd() !== "---") throw new Error(`${name}: a note starts with a "---" front matter line`);
  const close = lines.findIndex((l, i) => i > 0 && l.trimEnd() === "---");
  if (close < 0) throw new Error(`${name}: the front matter has no closing "---" line`);

  let tier: NoteTier | undefined;
  for (const raw of lines.slice(1, close)) {
    const line = raw.replace(/#.*$/, "").trim();
    if (line === "") continue;
    const m = /^([a-z]+):\s*(\S+)$/.exec(line);
    if (!m || m[1] !== TIER_KEY)
      throw new Error(`${name}: the front matter takes one "${TIER_KEY}:" line, not "${raw.trim()}"`);
    if (tier !== undefined) throw new Error(`${name}: "${TIER_KEY}:" is given twice`);
    if (!NOTE_TIERS.includes(m[2] as NoteTier)) {
      throw new Error(`${name}: "${TIER_KEY}: ${m[2]}" is not one of ${NOTE_TIERS.join(", ")}`);
    }
    tier = m[2] as NoteTier;
  }
  if (tier === undefined) throw new Error(`${name}: the front matter names no "${TIER_KEY}:" tier`);

  const entries: string[] = [];
  for (const line of lines.slice(close + 1)) {
    if (line.startsWith("- ")) {
      if (TIER_PREFIX.test(line))
        throw new Error(`${name}: leave the tier to the front matter, not the entry: ${line}`);
      entries.push(line.trimEnd());
    } else if (line.trim() === "") {
      continue;
    } else if (/^\s/.test(line) && entries.length > 0) {
      entries[entries.length - 1] += `\n${line.trimEnd()}`;
    } else {
      throw new Error(`${name}: every line is a "- " entry or an indented continuation of one: ${line}`);
    }
  }
  if (tier === "none" && entries.length > 0) throw new Error(`${name}: a "${TIER_KEY}: none" note has no entries`);
  if (tier !== "none" && entries.length === 0)
    throw new Error(`${name}: a "${TIER_KEY}: ${tier}" note needs at least one entry`);
  return { name, tier, entries };
}

/** The highest release tier among the notes, or undefined when none ships anything. */
export function highestTier(notes: readonly ChangeNote[]): Tier | undefined {
  let best = 0;
  for (const note of notes) best = Math.max(best, NOTE_TIERS.indexOf(note.tier));
  return best === 0 ? undefined : (NOTE_TIERS[best] as Tier);
}

/**
 * The body of a version's changelog section: every entry led by its tier
 * (`- **minor**: ...`), the highest tier first and then by file name, so the
 * order never depends on the order the files were read.
 */
export function sectionBody(notes: readonly ChangeNote[]): string {
  const ordered = [...notes]
    .filter((n) => n.tier !== "none")
    .sort((a, b) => NOTE_TIERS.indexOf(b.tier) - NOTE_TIERS.indexOf(a.tier) || a.name.localeCompare(b.name, "en"));
  const entries = ordered.flatMap((n) => n.entries.map((e) => `- **${n.tier}**: ${e.slice(2)}`));
  return entries.join("\n");
}
