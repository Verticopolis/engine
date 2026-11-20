/**
 * Cuts an engine release from the change notes in `.changes/`
 * (CONTRIBUTING.md "Releasing", `.changes/README.md`). The Cut release
 * workflow runs it on main; it also runs by hand on a clean checkout:
 *
 *   node scripts/cut-release.ts                   the tier the notes ask for
 *   node scripts/cut-release.ts --version 2.0.0   a named version, at or past it
 *
 * It raises the crate version in `Cargo.toml`, the three lockfiles
 * that name the crate and `src/compat-record.json`, writes the
 * notes' entries into a new `## <version> (<date>)` changelog section, deletes
 * the notes and moves the quoted version pins in the guides. The rules are in
 * `tools/releaseCut.ts` and `tools/changeNotes.ts`. It prints the new
 * version on its last line.
 */
import { readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { highestTier, isNoteFile, parseNote, sectionBody } from "../tools/changeNotes.ts";
import {
  addSection,
  crateVersion,
  movePins,
  nextVersion,
  setCrateVersion,
  setLockedVersion,
  setRecordVersion,
} from "../tools/releaseCut.ts";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel: string) => readFileSync(join(root, rel), "utf8");
const write = (rel: string, text: string) => writeFileSync(join(root, rel), text);

const CARGO = "Cargo.toml";
const CHANGELOG = "CHANGELOG.md";
const NOTES = ".changes";
const RECORD = "src/compat-record.json";
const LOCKFILES = ["Cargo.lock", "consumer/Cargo.lock", "engine-bench/Cargo.lock"];
const GUIDES = ["docs/engine-site/guides/crate.md", "docs/engine-site/guides/javascript.md"];

function arg(name: string): string | undefined {
  const i = process.argv.indexOf(name);
  return i >= 0 ? process.argv[i + 1] : undefined;
}

try {
  const files = readdirSync(join(root, NOTES)).filter(isNoteFile).sort();
  const notes = files.map((f) => parseNote(f, read(join(NOTES, f))));
  const tier = highestTier(notes);
  if (tier === undefined)
    throw new Error(`no note in ${NOTES}/ names a tier above none, so there is nothing to release`);
  const was = crateVersion(read(CARGO));
  const version = nextVersion(was, tier, arg("--version"));
  const date = new Date().toISOString().slice(0, 10);

  write(CHANGELOG, addSection(read(CHANGELOG), version, date, sectionBody(notes)));
  for (const f of files) rmSync(join(root, NOTES, f));
  write(CARGO, setCrateVersion(read(CARGO), version));
  for (const lock of LOCKFILES) write(lock, setLockedVersion(read(lock), was, version));
  write(RECORD, setRecordVersion(read(RECORD), was, version));
  for (const guide of GUIDES) {
    const { text, moved } = movePins(read(guide), was, version);
    if (moved === 0) console.warn(`cut-release: ${guide} pins no "${was}"; check it by hand`);
    write(guide, text);
  }

  console.log(
    `cut-release: engine ${was} -> ${version} (${arg("--version") ? "named" : tier}), ${files.length} note(s) into the changelog section dated ${date}`,
  );
  console.log(version);
} catch (e) {
  console.error(`cut-release: ${e instanceof Error ? e.message : String(e)}`);
  process.exit(1);
}
