import { describe, expect, it } from "vitest";
import { DetailError, requireDetail } from "./checkpointDetail.ts";

/** The record's fx slots are pinned through the glue by `fxSequences.test.ts`;
 *  `scripts/fuzz-corpus.mts` holds the native and wasm32 records together. */
describe("checkpoint detail", () => {
  it("turns a missing or failing detail into a DetailError", () => {
    expect(() => requireDetail({})).toThrow(DetailError);
    expect(() =>
      requireDetail({
        detail: () => {
          throw new Error("boom");
        },
      }),
    ).toThrow(/boom/);
    expect(() =>
      requireDetail({
        detail: () => {
          throw new Error("boom");
        },
      }),
    ).toThrow(DetailError);
  });
});
