#!/usr/bin/env sh
# Packs a built engine (npm run wasm:build) into the three tarballs a release
# carries, each with its SHA-256 checksum beside it, in OUT (default dist/):
#
# - verticopolis-engine-VERSION.tgz, @verticopolis/engine: the player build,
#   node/ for Node and web/ for the browser, without the binding's tier 3 ops,
#   plus events/ (the gameplay event types and the bridge's rules). This is
#   the package a game ships, and the one npm will carry.
# - verticopolis-engine-tooling-VERSION.tgz, @verticopolis/engine-tooling:
#   the tooling build with the tier 3 ops, laid out the same way, for test
#   suites and the screenshot and e2e harnesses. Never shipped to players.
# - verticopolis-engine-conformance-VERSION.tgz,
#   @verticopolis/engine-conformance: conformance/ as it stands, the scenarios,
#   pinned hashes, fixtures and cases a host replays to prove it runs the real
#   simulation.
#
#   scripts/pack-engine.sh VERSION [OUT]
#
# VERSION is the crate version (x.y.z). Run from the repository root.
set -eu
version="$1"
out="${2:-dist}"
repo="Verticopolis/engine"

if ! printf '%s' "$version" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$'; then
  echo "::error::Engine version $version is not x.y.z or x.y.z-pre." >&2
  exit 1
fi
for dir in engine-rs/pkg-player engine-rs/pkg-web engine-rs/pkg engine-rs/pkg-web-tooling engine-rs/pkg-events; do
  if [ ! -d "$dir" ]; then
    echo "::error::$dir is missing; run npm run wasm:build first." >&2
    exit 1
  fi
done

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
mkdir -p "$out"
out="$(cd "$out" && pwd)"

# pack DIR: npm pack DIR into OUT and write the checksum beside it.
pack() {
  tgz="$(cd "$1" && npm pack --pack-destination "$out" --json | node -e 'process.stdout.write(JSON.parse(require("fs").readFileSync(0, "utf8"))[0].filename)')"
  (cd "$out" && sha256sum "$tgz" > "$tgz.sha256")
  echo "$out/$tgz"
}

# The player package. engine-package-files.sh writes its package.json, README,
# changelog and license, and the browser build's declarations.
mkdir -p "$work/player/node" "$work/player/web" "$work/player/events"
cp -R engine-rs/pkg-player/. "$work/player/node/"
cp -R engine-rs/pkg-web/. "$work/player/web/"
cp -R engine-rs/pkg-events/. "$work/player/events/"
sh scripts/engine-package-files.sh "$work/player" "${version%%-*}" "$repo" > /dev/null
# A pull request's or main's build carries its prerelease suffix.
VERSION="$version" DIR="$work/player" node -e '
  const fs = require("fs");
  const file = `${process.env.DIR}/package.json`;
  const pkg = JSON.parse(fs.readFileSync(file, "utf8"));
  pkg.version = process.env.VERSION;
  fs.writeFileSync(file, JSON.stringify(pkg, null, 2) + "\n");'
pack "$work/player"

# The tooling package: the same layout from the tooling build.
mkdir -p "$work/tooling/node" "$work/tooling/web" "$work/tooling/events"
cp -R engine-rs/pkg/. "$work/tooling/node/"
cp -R engine-rs/pkg-web-tooling/. "$work/tooling/web/"
cp -R engine-rs/pkg-events/. "$work/tooling/events/"
cp packages/verticopolis-engine/web.d.ts "$work/tooling/web/verticopolis_engine.d.ts"
cp LICENSE "$work/tooling/LICENSE"
cat > "$work/tooling/README.md" <<'README'
# @verticopolis/engine-tooling

The Verticopolis engine's tooling build: the same simulation as
`@verticopolis/engine`, plus the binding's tier 3 ops that test suites and the
screenshot and e2e harnesses use to set up and inspect a tower. A game never
ships it to players. `node/` is the build for Node, `web/` the build for the
browser and `events/` the gameplay event types. See
https://docs.verticopolis.com/javascript/.
README
VERSION="$version" DIR="$work/tooling" node -e '
  const fs = require("fs");
  const pkg = {
    name: "@verticopolis/engine-tooling",
    version: process.env.VERSION,
    description: "The Verticopolis engine with the tooling ops test suites and harnesses use. Not for shipping.",
    license: "MIT",
    repository: { type: "git", url: "git+https://github.com/Verticopolis/engine.git" },
    main: "node/verticopolis_engine.js",
    types: "node/verticopolis_engine.d.ts",
    files: ["node", "web", "events"],
    engines: { node: ">=22" },
  };
  fs.writeFileSync(`${process.env.DIR}/package.json`, JSON.stringify(pkg, null, 2) + "\n");'
pack "$work/tooling"

# The conformance package: conformance/ as it stands.
mkdir -p "$work/conformance"
cp -R conformance/. "$work/conformance/"
cp LICENSE "$work/conformance/LICENSE"
VERSION="$version" DIR="$work/conformance" node -e '
  const fs = require("fs");
  const pkg = {
    name: "@verticopolis/engine-conformance",
    version: process.env.VERSION,
    description: "The Verticopolis conformance suite: scenarios, pinned checkpoint hashes, fixtures and cases.",
    license: "MIT",
    repository: { type: "git", url: "git+https://github.com/Verticopolis/engine.git", directory: "conformance" },
  };
  fs.writeFileSync(`${process.env.DIR}/package.json`, JSON.stringify(pkg, null, 2) + "\n");'
pack "$work/conformance"
