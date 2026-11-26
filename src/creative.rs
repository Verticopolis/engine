//! Creative play: a free-build tower. Every facility the mode offers is
//! unlocked from the start, whatever the star rating, and the money never
//! runs out, and the Sky Bridge's star bar is lifted too. A host turns it on
//! for a tower it offers as a sandbox, each time it opens that tower: the save
//! leaves it out, so a tower saved in a sandbox opens as a plain tower
//! anywhere else (with the funds it was saved with). Nothing else changes:
//! the tower still earns, pays, rates and grows by the mode's rules, and the
//! stars still count for everything but what may be built. A host that reads
//! the catalog's star bars (`rules.sky_bridge.min_star`, each facility's
//! `min_star`) should also read [`Simulation::creative`] before it shows one.

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
    fn the_save_leaves_it_out() {
        let mut sim = Simulation::new_game(3, GameMode::Classic);
        let plain = sim.serialize();
        sim.set_creative(true);
        sim.money = 42.0;
        let free = sim.serialize();
        sim.set_creative(false);
        sim.money = 42.0;
        assert_eq!(free, sim.serialize(), "the save carries no creative mark");
        let back = deserialize(&free).unwrap();
        assert!(!back.creative, "a loaded tower is a plain tower");
        assert!(!back.is_unlocked(Kind::HotelSingle));
        assert!(!deserialize(&plain).unwrap().creative);
    }

    #[test]
    fn a_creative_tower_builds_a_sky_bridge_at_any_star() {
        let mut sim = Simulation::new_game(3, GameMode::Modern);
        assert!(sim.star < crate::tower::SKYBRIDGE_MIN_STAR);
        let refused = sim.quote_skybridge(15, 120);
        assert!(!refused.ok);
        assert!(refused
            .reason
            .as_deref()
            .unwrap_or("")
            .contains("unlocks at"));
        sim.set_creative(true);
        let quote = sim.quote_skybridge(15, 120);
        assert!(
            !quote.reason.as_deref().unwrap_or("").contains("unlocks at"),
            "the star bar is lifted: {:?}",
            quote.reason
        );
    }
}
