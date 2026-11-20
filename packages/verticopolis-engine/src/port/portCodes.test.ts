import { describe, expect, it, vi } from "vitest";
import { PortCodes, outcome } from "./portCodes.ts";
import { LAYOUT, mockPort, record } from "../testing/mockPort.ts";

/** The layout's code tables and a command's outcome, over a mock port. The
 *  engine side runs in `portReads.integration.test.ts`. */

describe("PortCodes", () => {
  it("reads the layout once and passes names as its codes", () => {
    const port = mockPort(vi);
    const c = new PortCodes(port);
    expect(port.viewLayout).toHaveBeenCalledOnce();
    expect([c.code("modes", "modern"), c.code("logKinds", "bad"), c.code("heatModes", "nope")]).toEqual([1, 2, NaN]);
    expect([c.kind("lobby"), c.kind("office"), c.code("filmPolicies", "blockbuster")]).toEqual([0, 2, 2]);
    expect(c.names("kinds")).toEqual(LAYOUT.kinds);
    expect([c.fields("paintTiles"), c.fields("noSuchOp")]).toEqual([["status", "delta", "placed"], []]);
  });

  it("reads a record's named slots", () => {
    const c = new PortCodes(mockPort(vi));
    const rec = record("paintTiles", { status: 1, delta: -1500, placed: 3 });
    expect(c.slot(rec, "paintTiles", "placed")).toBe(3);
    expect(c.slot(rec, "paintTiles", "missing")).toBeNaN();
    expect(c.slot(rec, "noSuchOp", "placed")).toBeNaN();
  });
});

describe("outcome", () => {
  it("asks the reason only for a refusal", () => {
    const port = mockPort(vi, { lastReason: () => "Too high to build" });
    expect(outcome(port, record("buildAt", { status: 1, delta: -40_000 }))).toEqual({ ok: true, delta: -40_000 });
    expect(port.lastReason).not.toHaveBeenCalled();
    expect(outcome(port, record("buildAt", { status: 0, delta: 0 }))).toEqual({ ok: false, reason: "Too high to build", delta: 0 });
    expect(port.lastReason).toHaveBeenCalledOnce();
  });
});
