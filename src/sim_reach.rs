//! Reachability reads the binding and the move-in checks use: whether a
//! floor, or a spot on it, can be reached from the lobby.

use crate::sim::Simulation;

impl Simulation {
    #[doc(hidden)]
    /// `floorReachable(floor)`: memoized per tower revision, no rng.
    pub fn floor_reachable(&mut self, floor: i64) -> bool {
        if floor == 1 {
            return true;
        }
        if self.reach_memo.0 != self.tower.revision {
            self.reach_memo = (self.tower.revision, Default::default());
        }
        if let Some(&hit) = self.reach_memo.1.get(&floor) {
            return hit;
        }
        let hit = self.crowd.floor_reachable(&self.tower, floor);
        self.reach_memo.1.insert(floor, hit);
        hit
    }

    #[doc(hidden)]
    /// `positionReachable(floor, x)`.
    pub fn position_reachable(&mut self, floor: i64, x: i64) -> bool {
        if floor == 1 {
            return true;
        }
        if self.tower.segment_run_count(floor) <= 1 {
            return self.floor_reachable(floor);
        }
        self.crowd.position_reachable(&self.tower, floor, x)
    }

    #[doc(hidden)]
    /// `unitReachable` (the retired TypeScript `src/engine/sim/demand.ts`).
    pub fn unit_reachable(&mut self, floor: i64, x: i64) -> bool {
        self.position_reachable(floor, x)
    }
}
