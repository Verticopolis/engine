/**
 * Compile the package's TypeScript layer into an engine package folder DIR
 * (scripts/pack-engine.sh calls it with the folder it packs):
 *
 * - model/, catalog/, binding/, port/, host/ and testing/: ESM JavaScript and
 *   .d.ts files from packages/verticopolis-engine/src, through
 *   packages/verticopolis-engine/tsconfig.build.json (tests and their
 *   fixture modules stay out);
 * - fixtures/: the engine's JSON fixtures (scripts/layer-fixtures.ts);
 * - events/: gameplayCatalog and the generated gameplayEventRules compiled to
 *   JavaScript and .d.ts, beside gameplayEvents.d.ts and the generated
 *   gameplayEventRules.ts, which the package has always carried.
 *
 *   node scripts/layer-build.ts DIR
 *
 * Needs the tooling build and the event types (npm run wasm:build), which the
 * layer is typechecked against. Run from the repository root after npm ci.
 */
import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync, mkdtempSync, readdirSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const out = process.argv[2];
if (!out) {
  console.error("usage: node scripts/layer-build.ts DIR");
  process.exit(2);
}
const root = resolve(import.meta.dirname, "..");
const dir = resolve(out);
const tsc = (args: string[]) =>
  execFileSync(resolve(root, "node_modules/.bin/tsc"), args, { cwd: root, stdio: "inherit" });

tsc(["-p", "packages/verticopolis-engine/tsconfig.build.json", "--outDir", dir]);

const fixtures = resolve(root, "packages/verticopolis-engine/src/fixtures");
mkdirSync(join(dir, "fixtures"), { recursive: true });
for (const name of readdirSync(fixtures))
  if (name.endsWith(".json")) copyFileSync(join(fixtures, name), join(dir, "fixtures", name));

// The event rules are generated TypeScript; the package ships them compiled too.
const stage = mkdtempSync(join(tmpdir(), "engine-events-"));
try {
  copyFileSync(resolve(root, "src/events/gameplayCatalog.ts"), join(stage, "gameplayCatalog.ts"));
  copyFileSync(resolve(root, "engine-rs/pkg-events/gameplayEventRules.ts"), join(stage, "gameplayEventRules.ts"));
  tsc([
    "--outDir",
    join(dir, "events"),
    "--declaration",
    "--strict",
    "--skipLibCheck",
    "--target",
    "es2023",
    "--module",
    "esnext",
    "--moduleResolution",
    "bundler",
    "--rewriteRelativeImportExtensions",
    "--verbatimModuleSyntax",
    join(stage, "gameplayCatalog.ts"),
    join(stage, "gameplayEventRules.ts"),
  ]);
} finally {
  rmSync(stage, { recursive: true, force: true });
}
console.log(`layer: compiled into ${dir}`);
