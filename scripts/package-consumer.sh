#!/usr/bin/env sh
# Installs a packed engine tarball (scripts/pack-engine.sh) into an empty
# project and uses it there, as a game that downloads the package would: the
# main for Node from require() and import, the tooling build, both browser
# builds with their WebAssembly, BUILD.json, the conformance suite, and the
# declarations and the shipped event rules under TypeScript. A file the
# package needs but does not carry fails here, where the repository's own
# tests, which read the source tree, cannot see it.
#
#   scripts/package-consumer.sh TARBALL
#
# Run from the repository root after npm ci (it uses the repository's tsc).
set -eu
tarball="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
here="$(pwd)"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

cp scripts/package-consumer/run.mjs scripts/package-consumer/types.ts "$work/"
cd "$work"
printf '{ "name": "engine-package-consumer", "private": true, "type": "module" }\n' > package.json
npm install --no-audit --no-fund --ignore-scripts --no-package-lock "$tarball"
node run.mjs
"$here/node_modules/.bin/tsc" --noEmit --strict --skipLibCheck false --target es2022 --module esnext \
  --moduleResolution bundler --lib es2022,dom,esnext.disposable types.ts
echo "package consumer: the declarations and the event rules type-check"
