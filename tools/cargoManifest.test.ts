import { describe, expect, it } from "vitest";
import { cargoPackage } from "./cargoManifest";

describe("cargoPackage", () => {
  it("reads the [package] name and version, not another table's", () => {
    const toml = [
      "[workspace]",
      'version = "9.9.9"',
      "",
      "[package]",
      'name = "verticopolis-engine"',
      'version = "1.60.1"',
      'edition = "2021"',
      "",
      "[dependencies]",
      'serde = { version = "1" }',
      'name = "not-the-package"',
    ].join("\n");
    expect(cargoPackage(toml)).toEqual({ name: "verticopolis-engine", version: "1.60.1" });
  });

  it("refuses a manifest whose [package] has no name or no version", () => {
    expect(() => cargoPackage('[package]\nversion = "1.0.0"\n')).toThrow("no [package] name");
    expect(() => cargoPackage('[package]\nname = "x"\n')).toThrow("no [package] version");
    expect(() => cargoPackage('[dependencies]\nname = "x"\nversion = "1"\n')).toThrow("no [package] name");
  });
});
