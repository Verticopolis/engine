/**
 * Reads the crate's `Cargo.toml`, for the package build (`scripts/wasm-build.ts`),
 * which records the crate's name and version in the package's build record.
 *
 * Node only; never imported by the engine package.
 */

/** The `[package]` name and version of `Cargo.toml`. */
export function cargoPackage(cargoToml: string): {
  name: string;
  version: string;
} {
  const pkg = /\[package\]([\s\S]*?)(?:\n\s*\[|$)/.exec(cargoToml)?.[1] ?? "";
  const field = (key: string) => new RegExp(`^${key}\\s*=\\s*"([^"]+)"`, "m").exec(pkg)?.[1];
  const name = field("name");
  const version = field("version");
  if (!name) throw new Error("Cargo.toml has no [package] name");
  if (!version) throw new Error("Cargo.toml has no [package] version");
  return { name, version };
}
