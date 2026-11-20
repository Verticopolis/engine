/**
 * Checks the change notes (`.changes/README.md`) on a pull request:
 *
 *   node scripts/change-notes-check.ts --base origin/main
 *
 * Every note under `.changes/` must parse. With `--base`, a pull request that
 * changes what the crate ships (the paths in SHIPPED) must add a note, a
 * `crate: none` one included, and must not edit or delete a note already on
 * the base branch, which belongs to the pull request that merged it. Release
 * branches (`release/`) and Dependabot's (`dependabot/`) are exempt from both,
 * the way the game's version check exempts them: the release PR deletes the
 * notes, and Dependabot cannot write one. The branch name comes from
 * GITHUB_HEAD_REF.
 */
import { execFileSync } from "node:child_process";
import { readdirSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { isNoteFile, parseNote } from "../src/build/changeNotes.ts";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const NOTES = ".changes";
/** Path prefixes whose change reaches the crate or the npm package. */
const SHIPPED = ["engine-rs/src/", "engine-rs/Cargo.toml", "packages/verticopolis-engine/"];
const EXEMPT_BRANCHES = ["release/", "dependabot/"];

function arg(name: string): string | undefined {
  const i = process.argv.indexOf(name);
  return i >= 0 ? process.argv[i + 1] : undefined;
}

const problems: string[] = [];
for (const f of readdirSync(join(root, NOTES)).filter(isNoteFile)) {
  try {
    parseNote(f, readFileSync(join(root, NOTES, f), "utf8"));
  } catch (e) {
    problems.push(e instanceof Error ? e.message : String(e));
  }
}

const base = arg("--base");
const branch = process.env.GITHUB_HEAD_REF ?? "";
if (base && !EXEMPT_BRANCHES.some((p) => branch.startsWith(p))) {
  const diff = execFileSync("git", ["diff", "--name-status", "--no-renames", `${base}...HEAD`], { cwd: root, encoding: "utf8" });
  const rows = diff
    .split("\n")
    .filter(Boolean)
    .map((l) => l.split("\t") as [string, string]);
  const isNote = (path: string) => path.startsWith(`${NOTES}/`) && isNoteFile(path.slice(NOTES.length + 1));
  const added = rows.filter(([s, p]) => s === "A" && isNote(p));
  for (const [s, p] of rows) {
    if (s !== "A" && isNote(p)) problems.push(`${p} is already on the base branch; add a new note instead of changing it`);
  }
  const shipped = rows.filter(([, p]) => SHIPPED.some((prefix) => p.startsWith(prefix))).map(([, p]) => p);
  if (shipped.length > 0 && added.length === 0) {
    problems.push(
      `this pull request changes what the crate ships (${shipped.slice(0, 3).join(", ")}${shipped.length > 3 ? ", ..." : ""}) and adds no note under ${NOTES}/; add one with its crate tier (see ${NOTES}/README.md)`,
    );
  }
}

if (problems.length > 0) {
  for (const p of problems) console.error(`::error::${p}`);
  process.exit(1);
}
console.log("change notes: OK");
