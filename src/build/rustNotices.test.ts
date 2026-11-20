import { describe, expect, it } from "vitest";
import {
  cargoTreePackages,
  htmlText,
  portEntry,
  removeAll,
  shippedCrates,
  stdLicenseProblems,
  type CargoMetadata,
} from "./rustNotices";

describe("removeAll", () => {
  it("removes a match that only forms once an inner one is gone", () => {
    expect(removeAll("a<!<!-- x -->-- y -->b", /<!--[\s\S]*?-->/g)).toBe("ab");
    expect(removeAll("<scr<script>x</script>ipt>y</script>z", /<(script|style)[^>]*>[\s\S]*?<\/\1>/gi)).toBe("z");
  });

  it("leaves text with no match as it is", () => {
    expect(removeAll("plain", /<[^>]*>/g)).toBe("plain");
  });
});

describe("htmlText", () => {
  it("decodes the page's text and drops its markup", () => {
    expect(htmlText("<p>a &#34;b&#34; &#x3c;c&#62; &lt;d&gt; &amp;lt;</p><br>e")).toBe('a "b" <c> <d> &lt;\n\ne');
    expect(htmlText("<!-- a > b -->kept")).toBe("kept");
  });

  it("drops a comment or script split around another one", () => {
    expect(htmlText("<!<!-- x -->-- y -->kept")).toBe("kept");
    expect(htmlText("<scr<script>x</script>ipt>y</script>kept")).toBe("kept");
  });

  it("refuses markup it cannot read rather than passing it on", () => {
    expect(() => htmlText('<a title="x>y">z</a>')).toThrow(/markup/);
    expect(() => htmlText("<p>&copy; 2026</p>")).toThrow(/&copy;/);
  });
});

// Moved from the game (Verticopolis/web), which kept a copy of this file
// from before the repository split.
describe("the engine's third-party notices (#955, #974, #1356)", () => {
  it("lists the crates a WASM build compiles in and leaves build-only ones out (#955)", () => {
    const pkg = (name: string, kind = "lib") => ({
      id: name,
      name,
      version: "1.0.0",
      license: "MIT",
      manifest_path: `/r/${name}/Cargo.toml`,
      targets: [{ kind: [kind] }],
    });
    const dep = (name: string, kind: string | null = null) => ({ pkg: name, dep_kinds: [{ kind }] });
    const meta: CargoMetadata = {
      packages: [
        pkg("engine"),
        pkg("serde"),
        pkg("serde_derive", "proc-macro"),
        pkg("syn"),
        pkg("itoa"),
        pkg("cc"),
        pkg("proptest"),
      ],
      resolve: {
        root: "engine",
        nodes: [
          { id: "engine", deps: [dep("serde"), dep("serde_derive"), dep("cc", "build"), dep("proptest", "dev")] },
          { id: "serde", deps: [dep("itoa"), dep("serde_derive")] },
          { id: "serde_derive", deps: [dep("syn")] },
          { id: "syn", deps: [] },
          { id: "itoa", deps: [] },
          { id: "cc", deps: [] },
          { id: "proptest", deps: [] },
        ],
      },
    };
    // A proc-macro and what only it pulls in, build and dev dependencies and the root itself stay out.
    expect(shippedCrates(meta).map((p) => p.name)).toEqual(["itoa", "serde"]);
    expect(() => shippedCrates({ ...meta, resolve: { ...meta.resolve, root: null } })).toThrow(/no root/);
    // Narrowed to what `cargo tree` says this build compiles (metadata unifies features across builds).
    expect(shippedCrates(meta, new Set(["engine v1.0.0", "serde v1.0.0"])).map((p) => p.name)).toEqual(["serde"]);
    // A listing that does not name the root was misread, and would drop every crate.
    expect(() => shippedCrates(meta, new Set(["serde v1.0.0"]))).toThrow(/root crate/);
    expect(() => shippedCrates(meta, new Set())).toThrow(/root crate/);
    // What cargo tree prints: the root with its path, repeats marked (*), proc-macros marked.
    const listing = "engine v1.0.0 (/r/engine)\nserde v1.0.0\nserde_derive v1.0.0 (proc-macro)\nserde v1.0.0 (*)\n\n";
    expect([...cargoTreePackages(listing)]).toEqual(["engine v1.0.0", "serde v1.0.0", "serde_derive v1.0.0"]);
    expect(shippedCrates(meta, cargoTreePackages(listing)).map((p) => p.name)).toEqual(["serde"]);
  });

  it("holds the standard library's license to a reviewed list of its parts (#974)", () => {
    const LICENSE = "(MIT OR Apache-2.0) AND Unicode-3.0 AND Zlib";
    const dir = (path: string, license: string) =>
      `<p><b>File/Directory:</b> <code>${path}</code></p><p><b>License:</b> ${license}</p>`;
    const dep = (name: string, license: string) => `<h3>📦 ${name}</h3><p><b>License:</b> ${license}</p>`;
    const page = [
      dir(".", "Apache-2.0 OR MIT"),
      dir("library/core/src/unicode/unicode_data.rs", "Unicode-3.0"),
      dir("library/std/src/sys/sync/mutex/fuchsia.rs", "BSD-2-Clause AND (Apache-2.0 OR MIT)"),
      dep("dlmalloc-0.2.13", "MIT/Apache-2.0"),
      dep("foldhash-0.2.0", "Zlib"),
      dep("fortanix-sgx-abi-0.6.1", "MPL-2.0"),
      dep("r-efi-5.3.0", "MIT OR Apache-2.0 OR LGPL-2.1-or-later"),
      dep("wasip3-0.6.0+wasi-0.3.0-rc-2026-03-15", "Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT"),
      dep("vex-sdk-0.27.1", "MIT"),
    ].join("\n");
    expect(stdLicenseProblems(page, LICENSE)).toEqual([]);
    // A new part under a license that offers neither MIT nor Apache-2.0 waits for a review.
    expect(stdLicenseProblems(page + dep("newcrate-1.0.0", "BSD-3-Clause"), LICENSE).join()).toMatch(
      /newcrate is BSD-3-Clause, which is not reviewed/,
    );
    expect(stdLicenseProblems(page + dir("library/std/src/new.rs", "MIT AND BSD-2-Clause"), LICENSE).join()).toMatch(
      /new\.rs is MIT AND BSD-2-Clause/,
    );
    // A reviewed part that changes license, or a wasm32 part the entry leaves out.
    expect(stdLicenseProblems(page.replace("MPL-2.0", "GPL-2.0-only"), LICENSE).join()).toMatch(
      /fortanix-sgx-abi is now GPL-2\.0-only/,
    );
    expect(stdLicenseProblems(page, "(MIT OR Apache-2.0) AND Unicode-3.0").join()).toMatch(
      /foldhash \(Zlib\) reaches wasm32/,
    );
    // A heading with attributes or markup still parses, and every license line under a part is checked.
    expect(
      stdLicenseProblems(
        page
          .replace("<h3>📦 fortanix-sgx-abi-0.6.1</h3>", '<h3 id="x">📦 <a href="#">fortanix-sgx-abi-0.6.1</a></h3>')
          .replace("MPL-2.0", "GPL-3.0-only"),
        LICENSE,
      ).join(),
    ).toMatch(/fortanix-sgx-abi is now GPL-3\.0-only/);
    expect(
      stdLicenseProblems(
        page.replace("<b>License:</b> MPL-2.0", "<b>License:</b> MPL-2.0</p><p><b>License:</b> GPL-3.0-only"),
        LICENSE,
      ).join(),
    ).toMatch(/is now GPL-3\.0-only/);
    // A relabeled license line, or a part with none, fails instead of being skipped.
    expect(
      stdLicenseProblems(page.replace("<b>License:</b> MPL-2.0", "<b>License</b>: GPL-3.0-only"), LICENSE).join(),
    ).toMatch(/layout may have changed/);
    // A reviewed wasm32 part that is no longer listed asks for a new review.
    expect(stdLicenseProblems(page.replace(dep("foldhash-0.2.0", "Zlib"), ""), LICENSE).join()).toMatch(
      /foldhash .* is no longer listed/,
    );
    // Prerelease versions and a parenthesized choice parse; a license is matched as a whole SPDX id.
    expect(stdLicenseProblems(page + dep("newcrate-1.0.0-rc.1", "(MIT OR Apache-2.0)"), LICENSE)).toEqual([]);
    expect(stdLicenseProblems(page, "(MIT OR Apache-2.0) AND Unicode-3.0 AND Zlib-Acknowledgement").join()).toMatch(
      /foldhash \(Zlib\) reaches wasm32/,
    );
    // Every path in a grouped file entry is reviewed, and a license that is not plain SPDX is refused.
    const grouped = page.replace(
      "<code>library/std/src/sys/sync/mutex/fuchsia.rs</code>",
      "<code>library/std/src/sys/sync/mutex/fuchsia.rs</code>, <code>library/std/src/sys/alloc/wasm.rs</code>",
    );
    expect(stdLicenseProblems(grouped, LICENSE).join()).toMatch(/alloc\/wasm\.rs is BSD-2-Clause/);
    expect(stdLicenseProblems(page + dep("lower-1.0.0", "MIT OR Apache-2.0 and GPL-3.0-only"), LICENSE).join()).toMatch(
      /cannot read as SPDX/,
    );
    expect(stdLicenseProblems(page + dep("amp-1.0.0", "MIT OR Apache-2.0 &amp; GPL-3.0-only"), LICENSE).join()).toMatch(
      /cannot read as SPDX/,
    );
    // A page whose layout changed lists nothing, which is refused rather than passed.
    expect(stdLicenseProblems("<p>no parts</p>", LICENSE).join()).toMatch(/lists no parts/);
  });

  it("decodes the toolchain's copyright page and refuses an entity it does not know", () => {
    expect(htmlText("<p>a &#34;b&#34; &#x3c;c&#62; &lt;d&gt; &amp;lt;</p><br>e")).toBe('a "b" <c> <d> &lt;\n\ne');
    expect(() => htmlText("<p>&copy; 2026</p>")).toThrow(/&copy;/);
    expect(() => htmlText("&#x110000;")).toThrow(/&#x110000;/);
    // One pass: an encoded ampersand is decoded once, whichever way it is written.
    expect(htmlText("&#38;lt; &#x26;amp; &amp;amp;")).toBe("&lt; &amp; &amp;");
    expect(() => htmlText("a&#0;b")).toThrow(/not a character/);
    expect(() => htmlText("&#xD800;")).toThrow(/not a character/);
    expect(htmlText("<!-- a > b -->kept")).toBe("kept");
    expect(() => htmlText("a&#x9b;b")).toThrow(/not a character/);
    // Markup the strip misreads, and an entity without its semicolon, are refused rather than copied.
    expect(() => htmlText('<a title="x>y">z</a>')).toThrow(/markup/);
    expect(() => htmlText("&copy 2026")).toThrow(/markup/);
    expect(() => htmlText("a\u001bb")).toThrow(/markup/);
    expect(() => htmlText("a\u009bb")).toThrow(/markup/);
    expect(htmlText("a\tb\nc")).toBe("a\tb\nc");
  });

  it("builds a port's entry from the license vendored beside it, with no node_modules (#1356)", () => {
    const port = { name: "fflate", version: "0.8.3", license: "MIT", source: "src/fflate.rs", notice: "src/fflate" };
    const problems: string[] = [];
    expect(portEntry(port, { licenseText: "", hasLicenseFile: false, problems: [] }, problems)).toBeNull();
    expect(problems.join()).toMatch(/vendor its license in src\/fflate\//);
    const unreadable: string[] = [];
    expect(
      portEntry(
        port,
        { licenseText: "MIT text", hasLicenseFile: true, problems: [{ file: "LICENSE", reason: "is not UTF-8" }] },
        unreadable,
      ),
    ).not.toBeNull();
    expect(unreadable).toEqual(["fflate: LICENSE is not UTF-8"]);
    const found: string[] = [];
    expect(
      portEntry(port, { licenseText: "MIT License, Arjun Barrett", hasLicenseFile: true, problems: [] }, found),
    ).toEqual({
      name: "fflate",
      version: "0.8.3",
      license: "MIT",
      licenseText: "MIT License, Arjun Barrett",
      hasLicenseFile: true,
      kind: "port",
    });
    expect(found).toEqual([]);
  });
});
