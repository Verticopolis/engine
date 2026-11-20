/**
 * Cuts an engine release from the `## Unreleased` lines of
 * `engine-rs/CHANGELOG.md` (CONTRIBUTING.md "Releasing"). The Cut release
 * workflow runs it on main; it also runs by hand on a clean checkout:
 *
 *   node scripts/cut-release.ts                   the tier the entries ask for
 *   node scripts/cut-release.ts --version 2.0.0   a named version, at or past it
 *
 * It raises the crate version in `engine-rs/Cargo.toml`, the three lockfiles
 * that name the crate and `engine-rs/src/compat-record.json`, renames
 * `## Unreleased` to `## <version> (<date>)` and moves the quoted version pins
 * in the guides. The rules are in `src/build/releaseCut.ts`. It prints the new
 * version on its last line.
 */
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import {
  crateVersion,
  movePins,
  nextVersion,
  renameUnreleased,
  setCrateVersion,
  setLockedVersion,
  setRecordVersion,
  unreleasedTier,
} from "../src/build/releaseCut.ts";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel: string) => readFileSync(join(root, rel), "utf8");
const write = (rel: string, text: string) => writeFileSync(join(root, rel), text);

const CARGO = "engine-rs/Cargo.toml";
const CHANGELOG = "engine-rs/CHANGELOG.md";
const RECORD = "engine-rs/src/compat-record.json";
const LOCKFILES = ["Cargo.lock", "engine-rs/consumer/Cargo.lock", "engine-bench/Cargo.lock"];
const GUIDES = ["docs/engine-site/guides/crate.md", "docs/engine-site/guides/javascript.md"];

function arg(name: string): string | undefined {
  const i = process.argv.indexOf(name);
  return i >= 0 ? process.argv[i + 1] : undefined;
}

try {
  const changelog = read(CHANGELOG);
  const tier = unreleasedTier(changelog);
  const was = crateVersion(read(CARGO));
  const version = nextVersion(was, tier, arg("--version"));
  const date = new Date().toISOString().slice(0, 10);

  write(CHANGELOG, renameUnreleased(changelog, version, date));
  write(CARGO, setCrateVersion(read(CARGO), version));
  for (const lock of LOCKFILES) write(lock, setLockedVersion(read(lock), was, version));
  write(RECORD, setRecordVersion(read(RECORD), was, version));
  for (const guide of GUIDES) {
    const { text, moved } = movePins(read(guide), was, version);
    if (moved === 0) console.warn(`cut-release: ${guide} pins no "${was}"; check it by hand`);
    write(guide, text);
  }

  console.log(`cut-release: engine ${was} -> ${version} (${arg("--version") ? "named" : tier}), changelog section dated ${date}`);
  console.log(version);
} catch (e) {
  console.error(`cut-release: ${e instanceof Error ? e.message : String(e)}`);
  process.exit(1);
}
