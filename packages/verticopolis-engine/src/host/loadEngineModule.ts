import type { BuildMetadataResult } from "./buildSection.ts";
import { compileEngineBytes, initEngineGlue, type EngineLoadOptions } from "./engineLoad.ts";
import { DEV_SKIP_MARKER, ENGINE_BUILD_RECORD, ENGINE_WASM } from "./enginePackageNames.ts";
import {
  browserDigest,
  EnginePackageAborted,
  isControlled,
  verifyEnginePackage,
  type PackageUrls,
  type VerifyOptions,
} from "./enginePackageCheck.ts";
import type { ExpectedEnginePackage } from "./enginePackageRecord.ts";

/**
 * The one place that turns the served engine package into a running module
 * (#985 criterion 5). The main-thread boot, the engine worker and the dual
 * run's worker all load through it, so no route runs bytes the shell was not
 * built for. A development serve, which skips the check, loads through
 * {@link loadEngineUnchecked} instead, from the frontend's own dev branch,
 * which its production build drops. Design:
 * _bmad-output/planning-artifacts/design/
 * design-engine-package-set-consistency-2026-10-15.md, section 5.3.
 *
 * It fetches `BUILD.json`, the glue and the binary once each, refuses unless
 * they are the package this shell was built with (`verifyEnginePackage`),
 * compiles the hashed binary (reading the build section the build stamped into
 * it, #963), imports the glue from a Blob made of the very bytes that were
 * hashed, and hands the compiled module to the glue's `init`. Nothing is fetched a second
 * time, so a cache that changes between the check and the run cannot matter.
 *
 * Permanent host infrastructure: it is the load path of the Rust engine in
 * every build, and nothing in it depends on the TypeScript engine.
 */

/** What the wasm-bindgen glue exports: `default` is its `init`. */
export interface EngineGlue {
  default(input?: { module_or_path: WebAssembly.Module }): Promise<{ memory?: WebAssembly.Memory } | undefined>;
}

export interface LoadedEngine {
  /** The glue's module namespace, ready for `checkBinding`. */
  mod: EngineGlue;
  /** What `init` returned: the instance's exports, for the linear memory. */
  exports: { memory?: WebAssembly.Memory } | undefined;
  /** Always true here; a development serve's own loader reports false. */
  verified: boolean;
}

export interface LoadEngineOptions {
  /** Aborted when the caller's deadline passes. Checked after every await. */
  signal?: AbortSignal;
  /** The package the shell was built with: the record the build compiled in
   *  (the web game's `__ENGINE_PACKAGE__`), or null for a shell built without
   *  one, which the check then reads as unverifiable. */
  expected: ExpectedEnginePackage | null;
  /** Called with what the binary's build section said, as soon as the verified
   *  binary has compiled, so a glue that then fails to import or link still
   *  leaves it (#963). */
  onBuild?: (build: BuildMetadataResult) => void;
  // The rest are test seams; the browser supplies each by default.
  fetch?: VerifyOptions["fetch"];
  digest?: VerifyOptions["digest"];
  controlled?: boolean;
  origin?: string;
  importModule?: (url: string) => Promise<EngineGlue>;
  compile?: (bytes: Uint8Array) => Promise<{ module: WebAssembly.Module; build: BuildMetadataResult }>;
  objectUrls?: Pick<typeof URL, "createObjectURL" | "revokeObjectURL">;
}

/** The URLs of the three files, from the URL the glue is served at. */
export function enginePackageUrls(glue: string): PackageUrls {
  return { glue, wasm: new URL(ENGINE_WASM, glue).href, record: new URL(ENGINE_BUILD_RECORD, glue).href };
}

const importByUrl = (url: string) => import(/* @vite-ignore */ url) as Promise<EngineGlue>;

/**
 * Load and initialize the engine package served at `glueUrl`. Rejects with an
 * `EnginePackageError` for a refusal (see {@link verifyEnginePackage}), or
 * `EnginePackageAborted` once `signal` fires and a step then settles OK (its
 * result is dropped). A step that fails after the signal fired keeps its own
 * error; the caller that set the deadline has already given up on the load.
 * A step already running (the compile inside `init`) cannot be cancelled, so
 * a late engine can finish and stay in memory with the glue's module record.
 */
export async function loadEngineModule(glueUrl: string, opts: LoadEngineOptions): Promise<LoadedEngine> {
  const importModule = opts.importModule ?? importByUrl;
  const signal = opts.signal ?? new AbortController().signal;
  const gone = () => {
    if (signal.aborted) throw new EnginePackageAborted();
  };
  const { glue, wasm } = await verifyEnginePackage({
    expected: opts.expected,
    urls: enginePackageUrls(glueUrl),
    fetch: opts.fetch ?? ((url, init) => fetch(url, init)),
    digest: "digest" in opts ? opts.digest : browserDigest(),
    signal,
    controlled: opts.controlled ?? isControlled(globalThis.navigator),
    origin: opts.origin ?? globalThis.location.origin,
  });
  gone();
  // Compiled before the glue is imported, as engineLoad.ts does, so the build
  // is known even when the glue then fails to import or to link.
  const binary = await (opts.compile ?? compileEngineBytes)(wasm);
  opts.onBuild?.(binary.build);
  gone();
  const urls = opts.objectUrls ?? URL;
  const blobUrl = urls.createObjectURL(new Blob([glue as BlobPart], { type: "text/javascript" }));
  let mod: EngineGlue;
  try {
    mod = await importModule(blobUrl);
  } finally {
    urls.revokeObjectURL(blobUrl);
  }
  gone();
  const exports = await mod.default({ module_or_path: binary.module });
  gone();
  return { mod, exports, verified: true };
}

/** How {@link loadEngineUnchecked} loads: the dev serve's binary URL and build
 *  callback, and a test seam for the glue loader. */
export interface UncheckedLoadOptions extends EngineLoadOptions {
  initGlue?: typeof initEngineGlue;
}

/**
 * A development serve's load: the served glue and the binary beside it, with
 * no byte check, since `npm run wasm:build` rebuilds the package under a
 * running page. It logs the dev-skip marker. A frontend calls it only inside
 * its own dev branch (Vite's `import.meta.env.DEV`), so a production build
 * drops the call and the marker line with it; the web game's precache check
 * fails the build if the marker stays.
 */
export async function loadEngineUnchecked(glueUrl: string, opts: UncheckedLoadOptions = {}): Promise<LoadedEngine> {
  console.info(`[wasm] ${DEV_SKIP_MARKER}`);
  const { mod, exports } = await (opts.initGlue ?? initEngineGlue)(glueUrl, {
    onBuild: opts.onBuild,
    wasmUrl: opts.wasmUrl,
  });
  return { mod: mod as EngineGlue, exports: exports as LoadedEngine["exports"], verified: false };
}
