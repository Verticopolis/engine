---
crate: minor
---
- A free-build tower: `Simulation::set_creative` and the binding's
  `setCreative(on)` and `creative()`. A creative tower has every kind its
  mode offers unlocked whatever its star rating, and its funds fill back to
  `creative::CREATIVE_FUNDS` after every step of play. The save keeps it
  (`"creative": true`, written only when on). Nothing else about the
  simulation changes, and no host has to offer it.
