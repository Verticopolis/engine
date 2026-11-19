#!/usr/bin/env sh
# Finishes the engine's npm package in DIR, which already holds the built
# node/ and web/ trees: adds the package's own README, the browser build's
# declarations, the license and the engine changelog, and writes package.json.
# release-tag.yml runs it for each new crate release; the packed tarball goes
# on the engine-v<version> GitHub release, and engine-publish.yml publishes
# that same tarball to npm as @verticopolis/engine.
#
#   scripts/engine-package-files.sh DIR VERSION REPO [CHANGELOG]
#
# CHANGELOG defaults to engine-rs/CHANGELOG.md. Run from the repository root.
set -eu
dir="$1"
version="$2"
repo="$3"
changelog="${4:-engine-rs/CHANGELOG.md}"

if ! printf '%s' "$version" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$'; then
  echo "::error::Engine version $version is not a plain x.y.z." >&2
  exit 1
fi
if [ ! -d "$dir/node" ] || [ ! -d "$dir/web" ]; then
  echo "::error::$dir needs the built node/ and web/ trees first." >&2
  exit 1
fi

cp packages/verticopolis-engine/web.d.ts "$dir/web/verticopolis_engine.d.ts"
cp packages/verticopolis-engine/README.md "$dir/README.md"
cp "$changelog" "$dir/CHANGELOG.md"
cp LICENSE "$dir/LICENSE"

# The package is defined by the repository root's package.json; the version
# comes from the release (the crate's), so no package.json version is ever
# edited by hand.
VERSION="$version" DIR="$dir" node -e '
  const fs = require("fs");
  const pkg = JSON.parse(fs.readFileSync("package.json", "utf8"));
  pkg.version = process.env.VERSION;
  fs.writeFileSync(`${process.env.DIR}/package.json`, JSON.stringify(pkg, null, 2) + "\n");'
echo "@verticopolis/engine"
