// Type-checked in the scratch project (scripts/package-consumer.sh) against
// the packed package's declarations, as a game written for a bundler reads
// them ("moduleResolution": "bundler", README).
import { Engine, catalog, engineIdentity, exportTdt, importTdt } from "@verticopolis/engine";
import init, { initSync } from "@verticopolis/engine/web/verticopolis_engine.js";
import type { GameplayMilestoneId } from "@verticopolis/engine/events/gameplayEvents";
import { GAMEPLAY_EVENT_RULES, type GameplayEventRules } from "@verticopolis/engine/events/gameplayEventRules";

const milestone: GameplayMilestoneId = "pop-500";
const rules: GameplayEventRules = GAMEPLAY_EVENT_RULES;
const calls: unknown[] = [Engine, catalog, engineIdentity, exportTdt, importTdt, init, initSync];
export const used = [milestone, rules.events.length, calls.length];
