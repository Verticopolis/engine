---
crate: minor
---
- A tower that has ever been a free-build tower carries a lasting mark: `setCreative(true)`
  sets `Simulation::free_built` for good, the save writes `"freeBuilt": true`, and a load
  restores the mark without turning free building on. The binding reads it with
  `freeBuilt()`, so a host can refuse a tower built with unlimited money or open it only
  as a sandbox. Towers never put in free building save exactly as before.
