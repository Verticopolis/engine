//! Creative play: a free-build tower. Every facility the mode offers is
//! unlocked from the start, whatever the star rating, and the money never
//! runs out. A host turns it on for a tower it offers as a sandbox, and the
//! tower keeps it in its save. Nothing else changes: the tower still earns,
//! pays, rates and grows by the mode's rules, and the stars still count for
//! everything but what may be built.

use crate::sim::Simulation;

/// The funds a creative tower holds. Every hour of play fills them back up,
/// so no build, however large, can run them out.
pub const CREATIVE_FUNDS: f64 = 1_000_000_000_000.0;

impl Simulation {
    /// Turns creative play on or off. On, the funds fill to
    /// [`CREATIVE_FUNDS`] at once and after every step of play; off, the
    /// funds stay where they are and the mode's star unlocks apply again.
    ///
    /// ```
    /// use verticopolis_engine::clock::GameMode;
    /// use verticopolis_engine::creative::CREATIVE_FUNDS;
    /// use verticopolis_engine::facilities::Kind;
    /// use verticopolis_engine::sim::Simulation;
    ///
    /// let mut sim = Simulation::new_game(1, GameMode::Classic);
    /// assert!(!sim.is_unlocked(Kind::HotelSingle));
    /// sim.set_creative(true);
    /// assert!(sim.is_unlocked(Kind::HotelSingle));
    /// assert_eq!(sim.money, CREATIVE_FUNDS);
    /// ```
    pub fn set_creative(&mut self, on: bool) {
        self.creative = on;
        self.refill_creative_funds();
    }

    /// Fills a creative tower's funds back up; a no-op on any other tower.
    pub(crate) fn refill_creative_funds(&mut self) {
        if self.creative {
            self.money = CREATIVE_FUNDS;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::GameMode;
    use crate::facilities::Kind;
    use crate::load::deserialize;

    #[test]
    fn a_creative_tower_unlocks_every_kind_its_mode_offers() {
        for mode in [GameMode::Classic, GameMode::Modern] {
            let mut sim = Simulation::new_game(3, mode);
            sim.set_creative(true);
            for f in crate::facilities::FACILITIES.iter() {
                assert_eq!(
                    sim.is_unlocked(f.kind),
                    f.available_in(mode),
                    "{:?} in {mode:?}",
                    f.kind
                );
            }
            sim.set_creative(false);
            assert_eq!(
                sim.is_unlocked(Kind::HotelSingle),
                sim.star >= Kind::HotelSingle.facility().min_star
            );
        }
    }

    #[test]
    fn its_funds_fill_back_up_after_play_and_stay_put_once_it_is_off() {
        let mut sim = Simulation::new_game(3, GameMode::Classic);
        sim.set_creative(true);
        sim.money -= 5_000_000.0;
        sim.tick(60.0);
        assert_eq!(sim.money, CREATIVE_FUNDS);
        sim.set_creative(false);
        sim.money = 42.0;
        sim.tick(60.0);
        assert!(
            sim.money < CREATIVE_FUNDS,
            "play goes on as usual once it is off"
        );
    }

    #[test]
    fn the_save_keeps_it() {
        let mut sim = Simulation::new_game(3, GameMode::Classic);
        let plain = sim.serialize();
        assert!(!deserialize(&plain).unwrap().creative);
        sim.set_creative(true);
        let back = deserialize(&sim.serialize()).unwrap();
        assert!(back.creative);
        assert!(back.is_unlocked(Kind::HotelSingle));
    }
}
