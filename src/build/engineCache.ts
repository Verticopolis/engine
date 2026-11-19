/**
 * What CI's cached copy of the built engine packages is keyed on
 * (.github/actions/engine-wasm), so a push that leaves the engine alone
 * reuses the packages instead of installing Rust and compiling again.
 *
 * Node only (the build scripts and CI).
 */
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

/**
 * The files the package's build runs or checks it with, beside the crate
 * itself: the scripts that build it and write its third-party record (#955)
 * and every module they import, so a change to what the build checks rebuilds
 * instead of restoring a package the old checks passed.
 */
export const ENGINE_BUILD_FILES = [
  "scripts/wasm-build.ts",
  "scripts/engine-third-party.ts",
  "scripts/license-files.ts",
  "src/build/thirdPartyNotices.ts",
  "src/build/rustNotices.ts",
  "src/wasmhost/packageHash.ts",
  "src/wasmhost/engineAssets.ts",
  "src/wasmhost/enginePackageNames.ts",
  "src/wasmhost/enginePackageRecord.ts",
  "src/wasmhost/buildSection.ts",
  "src/build/engineCompat.ts",
  "src/events/gameplayCatalog.ts",
  "scripts/build-engine-wasm.sh",
  "scripts/pack-engine.sh",
  "scripts/engine-package-files.sh",
  "src/build/engineCache.ts",
] as const;

/**
 * The cache key for a package built from `sources` (engineSourceHash of the
 * crate): `prefix`, the source hash, and 16 hex characters of a SHA-256 over
 * the build files, then `extraFiles` (the caller's own cache logic, which
 * decides what a hit means), then the `wasm:build` npm script line.
 */
export function engineCacheKey(root: string, sources: string, prefix: string, extraFiles: readonly string[] = []): string {
  const hash = createHash("sha256");
  for (const file of [...ENGINE_BUILD_FILES, ...extraFiles]) hash.update(readFileSync(resolve(root, file)));
  const scripts = (JSON.parse(readFileSync(resolve(root, "package.json"), "utf8")) as { scripts?: Record<string, string> }).scripts ?? {};
  hash.update(`${scripts["wasm:build"] ?? ""}\n`);
  return `${prefix}-${sources}-${hash.digest("hex").slice(0, 16)}`;
}
