# Project Context: Verticopolis engine (AI agent rules)

Foundational context every BMAD agent (bmm / cis / **gds game agents**) must carry
when working in Verticopolis/engine. Concise on purpose: only the non-obvious
rules an LLM needs reminding of. [CLAUDE.md](../CLAUDE.md),
[CONTRIBUTING.md](../CONTRIBUTING.md) and [ARCHITECTURE.md](../ARCHITECTURE.md)
hold the full rules; [docs/parity.md](../docs/parity.md) says how a Classic tower
matches the 1994 game.

## What this is
- The **Verticopolis engine**: the one simulation every Verticopolis frontend runs.
  The Rust crate `verticopolis-engine` sits at this repository's root (`Cargo.toml`,
  `src/`, `tests/`); the WebAssembly build and the npm package
  `@verticopolis/engine` (TypeScript layer in `packages/verticopolis-engine/`), the
  conformance suite (`conformance/`), the docs site and the bench live here too.
- Verticopolis is a from-scratch, clean-room homage to **SimTower (1994,
  Maxis/OPeNBooK)**. **The single source of truth for Classic is SimTower 1994**,
  specifically the GameFAQs/Kiwizoid FAQ (summarized with attribution in
  `_bmad-output/planning-artifacts/reviews/faq-parity-2026-06-30/faq-canon.md`).
  For **Classic**, the bar is **gameplay parity**, not reinvention.
- Frontends (the free web game and other consumers) may differ. Simulations may
  not: a frontend never adds, copies or patches a rule. A rule change lands here,
  with a regenerated conformance lock in the same PR, and the engine asks only for
  what any frontend could use.

## Scope: Classic parity-locked, Modern is a curated sandbox (2026-07-22)
- **Classic** stays rule-faithful to SimTower 1994. No new mechanics, no new
  content. Its golden masters must stay byte-stable.
- **Modern** is the sanctioned home for **post-1994 additions**: new room types,
  new economy behaviors, and freeform build controls the original could not do.
  These are **added deliberately** through the design process (party ratification,
  then a design doc or spec, then `/gds-code-review`), never ad hoc. A Modern
  addition must leave Classic byte-identical.
- **Any gameplay or building-rule change needs the owner's explicit OK before
  merge.** A party or roundtable verdict is not that OK.
- **No TDT for Modern.** The 1994 `.TDT` format is **Classic-only**. No
  Modern-only feature adds TDT handling (no new ordinals, no format-version bump
  for Modern content); Modern persists through the native save format only.
- The living plan for Modern additions is
  `_bmad-output/planning-artifacts/gdds/gdd-modern-expansion-2026-07-22.md`.

## Where the truth lives
- **This crate is the source of truth for gameplay rules, caps and tuning**:
  `src/facilities.rs` (lot 375 wide, floors −9…100, per-facility
  cost/width/minStar/population, build caps, `POOLED_CAPS`, `Kind::max_cars`,
  `Kind::max_span`), `src/star.rs` (`STAR_THRESHOLDS`, `TOWER_POPULATION`) and
  `src/rules.rs` with its `rules/` submodules (the Classic and Modern rule values).
  Caps are enforced in one place, `Tower::cap_reason` (`src/tower.rs`).
  Frontends read all of it through the catalog (`src/catalog.rs`);
  `conformance/catalog-digests.json` is written from it, so a changed cap moves
  that digest in the PR that changes it.
- Where things are:
  - `src/sim.rs`, `sim_loop.rs`, `clock.rs`, `fixed_step.rs`: the clock, the step
    loop and pacing; `star.rs`: star evaluation; `events*.rs`: VIP, fire, bomb, thief.
  - `src/tower.rs` and `tower/`: placement and geometry (`is_supported`), removal
    (`removal_reason`), served-floor reachability; `tower_query.rs`: staff
    connectivity (`staff_connected`).
  - `src/economy.rs`, `econ.rs`, `rent.rs`, `housekeeping*.rs`, `crowd/`: money,
    rent, housekeeping and routing.
  - `src/dispatch*.rs`: the elevator car controller (`ElevatorDispatch`).
    Transport-kind predicates (`is_staff_only_transport` and the rest) are on
    `Kind` in `src/facilities/predicates.rs`.
- The TypeScript reference engine was deleted (Verticopolis/web#1875). Never add
  a TypeScript copy of a rule, here or in a frontend.

## Canon rules that are easy to get wrong
- **Star ladder** (population + gates): 2★ 300 · 3★ 1,000 (Security) · 4★ 5,000
  (Medical + Recycling + >1 Suite + a favorable VIP) · 5★ 10,000 (Metro). **TOWER = 15,000.**
- **Rating census** counts office workers + condo residents + commercial venue
  customers. Commercial counts LIVE customers (whoever is eating right now, often
  0); the catalog values (fast food 25 / restaurant 35 / shop 20, chosen per
  footprint) are what TDT export writes, not a flat census add.
  `Unit::census_count` (`src/tower_query_state.rs`) owns the rule. **Hotel guests
  count while climbing up through 4★, then drop out** (`rating_population`,
  `src/star.rs`).
- **Two-ride rule:** a trip uses at most **two** transport rides (one sky-lobby
  transfer); floors reachable only via 3+ rides draw no commuters. Sky lobbies go
  every ~15 floors.
- **Transport pooling matches the original and is deliberate:** all three elevator
  kinds (standard, service, express) share **one 24-shaft pool** in Classic, with
  express inside it; Modern widens that one pool to **48 shafts**
  (`MODERN_ELEVATOR_SHAFT_CAP`). Stairs and escalators share a separate 64-link
  pool in Classic; Modern sets no limit on them, only the lot's room. 8 cars per
  shaft for every elevator kind. Do not "fix" express out of the elevator pool.
- **Elevators may stand in front of rooms.** Never reintroduce a rule that keeps a
  shaft off a room (Verticopolis/web#2454 was reverted).
- **Parking** has a **Ramp** + **Spaces**; a space only functions when chained
  (contiguous spaces) back to a ramp. Unconnected spaces are dead ("red X").
- **Office noise** caps adjacent hotel/condo satisfaction (≤0.6). In **Classic**
  that is all it does (canon: caps but never evicts). In **Modern** it also erodes
  below the cap and can eventually evict (`noise_erosion_scale` in
  `src/rules/satisfaction.rs` gates this; Classic returns 0).
- **Cinemas** book an average (~$150k) or **blockbuster** (~$300k, bigger crowd)
  film monthly.
- Emergencies (fire rescue, bomb ransom) are **player choices** that **pause** the
  sim. Fires have no evacuation.
- **Service elevators are staff-only**: tenants never route through or board them,
  and they do **not** count toward served-floor reachability. Staff route over the
  staff network (service elevators + stairs + escalators) and **service elevators
  win route ties** (build order must not decide). Staff-only shafts answer **only
  real staff calls**, never the statistical tenant-demand estimate.
- **Housekeeping never cleans instantly**: a room stays dirty until a housekeeper
  physically arrives over the staff network. Capacity is finite (~20 rooms per
  crew per day); over-capacity and can't-reach conditions are separate states.
- **Stairs and escalators are fixed two-floor flights**, with a span cap enforced
  on **every** path including resize and extend, and stacking allowed only with an
  exact-footprint match sharing the landing floor.
- **Structure needs support both ways**: a floor above the ground story requires
  the story below fully built beneath it, and bulldozing is refused when it would
  leave the story above hanging (`is_supported` / `removal_reason`).
- No people warp and none get stuck.

## Ratified divergences from canon (intentional: do NOT "fix")
- **Wedding Hall** on floor 100 stands in for the original **Cathedral**
  (religion-neutral clean-room choice); mechanics are identical.
- Canon-non-removable structures are **kept removable** (partial-refund bulldoze)
  as a QoL choice.

## Artifacts & working conventions
- BMAD output lives under `_bmad-output/` in this repository: the backlog in
  `implementation-artifacts/backlog.md`, specs and stories beside it, plans in
  `planning-artifacts/`, party findings and room memory in `party-mode/`. Records
  dated before 2026-11-20 came over from Verticopolis/web with their history; the
  paths they name (`engine-rs/...`, `src/storage/...`) are the paths of that time.
- **Deep adversarial review is mandatory, in the same session that writes the
  code:** `/gds-code-review` for simulation, binding, frame and save work,
  `/bmad-code-review` for CI, packaging and tooling. Its layers are the review; a
  self-read or a generic `/code-review` does not count. Fix every `patch` finding,
  record every `defer` in `_bmad-output/implementation-artifacts/backlog.md`. An
  independent GO from a separate thread or person comes before every merge.
- **Hot paths get the complexity checklist**,
  `docs/performance/hot-path-review.md` in Verticopolis/web, which governs this
  repository too. The tick loop, dispatch, the crowd and the binding run over the
  whole tower every step: look entities up by id, hoist tower-wide facts out of
  per-unit and per-person loops, keep counters instead of re-scanning, and memoize
  per-revision work. Nothing ever gets slower.
- **Backlog rows mirror to GitHub issues in this repository** (see the backlog's
  "How items flow").
- **Tests:** shift-left and regression (every fix ships a test that pins the
  failure); every test asserts what its name promises; no guard tests (never a
  test that reads source, workflow or configuration files); no host access in the
  simulation (`tests/independence.rs`).
- **Change notes, never a version edit:** `.changes/<branch-slug>.md` with its
  `crate:` tier (see `.changes/README.md`). A bug fix is patch; minor only for new
  features or API.
- **Saves:** no lost saves. A format change proves an identical game on real old
  saves and both maxed towers.
- **Prose:** American English, no em-dashes, no "X, not Y" emphatic pattern, no
  marketing vocabulary. A 500-line ceiling per source file (tests and generated
  files exempt).
- **Merge commits only** to `main`, never a squash.
