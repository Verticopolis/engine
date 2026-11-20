// Runs in a scratch project that installed the packed @verticopolis/engine
// tarball and nothing else (scripts/package-consumer.sh), so it sees the
// package the way a game that downloads it does.
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const fail = (message) => {
  console.error(`package consumer: ${message}`);
  process.exit(1);
};
const identity = (engine, label) => {
  const id = JSON.parse(engine.engineIdentity());
  if (!id.catalogDigests?.classic || !id.catalogDigests?.modern) fail(`${label}: engineIdentity() has no catalog digests`);
  return JSON.stringify(id);
};

// The package's main: the player build for Node, from CommonJS and from ESM.
const player = require("@verticopolis/engine");
const playerId = identity(player, "require main");
const imported = (await import("@verticopolis/engine")).default;
if (identity(imported, "import main") !== playerId) fail("import and require of the main load different engines");
for (const mode of ["classic", "modern"]) {
  const catalog = JSON.parse(player.catalog(mode));
  if (catalog.mode !== mode || !Array.isArray(catalog.facilities) || catalog.facilities.length === 0) fail(`catalog("${mode}") is empty`);
}

// The tooling build for Node, which the README loads with require().
const tooling = require("@verticopolis/engine/tooling");
if (identity(tooling, "tooling") !== playerId) fail("the tooling build reports another engine than the player build");

// The browser builds: ES modules that take the WebAssembly bytes from the host.
for (const dir of ["web", "tooling/web"]) {
  const web = await import(`@verticopolis/engine/${dir}/verticopolis_engine.js`);
  await web.default({ module_or_path: readFileSync(require.resolve(`@verticopolis/engine/${dir}/verticopolis_engine_bg.wasm`)) });
  if (identity(web, dir) !== playerId) fail(`${dir} reports another engine than the Node build`);
  const build = JSON.parse(readFileSync(require.resolve(`@verticopolis/engine/${dir}/BUILD.json`), "utf8"));
  if (!/^[0-9a-f]{64}$/.test(build.sources ?? "")) fail(`${dir}/BUILD.json has no sources hash`);
}

// The conformance suite a host replays.
const expected = JSON.parse(readFileSync(require.resolve("@verticopolis/engine/conformance/expected.json"), "utf8"));
if (!expected.scenarios || Object.keys(expected.scenarios).length === 0) fail("conformance/expected.json has no scenarios");

console.log(`package consumer: ${require("@verticopolis/engine/package.json").version} loads from Node, ESM, the browser builds and the tooling build`);
