---
crate: patch
---
- the conformance referee writes both locks with `--update`
  (`cargo run --release --features conformance --bin conformance -- --update`):
  every scenario's checkpoints into `conformance/expected.json` and each
  mode's catalog digest into `conformance/catalog-digests.json`, only when
  every scenario runs to its end and never in CI.
