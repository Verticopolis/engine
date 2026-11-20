import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { ENGINE_BUILD_FILES, engineCacheKey } from "./engineCache";

let dir: string;
beforeEach(() => {
  dir = mkdtempSync(join(tmpdir(), "vc-engine-cache-"));
});
afterEach(() => {
  rmSync(dir, { recursive: true, force: true });
});

/** A repository root holding every build file and a package.json with the given wasm:build script. */
function writeRoot(root: string, wasmBuild = "node scripts/wasm-build.ts"): void {
  for (const file of [...ENGINE_BUILD_FILES, "extra.ts"]) {
    mkdirSync(dirname(join(root, file)), { recursive: true });
    writeFileSync(join(root, file), `contents of ${file}`);
  }
  writeFileSync(join(root, "package.json"), JSON.stringify({ scripts: { "wasm:build": wasmBuild } }));
}

describe("engineCacheKey", () => {
  it("names the prefix and the sources, and moves when a build file, an extra file or the npm script changes", () => {
    const root = join(dir, "root");
    writeRoot(root);
    const base = engineCacheKey(root, "abc", "test-v1", ["extra.ts"]);
    expect(base).toMatch(/^test-v1-abc-[0-9a-f]{16}$/);
    expect(engineCacheKey(root, "abc", "test-v1", ["extra.ts"])).toBe(base);

    writeFileSync(join(root, "scripts/wasm-build.ts"), "changed");
    const afterBuildFile = engineCacheKey(root, "abc", "test-v1", ["extra.ts"]);
    expect(afterBuildFile).not.toBe(base);

    writeFileSync(join(root, "extra.ts"), "changed");
    const afterExtra = engineCacheKey(root, "abc", "test-v1", ["extra.ts"]);
    expect(afterExtra).not.toBe(afterBuildFile);

    writeFileSync(join(root, "package.json"), JSON.stringify({ scripts: { "wasm:build": "something else" } }));
    expect(engineCacheKey(root, "abc", "test-v1", ["extra.ts"])).not.toBe(afterExtra);
  });

  it("throws when a build file is missing, rather than keying without it", () => {
    const root = join(dir, "root");
    writeRoot(root);
    rmSync(join(root, "tools/engineCompat.ts"));
    expect(() => engineCacheKey(root, "abc", "test-v1")).toThrow();
  });
});
