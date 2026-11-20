/**
 * The rules behind the Cut release workflow (`.github/workflows/cut-release.yml`,
 * CONTRIBUTING.md "Releasing"): the next version and the text edits a
 * release makes. The change notes it reads are in `changeNotes.ts`.
 * `scripts/cut-release.ts` applies them to the tree.
 *
 * Node only, and ERASABLE so plain `node` runs the script that imports it.
 */

export type Tier = "major" | "minor" | "patch";

const SEMVER = /^(\d+)\.(\d+)\.(\d+)$/;

function parse(version: string): [number, number, number] {
  const m = SEMVER.exec(version);
  if (!m) throw new Error(`"${version}" is not a version like 1.2.3`);
  return [Number(m[1]), Number(m[2]), Number(m[3])];
}

export function bumpVersion(version: string, tier: Tier): string {
  const [major, minor, patch] = parse(version);
  if (tier === "major") return `${major + 1}.0.0`;
  if (tier === "minor") return `${major}.${minor + 1}.0`;
  return `${major}.${minor}.${patch + 1}`;
}

/** True when `b` is a later version than `a`. */
export function isLater(a: string, b: string): boolean {
  const x = parse(a);
  const y = parse(b);
  for (let i = 0; i < 3; i++) if (x[i] !== y[i]) return y[i] > x[i];
  return false;
}

/**
 * The version to release: the tier's bump of `current`, or `named` when the
 * maintainer names one, which must be that bump or later.
 */
export function nextVersion(current: string, tier: Tier, named?: string): string {
  const bumped = bumpVersion(current, tier);
  if (named === undefined || named === "") return bumped;
  parse(named);
  if (named !== bumped && !isLater(bumped, named)) throw new Error(`${named} is not ${bumped} or later (the ${tier} release after ${current})`);
  return named;
}

/**
 * Adds the `## <version> (<date>)` section with `body` above the newest
 * version's section, under the changelog's introduction.
 */
export function addSection(changelog: string, version: string, date: string, body: string): string {
  const lines = changelog.split("\n");
  if (lines.some((l) => l.startsWith(`## ${version} `) || l.trimEnd() === `## ${version}`)) throw new Error(`the changelog already has a ${version} section`);
  if (lines.some((l) => l.trimEnd() === "## Unreleased")) throw new Error(`the changelog still has a "## Unreleased" section; move its entries to .changes/ notes`);
  const first = lines.findIndex((l) => l.startsWith("## "));
  const section = [`## ${version} (${date})`, "", ...body.trimEnd().split("\n"), ""];
  if (first < 0) return [...lines, ...(lines.at(-1) === "" ? [] : [""]), ...section].join("\n");
  return [...lines.slice(0, first), ...section, ...lines.slice(first)].join("\n");
}

/** The `version` of the `[package]` table of a Cargo.toml. */
export function crateVersion(cargoToml: string): string {
  let inPackage = false;
  for (const line of cargoToml.split("\n")) {
    if (/^\[/.test(line)) inPackage = line.trim() === "[package]";
    else if (inPackage) {
      const m = /^version\s*=\s*"([^"]*)"/.exec(line);
      if (m) return m[1];
    }
  }
  throw new Error("Cargo.toml has no [package] version");
}

/** Sets the `version` of the `[package]` table of a Cargo.toml. */
export function setCrateVersion(cargoToml: string, version: string): string {
  const lines = cargoToml.split("\n");
  let inPackage = false;
  for (let i = 0; i < lines.length; i++) {
    if (/^\[/.test(lines[i])) inPackage = lines[i].trim() === "[package]";
    else if (inPackage && /^version\s*=/.test(lines[i])) {
      lines[i] = lines[i].replace(/"[^"]*"/, `"${version}"`);
      return lines.join("\n");
    }
  }
  throw new Error("Cargo.toml has no [package] version");
}

/**
 * Moves the lockfile's one `verticopolis-engine` package from `from` to `to`.
 * Refuses a lockfile whose entry is missing, repeated or at another version,
 * since that lockfile is out of step with the crate.
 */
export function setLockedVersion(lock: string, from: string, to: string): string {
  const entry = /(\[\[package\]\]\nname = "verticopolis-engine"\nversion = ")([^"]*)(")/g;
  const found = [...lock.matchAll(entry)];
  if (found.length !== 1) throw new Error(`expected one verticopolis-engine package in the lockfile, found ${found.length}`);
  if (found[0][2] !== from) throw new Error(`the lockfile has verticopolis-engine ${found[0][2]}, not ${from}`);
  return lock.replace(entry, `$1${to}$3`);
}

/** Moves the compatibility record's `crateVersion` from `from` to `to`. */
export function setRecordVersion(record: string, from: string, to: string): string {
  const pin = `"crateVersion": "${from}"`;
  if (record.split(pin).length !== 2) throw new Error(`compat-record.json does not hold ${pin} once`);
  return record.replace(pin, `"crateVersion": "${to}"`);
}

/**
 * Moves the quoted version pins in a guide (`"1.2.3"`, `"^1.2.3"`,
 * `"=1.2.3"`) from `from` to `to`. Unquoted mentions are history and stay.
 * Returns the text and how many pins moved.
 */
export function movePins(text: string, from: string, to: string): { text: string; moved: number } {
  const escaped = from.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const pin = new RegExp(`"([=^]?)${escaped}"`, "g");
  let moved = 0;
  const out = text.replace(pin, (_m, prefix: string) => {
    moved++;
    return `"${prefix}${to}"`;
  });
  return { text: out, moved };
}
