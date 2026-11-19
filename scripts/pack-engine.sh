#!/usr/bin/env sh
# Packs a built engine (npm run wasm:build) into the one tarball a release
# carries, verticopolis-engine-VERSION.tgz (@verticopolis/engine), with its
# SHA-256 checksum beside it, in OUT (default dist/). The package holds:
#
# - node/ for Node and web/ for the browser: the player build, without the
#   binding's tier 3 ops, the build a game ships;
# - events/: the gameplay event types and the bridge's rules;
# - tooling/: the tooling build with the tier 3 ops, node/ and web/ laid out
#   the same way, for test suites and the screenshot and e2e harnesses
#   (`@verticopolis/engine/tooling`). Never shipped to players;
# - conformance/: the conformance suite as it stands, the scenarios, pinned
#   hashes, fixtures and cases a host replays to prove it runs the real
#   simulation (`@verticopolis/engine/conformance/...`).
#
# A game that imports only the player build bundles nothing from tooling/ or
# conformance/; they only make the install bigger.
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

# The package. engine-package-files.sh writes its package.json, README,
# changelog and license, and the browser builds' declarations.
mkdir -p "$work/player/node" "$work/player/web" "$work/player/events" \
  "$work/player/tooling/node" "$work/player/tooling/web" "$work/player/conformance"
cp -R engine-rs/pkg-player/. "$work/player/node/"
cp -R engine-rs/pkg-web/. "$work/player/web/"
cp -R engine-rs/pkg-events/. "$work/player/events/"
cp -R engine-rs/pkg/. "$work/player/tooling/node/"
cp -R engine-rs/pkg-web-tooling/. "$work/player/tooling/web/"
cp -R conformance/. "$work/player/conformance/"
sh scripts/engine-package-files.sh "$work/player" "${version%%-*}" "$repo" > /dev/null
# A pull request's or main's build carries its prerelease suffix.
VERSION="$version" DIR="$work/player" node -e '
  const fs = require("fs");
  const file = `${process.env.DIR}/package.json`;
  const pkg = JSON.parse(fs.readFileSync(file, "utf8"));
  pkg.version = process.env.VERSION;
  fs.writeFileSync(file, JSON.stringify(pkg, null, 2) + "\n");'
pack "$work/player"
