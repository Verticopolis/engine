/**
 * The cosmetic fx sequences (`santaFxSeq`, the explosion, thief, treasure and
 * VIP records) and the emergency counters (fires, gutted rooms, bombs) sit
 * outside the hashed views, so the conformance lock never sees them (Verticopolis/web#879).
 * The checkpoint detail carries them (`FX_SLOTS`), the seeded differential
 * compares them every hour, and this suite pins them at the end of the
 * scenarios that reach each site on the shipped build: an off-by-one at any
 * site changes a pinned value.
 */
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
import { FX_SLOTS, type CheckpointDetail, type FxSlot } from "./checkpointDetail.ts";
import { CONFORMANCE_DIR, loadScenario, runScenario } from "./scenario.ts";
import { startWasmEngine } from "./wasmEngine.ts";

type Nonzero = Partial<Record<FxSlot, number>>;

/** Every slot a scenario leaves non-zero at its last checkpoint. A slot not
 *  listed is 0 (the basement is floor 0, so a treasure there lists no floor). */
const PINNED: Record<string, Nonzero> = {
  // Riders boarding only cars stopped at their floor (Verticopolis/web#2051) lit one fire in
  // these two and a half years; Classic's 1994 dispatch rules (Verticopolis/web#2049) put the
  // run on another seeded path, which lit none; riders walking straight to
  // their place in line (Verticopolis/web#2116) put it on a path that lights one again;
  // riders staying aboard to their floors (3.7.3) put it on one that lights
  // none, and later dispatch changes lit three again; meal venues drawing only
  // their real diners (Verticopolis/web#2160) move the cars onto a path that lights none, and
  // walk-ins switched off (3.12.2) onto one that lights one again, and
  // express elevators stopping at every basement floor onto one that lights
  // none, and Classic express elevators stopping at every 15th floor (Verticopolis/web#1527)
  // onto one that lights one, gutting two rooms. The fire sites stay pinned
  // by the events and fire-rescue runs. Shop-only floors forgetting their
  // wait averages after the hour put it on a path that lights one, gutting
  // thirteen rooms, and wait averages starting from no wait (Verticopolis/web#2435) onto one
  // that lights none. Shoppers walking in from the street at the 1994 daily
  // counts (Verticopolis/web#2631) put it on one that lights one, gutting one room.
  "scenarios/holidays-classic.json": {
    santaFxSeq: 2,
    thiefSeq: 1,
    thiefCaught: 1,
    thiefFloor: 77,
    fires: 1,
    firesGutRooms: 1,
  },
  "scenarios/events-classic.json": {
    explosionSeq: 1,
    explosionFloor: 5,
    explosionX: 174.5,
    thiefFloor: 1,
    fires: 1,
    firesGutRooms: 1,
    bombs: 1,
  },
  // After the direct threat detonates, a second threat is called in during
  // the last two days, with Security sold and nobody to search (Verticopolis/web#1973). Venue
  // staff coming in as people (Verticopolis/web#2238) moved the run onto a path where only
  // the direct threat detonates. Slower Modern escalators and stairs moved
  // that blast into the lobby, and riders boarding only a car going their
  // way moved it to floor 42. Counting the walk to the first shaft moved it
  // back into the lobby. Office workers going home from an evening dinner or
  // movie (Verticopolis/web#2481) moved it along the lobby, from tile 273 to 281.
  // Emergencies on the real-world calendar rolling a tenth of their canon
  // odds made the run wait about a hundred days for its threat, and the
  // blast landed on floor 18 at tile 141.
  "scenarios/ransom-ignored-modern.json": {
    explosionSeq: 1,
    explosionFloor: 18,
    explosionX: 141,
    thiefFloor: 1,
    bombs: 1,
  },
  // The same change made the run wait about 145 days for its first fire,
  // which crosses the holidays, and its two fires gut two rooms.
  "scenarios/fire-rescue-ignored-modern.json": {
    santaFxSeq: 1,
    thiefFloor: 1,
    vipFxSeq: 1,
    fires: 2,
    firesGutRooms: 2,
  },
  "scenarios/vip-review-modern.json": { thiefFloor: 1, vipFxSeq: 1 },
  // Seed 13 since the real-world calendar's thief rolls a tenth of its canon
  // odds: one thief in the sixty days, where seed 17 now sees none.
  "fuzz-corpus/fx-thief-uncaught.json": { thiefSeq: 1, thiefFloor: 1 },
  // Sixty days at three stars also cross the holidays and light a fire.
  "fuzz-corpus/fx-thief-caught.json": {
    santaFxSeq: 3,
    thiefSeq: 1,
    thiefCaught: 1,
    thiefFloor: 1,
    fires: 1,
    firesGutRooms: 1,
  },
  "fuzz-corpus/fx-treasure-cap.json": { treasureSeq: 3, treasureX: 172, thiefFloor: 1 },
};

const asRecord = (d: CheckpointDetail): Nonzero =>
  Object.fromEntries(Object.entries(d.fx).filter(([, v]) => v !== 0)) as Nonzero;

/** A month or two of game time; the longest takes several seconds on a busy runner. */
const SLOW = 180_000;

describe("the fx sequences and emergency counters, pinned on the shipped build", () => {
  for (const [file, expected] of Object.entries(PINNED)) {
    it(
      `${file}: the shipped build ends on the pinned values`,
      () => {
        const wasm: CheckpointDetail[] = [];
        runScenario(loadScenario(resolve(CONFORMANCE_DIR, file)), startWasmEngine, wasm);
        expect(asRecord(wasm[wasm.length - 1])).toEqual(expected);
      },
      SLOW,
    );
  }

  it("reaches every slot at least once across the pinned scenarios, so no site goes unpinned", () => {
    const reached = new Set<string>();
    for (const expected of Object.values(PINNED)) for (const k of Object.keys(expected)) reached.add(k);
    // The positions (floor and x) can legitimately be 0; every sequence, flag and counter must be reached.
    const sites = FX_SLOTS.filter((k) => !/(Floor|X)$/.test(k));
    expect(sites.filter((k) => !reached.has(k))).toEqual([]);
  });
});
