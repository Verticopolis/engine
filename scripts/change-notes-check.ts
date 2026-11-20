/**
 * Checks the change notes (`.changes/README.md`) on a pull request:
 *
 *   node scripts/change-notes-check.ts --base origin/main
 *
 * Every note under `.changes/` must parse, and notes sit directly in
 * `.changes/`, never in a subfolder. With `--base`, a pull request:
 *
 * - that changes what the crate or the npm package ships (`isShipped`) must
 *   add a note;
 * - that changes Rust code under `src/` other than its tests must
 *   add a note with a tier of patch or higher, never only `crate: none`;
 * - must not edit or delete a note already on the base branch, which belongs
 *   to the pull request that merged it;
 * - must not change the version sections of `CHANGELOG.md` (from
 *   the first `## <version>` heading down) or add an `## Unreleased` one, nor
 *   change the crate version: only a release PR does that. The introduction
 *   above the sections is documentation and may change.
 *
 * Release branches (`release/`) and Dependabot's (`dependabot/`) are exempt,
 * the way the game's version check exempts them: the release PR deletes the
 * notes and writes the changelog, and Dependabot cannot write a note. The
 * branch name comes from GITHUB_HEAD_REF.
 */
import { execFileSync } from "node:child_process";
import { readdirSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { isNoteFile, isRustCode, isShipped, parseNote, type ChangeNote } from "../tools/changeNotes.ts";
import { crateVersion } from "../tools/releaseCut.ts";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const NOTES = ".changes";
const CHANGELOG = "CHANGELOG.md";
const CARGO = "Cargo.toml";
const EXEMPT_BRANCHES = ["release/", "dependabot/"];

function arg(name: string): string | undefined {
  const i = process.argv.indexOf(name);
  return i >= 0 ? process.argv[i + 1] : undefined;
}

const git = (...args: string[]) => execFileSync("git", args, { cwd: root, encoding: "utf8" });

/**
 * The crate's file `path` as the base branch has it. A base from before the
 * crate moved to the repository root keeps it under engine-rs/ (the root
 * Cargo.toml was then a workspace with no [package]), so `read` gets the
 * first copy it accepts.
 */
function onBase<T>(base: string, path: string, read: (text: string) => T): T {
  let first: unknown;
  for (const at of [path, `engine-rs/${path}`]) {
    try {
      return read(execFileSync("git", ["show", `${base}:${at}`], { cwd: root, encoding: "utf8", stdio: "pipe" }));
    } catch (e) {
      first ??= e;
    }
  }
  throw first;
}

const problems: string[] = [];
const notes = new Map<string, ChangeNote>();
for (const entry of readdirSync(join(root, NOTES), { withFileTypes: true })) {
  if (entry.isDirectory()) {
    problems.push(`${NOTES}/${entry.name}/ is a folder; notes sit directly in ${NOTES}/`);
    continue;
  }
  if (!isNoteFile(entry.name)) continue;
  try {
    notes.set(entry.name, parseNote(entry.name, readFileSync(join(root, NOTES, entry.name), "utf8")));
  } catch (e) {
    problems.push(e instanceof Error ? e.message : String(e));
  }
}

const base = arg("--base");
const branch = process.env.GITHUB_HEAD_REF ?? "";
if (base && !EXEMPT_BRANCHES.some((p) => branch.startsWith(p))) {
  const rows = git("diff", "--name-status", "--no-renames", `${base}...HEAD`)
    .split("\n")
    .filter(Boolean)
    .map((l) => l.split("\t") as [string, string]);
  const noteName = (path: string) => (path.startsWith(`${NOTES}/`) ? path.slice(NOTES.length + 1) : undefined);
  const added: ChangeNote[] = [];
  for (const [status, path] of rows) {
    const name = noteName(path);
    if (name === undefined || !isNoteFile(name) || name.includes("/")) continue;
    if (status !== "A") problems.push(`${path} is already on the base branch; add a new note instead of changing it`);
    else {
      const note = notes.get(name);
      if (note) added.push(note);
    }
  }

  const shipped = rows.filter(([, p]) => isShipped(p)).map(([, p]) => p);
  const list = (paths: string[]) => `${paths.slice(0, 3).join(", ")}${paths.length > 3 ? ", ..." : ""}`;
  if (shipped.length > 0 && added.length === 0) {
    problems.push(
      `this pull request changes what the crate ships (${list(shipped)}) and adds no note under ${NOTES}/; add one with its crate tier (see ${NOTES}/README.md)`,
    );
  }
  const rust = rows.filter(([, p]) => isRustCode(p)).map(([, p]) => p);
  if (rust.length > 0 && added.length > 0 && added.every((n) => n.tier === "none")) {
    problems.push(
      `this pull request changes Rust code (${list(rust)}), so its note needs a crate tier of patch or higher, not none`,
    );
  }

  if (rows.some(([, p]) => p === CHANGELOG)) {
    const sections = (text: string) => text.slice(Math.max(0, text.search(/^## \d/m)));
    const now = readFileSync(join(root, CHANGELOG), "utf8");
    if (/^## Unreleased\s*$/m.test(now) || sections(now) !== onBase(base, CHANGELOG, sections)) {
      problems.push(
        `only a release PR writes the version sections of ${CHANGELOG}; put the entries in a note under ${NOTES}/`,
      );
    }
  }
  if (rows.some(([, p]) => p === CARGO)) {
    const before = onBase(base, CARGO, crateVersion);
    const after = crateVersion(readFileSync(join(root, CARGO), "utf8"));
    if (before !== after)
      problems.push(`only a release PR changes the crate version (${before} -> ${after} in ${CARGO})`);
  }
}

if (problems.length > 0) {
  for (const p of problems) console.error(`::error::${p}`);
  process.exit(1);
}
console.log("change notes: OK");
