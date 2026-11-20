import { checkBinding, type WasmModule } from "../binding/binding.ts";
import { readCatalog } from "../binding/catalog.ts";
import type { Catalog } from "../catalog/catalogTypes.ts";
import type { GameMode } from "../model/types.ts";
import type { EnginePort } from "../port/enginePort.ts";
import { EngineBootError } from "./bootError.ts";
import { ENGINE_DIR, ENGINE_GLUE } from "./enginePackageNames.ts";
import { loadEngineModule, type LoadedEngine, type LoadEngineOptions } from "./loadEngineModule.ts";

/**
 * The Rust engine as the game's engine port (PR 5e). The loaded package's
 * `Engine` is the port itself: the app steps it, commands it and reads its
 * view through the same calls it makes on the TypeScript engine, with no
 * TypeScript copy of the tower beside it.
 */

/** Where the built package lives: `engine/` under the app's `base` (a game
 *  serves the package's `web/` folder there and passes its own base, Vite's
 *  `import.meta.env.BASE_URL` in the web game). Resolved against the page, so
 *  a relocated deployment finds it under its own base. Every file the package
 *  loads at run time is listed in engineAssets.ts. */
export function enginePackageUrl(base: string, href = globalThis.location?.href ?? "http://localhost/"): string {
  return new URL(`${base}${ENGINE_DIR}${ENGINE_GLUE}`, href).href;
}

/** How {@link loadWasmEngine} loads: the byte check's options, and whether
 *  this is a tooling build, which also needs the tier 3 ops. `loader` is
 *  {@link loadEngineModule} unless the frontend's dev branch passes
 *  `loadEngineUnchecked` (as `import.meta.env.DEV ? loadEngineUnchecked :
 *  undefined`, so a production build drops it). */
export interface LoadWasmOptions extends LoadEngineOptions {
  tooling: boolean;
  loader?: (url: string, opts: LoadEngineOptions) => Promise<LoadedEngine>;
}

/** Load and initialize the browser package through the byte check
 *  (loadEngineModule). A package that passes it but lacks what this shell
 *  calls is a mismatched package too (a shell and an engine from different
 *  builds), coded so analytics tells it from a broken engine. */
export async function loadWasmEngine(url: string, opts: LoadWasmOptions): Promise<WasmModule> {
  const { tooling, loader = loadEngineModule, ...load } = opts;
  const { mod } = await loader(url, load);
  try {
    return checkBinding(mod, { tooling });
  } catch (e) {
    throw new EngineBootError("package-mismatch", e instanceof Error ? e.message : String(e));
  }
}

/** How the host starts its first tower: the seed of the fresh tower, and
 *  `boot`, which loads the stored tower into that fresh engine's port (the
 *  game passes `BOOT_SEED` and `bootTower`). `checkCatalogs` is the app's own
 *  check that it can show every kind the catalogs list (components'
 *  `refuseForeignCatalogs`), and `useCatalogs` hands the catalogs to the
 *  app's renderer for the port (components' `useTowerSource`). */
export interface RustBoot<T> {
  seed: number;
  boot: (port: EnginePort) => T;
  checkCatalogs?: (catalogs: Readonly<Record<GameMode, Catalog>>) => void;
  useCatalogs: (port: EnginePort, catalogs: Readonly<Record<GameMode, Catalog>>) => void;
}

/** A fresh Rust engine, handed to `host.boot` to load the stored tower into.
 *  Prices, sizes and build rules come from the engine's own catalogs, read
 *  first: a package whose catalogs this build cannot read, that list other
 *  kinds than this build draws, or whose two modes state different worlds, is
 *  a mismatched package and is refused before an engine is built. */
export function rustBootTower<T>(mod: WasmModule, host: RustBoot<T>): T {
  let catalogs: Record<GameMode, Catalog>;
  try {
    // Both modes: a Classic tower's patronage readout uses the Modern yardstick.
    catalogs = { classic: readCatalog(mod, "classic"), modern: readCatalog(mod, "modern") };
    host.checkCatalogs?.(catalogs);
  } catch (e) {
    throw new EngineBootError("package-mismatch", e instanceof Error ? e.message : String(e));
  }
  const port: EnginePort = mod.Engine.newGame(host.seed, "classic");
  host.useCatalogs(port, catalogs);
  return host.boot(port);
}

/** What `window.__vcEngine` says while the Rust engine runs the game: the
 *  version lines and Copy diagnostics read `status`, and tooling reaches the
 *  engine through `port`. A trap that stops the game turns `hosted` off with
 *  the reason in `lastError`. */
export interface RustEngineHandle {
  status: { engine: "wasm"; hosted: boolean; lastError: string | null };
  port: EnginePort;
}

/** Say that the Rust engine runs the game. Published before the app is
 *  built, so the title screen's version line reads it as it renders. */
export function publishRustEngine(
  port: EnginePort,
  g: { __vcEngine?: RustEngineHandle } = globalThis as { __vcEngine?: RustEngineHandle },
): RustEngineHandle {
  const handle: RustEngineHandle = { status: { engine: "wasm", hosted: true, lastError: null }, port };
  g.__vcEngine = handle;
  return handle;
}
