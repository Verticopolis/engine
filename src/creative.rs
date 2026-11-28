//! Creative play: a free-build tower. Every facility the mode offers is
//! unlocked from the start, whatever the star rating, and the money never
//! runs out, and the Sky Bridge's star bar is lifted too. A host turns it on
//! for a tower it offers as a sandbox, each time it opens that tower: the save
//! leaves the switch itself out. Turning it on also marks the tower as
//! free-built for good ([`Simulation::free_built`]), and the save keeps that
//! mark, so a host can refuse such a tower or open it only as a sandbox; the
//! mark never turns free building back on by itself. Nothing else changes:
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
    /// [`CREATIVE_FUNDS`] at once and after every step of play, and the tower
    /// is marked [`Simulation::free_built`] for good; off, the funds stay
    /// where they are and the mode's star unlocks apply again.
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
        self.free_built |= on;
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
    fn a_tower_file_carries_the_mark_through_the_export() {
        let mut sim = Simulation::new_game(5, GameMode::Modern);
        sim.set_creative(true);
        let text = crate::canonical::js_json(&sim.serialize());
        let file = crate::export_stream::encode_vctower_string_as(
            text,
            "Sandbox 1.0.0",
            1_700_000_000_000.0,
            crate::load::Frame::Binary,
        )
        .unwrap();
        let read = crate::load::decode_vctower_bytes(&file).unwrap();
        let back = deserialize(&read).unwrap();
        assert!(back.free_built && !back.creative);
    }

    #[test]
    fn the_save_leaves_the_switch_out_and_keeps_the_mark() {
        let mut sim = Simulation::new_game(3, GameMode::Classic);
        let plain = sim.serialize();
        assert!(
            plain.get("freeBuilt").is_none(),
            "a tower built by the rules saves as before"
        );
        sim.set_creative(true);
        sim.money = 42.0;
        let free = sim.serialize();
        sim.set_creative(false);
        sim.money = 42.0;
        assert!(sim.free_built, "turning free building off keeps the mark");
        assert_eq!(
            free,
            sim.serialize(),
            "the save carries the mark, not the switch"
        );
        assert_eq!(free.get("freeBuilt"), Some(&serde_json::json!(true)));
        let back = deserialize(&free).unwrap();
        assert!(!back.creative, "loading never turns free building on");
        assert!(back.free_built, "the mark survives a save and a load");
        assert!(!back.is_unlocked(Kind::HotelSingle));
        let back_plain = deserialize(&plain).unwrap();
        assert!(!back_plain.creative && !back_plain.free_built);
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
