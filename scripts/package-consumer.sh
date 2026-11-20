#!/usr/bin/env sh
# Installs a packed engine tarball (scripts/pack-engine.sh) into an empty
# project and uses it there, as a game that downloads the package would: the
# main for Node from require() and import, the tooling build, both browser
# builds with their WebAssembly, BUILD.json, the conformance suite, the
# TypeScript layer (every compiled module, the frame reader on a real engine,
# the test helpers), and the declarations, the layer's included, and the
# shipped event rules under TypeScript. It also fails if the engine names a
# components package in any dependency list. A file the
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
# Every declaration the layer ships, imported by its subpath, so a .d.ts
# that names a module the package does not carry fails here.
layer="$work/node_modules/@verticopolis/engine"
for dir in model catalog binding port host testing events; do
  for f in "$layer/$dir"/*.d.ts; do
    name="$(basename "$f" .d.ts)"
    printf 'export type * as %s_%s from "@verticopolis/engine/%s/%s";\n' "$dir" "$name" "$dir" "$name"
  done
done > layer.ts
"$here/node_modules/.bin/tsc" --noEmit --strict --skipLibCheck false --target es2023 --module esnext \
  --moduleResolution bundler --lib es2023,dom,esnext.disposable --typeRoots "$here/node_modules/@types" --types node types.ts layer.ts
echo "package consumer: the declarations and the event rules type-check"
