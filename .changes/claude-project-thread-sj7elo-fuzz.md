---
crate: patch
---
- the `fuzz` tool (feature `conformance`) takes an optional third argument in
  its replay form, `fuzz --replay <scenario.json> <out-dir> [conformance-dir]`,
  naming the conformance directory a scenario's fixtures load from. The
  WebAssembly build of the fuzzer, run under WASI, sees only the directories
  it is given, so the nightly fuzz and the corpus check pass it. The engine
  itself is unchanged.
