#!/usr/bin/env sh
# Finishes the engine's npm package in DIR, which already holds the built
# node/, web/, tooling/node/ and tooling/web/ trees and conformance/: adds the package's own README, the browser build's
# declarations, the license and the engine changelog, and writes package.json.
# release-tag.yml runs it for each new crate release; the packed tarball goes
# on the v<version> GitHub release, and engine-publish.yml publishes
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
for tree in node web tooling/node tooling/web conformance; do
  if [ ! -d "$dir/$tree" ]; then
    echo "::error::$dir needs the built $tree/ tree first." >&2
    exit 1
  fi
done

cp packages/verticopolis-engine/web.d.ts "$dir/web/verticopolis_engine.d.ts"
cp packages/verticopolis-engine/web.d.ts "$dir/tooling/web/verticopolis_engine.d.ts"
# require("@verticopolis/engine/tooling") loads the tooling build for Node.
printf '{\n  "main": "node/verticopolis_engine.js",\n  "types": "node/verticopolis_engine.d.ts"\n}\n' > "$dir/tooling/package.json"
cp packages/verticopolis-engine/README.md "$dir/README.md"
cp "$changelog" "$dir/CHANGELOG.md"
cp LICENSE "$dir/LICENSE"

# The package is defined by the repository root's package.json; the version
# comes from the release (the crate's), so no package.json version is ever
# edited by hand. The root's scripts and dev dependencies build the package
# and do not ship in it.
VERSION="$version" DIR="$dir" node -e '
  const fs = require("fs");
  const pkg = JSON.parse(fs.readFileSync("package.json", "utf8"));
  pkg.version = process.env.VERSION;
  delete pkg.scripts;
  delete pkg.devDependencies;
  fs.writeFileSync(`${process.env.DIR}/package.json`, JSON.stringify(pkg, null, 2) + "\n");'
echo "@verticopolis/engine"
