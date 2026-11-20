---
crate: patch
---
- the npm package's `events/gameplayEventRules.ts` declares its
  own `GameplayEventRules` type and exports it. It imported that type from
  `./gameplayCatalog`, a file the package never carried, so TypeScript
  refused the file in a game that imported the rules. The rules themselves
  are unchanged.
