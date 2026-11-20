//! Batch pricing: one price for every room of a kind. In Modern the band
//! clamps a target and counts the clamp; on the Classic ladder a target
//! snaps to a rung and never clamps. The default is stored as no price of
//! the room's own, a sold condo keeps its price, the default-only filter
//! skips a hand-priced or off-market room, a quote is the batch without the
//! write, and any reprice puts a No Rate room back on the market. A single
//! room's step moves by one band step in Modern and one rung in Classic.
use crate::clock::GameMode;
use crate::econ::rent_of;
use crate::facilities::Kind;
use crate::rent::BatchTarget;
use crate::sim::Simulation;
use crate::tower::UnitState;

use super::{lay_row, unit};

/// A floor-2 tower of `mode` with `n` rooms of `kind`, `width` apart, all
/// on the default price. Returns the rooms' ids.
fn rooms(mode: GameMode, seed: u32, kind: Kind, n: i64, width: i64) -> (Simulation, Vec<i64>) {
    let mut sim = Simulation::new_game(seed, mode);
    sim.money = 1e9;
    sim.star = 5;
    lay_row(&mut sim, Kind::Lobby, 1);
    lay_row(&mut sim, Kind::Floor, 2);
    let ids = (0..n)
        .map(|i| {
            let x = 100 + i * width;
            assert!(sim.build(kind, 2, x).ok, "{kind:?} at {x}");
            sim.tower.unit_at(2, x).unwrap().id
        })
        .collect();
    (sim, ids)
}

fn offices(mode: GameMode, n: i64) -> (Simulation, Vec<i64>) {
    rooms(mode, 1, Kind::Office, n, 9)
}

/// What a room charges: nothing off the market, else its own price or its
/// kind's default.
fn price(sim: &Simulation, id: i64) -> f64 {
    let u = unit(sim, id);
    rent_of(u.kind, u.rent, u.no_rate)
}

/// Marks a condo sold, the way a save carries one.
fn sell(sim: &mut Simulation, id: i64) {
    let u = sim.tower.get_unit_mut(id).unwrap();
    u.ever_occupied = true;
    u.state = UnitState::Occupied;
}

fn batch(sim: &mut Simulation, kind: Kind, target: BatchTarget) -> crate::rent::BatchRentResult {
    sim.apply_rent_batch(kind, target, false).unwrap()
}

#[test]
fn a_modern_batch_sets_an_exact_price_and_clamps_to_the_band() {
    let (mut sim, ids) = offices(GameMode::Modern, 4);
    let r = batch(&mut sim, Kind::Office, BatchTarget::Price(12_000.0));
    assert_eq!((r.matched, r.eligible, r.changed), (4, 4, 4));
    assert!(ids.iter().all(|&id| price(&sim, id) == 12_000.0));
    let high = batch(&mut sim, Kind::Office, BatchTarget::Price(999_999.0));
    assert_eq!(high.clamped_high, 4);
    assert!(
        ids.iter().all(|&id| price(&sim, id) == 20_000.0),
        "band top"
    );
    let low = batch(&mut sim, Kind::Office, BatchTarget::Price(1.0));
    assert_eq!(low.clamped_low, 4);
    assert!(
        ids.iter().all(|&id| price(&sim, id) == 2_000.0),
        "band bottom"
    );
}

#[test]
fn the_default_is_stored_as_no_price_of_the_rooms_own() {
    let (mut sim, ids) = offices(GameMode::Modern, 3);
    batch(&mut sim, Kind::Office, BatchTarget::Price(15_000.0));
    assert!(ids.iter().all(|&id| unit(&sim, id).rent == Some(15_000.0)));
    let r = batch(&mut sim, Kind::Office, BatchTarget::Default);
    assert_eq!(r.changed, 3);
    assert!(ids.iter().all(|&id| unit(&sim, id).rent.is_none()));
    assert!(ids.iter().all(|&id| price(&sim, id) == 10_000.0));
    // A batch to the default price itself is stored the same way, so a later
    // default-only batch counts no room as hand-priced.
    batch(&mut sim, Kind::Office, BatchTarget::Price(10_000.0));
    assert!(ids.iter().all(|&id| unit(&sim, id).rent.is_none()));
    let r = sim
        .apply_rent_batch(Kind::Office, BatchTarget::Price(15_000.0), true)
        .unwrap();
    assert_eq!((r.skipped_custom, r.changed), (0, 3));
}

#[test]
fn the_default_only_filter_skips_a_hand_priced_room() {
    let (mut sim, ids) = offices(GameMode::Modern, 3);
    assert_eq!(sim.adjust_rent(ids[0], 1), Some(11_000.0));
    let r = sim
        .apply_rent_batch(Kind::Office, BatchTarget::Price(18_000.0), true)
        .unwrap();
    assert_eq!((r.skipped_custom, r.eligible), (1, 2));
    assert_eq!(price(&sim, ids[0]), 11_000.0, "kept");
    assert_eq!(price(&sim, ids[1]), 18_000.0);
    assert_eq!(price(&sim, ids[2]), 18_000.0);
}

#[test]
fn a_sold_condo_keeps_its_price() {
    let (mut sim, ids) = rooms(GameMode::Modern, 2, Kind::Condo, 2, 16);
    sell(&mut sim, ids[0]);
    let r = batch(&mut sim, Kind::Condo, BatchTarget::Price(200_000.0));
    assert_eq!((r.skipped_sold, r.matched, r.eligible), (1, 2, 1));
    assert_eq!(price(&sim, ids[0]), 160_000.0, "the sold one is untouched");
    assert_eq!(price(&sim, ids[1]), 200_000.0);
}

#[test]
fn a_quote_counts_what_the_batch_would_overwrite_and_writes_nothing() {
    let (mut sim, ids) = offices(GameMode::Modern, 3);
    sim.adjust_rent(ids[0], 1);
    let off = sim
        .compute_batch(Kind::Office, BatchTarget::Price(14_000.0), false, false)
        .unwrap();
    assert_eq!((off.custom_overwritten, off.skipped_custom), (1, 0));
    let on = sim
        .compute_batch(Kind::Office, BatchTarget::Price(14_000.0), true, false)
        .unwrap();
    assert_eq!((on.custom_overwritten, on.skipped_custom), (0, 1));
    assert_eq!(price(&sim, ids[1]), 10_000.0, "the quotes wrote nothing");

    let (mut sim, ids) = offices(GameMode::Modern, 4);
    let before = sim.serialize();
    let quote = sim
        .compute_batch(Kind::Office, BatchTarget::Price(13_000.0), false, false)
        .unwrap();
    assert_eq!(sim.serialize(), before);
    assert_eq!(
        batch(&mut sim, Kind::Office, BatchTarget::Price(13_000.0)),
        quote
    );
    assert!(ids.iter().all(|&id| price(&sim, id) == 13_000.0));
}

#[test]
fn an_unpriced_kind_has_no_batch_and_every_priced_kind_quotes_alike_twice() {
    let (mut sim, _) = offices(GameMode::Modern, 4);
    assert!(sim
        .compute_batch(Kind::Security, BatchTarget::Price(100.0), false, false)
        .is_none());
    assert!(sim
        .apply_rent_batch(Kind::Lobby, BatchTarget::Price(100.0), false)
        .is_none());
    let priced: Vec<Kind> = crate::facilities::FACILITIES
        .iter()
        .map(|f| f.kind)
        .filter(|k| crate::econ::rent_config(*k).is_some())
        .collect();
    assert!(priced.contains(&Kind::Office) && priced.contains(&Kind::Condo));
    for kind in priced {
        let a = sim.compute_batch(kind, BatchTarget::Price(12_345.0), false, false);
        assert!(a.is_some(), "{kind:?}");
        let b = sim.compute_batch(kind, BatchTarget::Price(12_345.0), false, false);
        assert_eq!(a, b, "{kind:?}");
    }
}

#[test]
fn a_modern_step_moves_one_band_step_and_stops_at_the_top() {
    let (mut sim, ids) = offices(GameMode::Modern, 1);
    assert_eq!(sim.adjust_rent(ids[0], 1), Some(11_000.0));
    assert_eq!(sim.adjust_rent(ids[0], -1), Some(10_000.0));
    for _ in 0..30 {
        sim.adjust_rent(ids[0], 1);
    }
    assert_eq!(price(&sim, ids[0]), 20_000.0);
}

#[test]
fn any_reprice_puts_a_no_rate_room_back_on_the_market() {
    let (mut sim, ids) = offices(GameMode::Classic, 2);
    assert!(sim.set_no_rate(ids[0]));
    assert_eq!(price(&sim, ids[0]), 0.0);
    sim.adjust_rent(ids[0], 1);
    assert!(!unit(&sim, ids[0]).no_rate);
    assert!(price(&sim, ids[0]) > 0.0);

    assert!(sim.set_no_rate(ids[0]));
    batch(&mut sim, Kind::Office, BatchTarget::Price(15_000.0));
    assert!(ids.iter().all(|&id| !unit(&sim, id).no_rate));
    assert!(ids.iter().all(|&id| price(&sim, id) == 15_000.0));
}

#[test]
fn a_skipped_sold_condo_keeps_its_no_rate_and_a_repriced_one_clears_it() {
    let (mut sim, ids) = rooms(GameMode::Classic, 2, Kind::Condo, 2, 16);
    sell(&mut sim, ids[0]);
    for &id in &ids {
        sim.tower.get_unit_mut(id).unwrap().no_rate = true;
    }
    let r = batch(&mut sim, Kind::Condo, BatchTarget::Price(200_000.0));
    assert_eq!(r.skipped_sold, 1);
    assert!(unit(&sim, ids[0]).no_rate, "skipped, kept");
    assert!(!unit(&sim, ids[1]).no_rate, "repriced, cleared");
    assert_eq!(price(&sim, ids[1]), 200_000.0);
}

#[test]
fn a_classic_target_lands_on_a_rung_or_snaps_to_the_nearest_and_never_clamps() {
    let (mut sim, ids) = offices(GameMode::Classic, 3);
    let r = batch(&mut sim, Kind::Office, BatchTarget::Price(5_000.0));
    assert_eq!((r.changed, r.clamped_low + r.clamped_high), (3, 0));
    assert!(ids.iter().all(|&id| price(&sim, id) == 5_000.0));
    // Off the ladder, ties go up.
    let r = batch(&mut sim, Kind::Office, BatchTarget::Price(7_500.0));
    assert_eq!(r.clamped_low + r.clamped_high, 0);
    assert!(ids.iter().all(|&id| price(&sim, id) == 10_000.0));
}

#[test]
fn a_no_rate_batch_takes_rooms_off_the_market_and_keeps_their_tenants() {
    let (mut sim, ids) = rooms(GameMode::Classic, 2, Kind::Office, 3, 9);
    {
        let u = sim.tower.get_unit_mut(ids[0]).unwrap();
        u.state = UnitState::Occupied;
        u.ever_occupied = true;
        u.occupants = 6;
    }
    let quote = sim
        .compute_batch(Kind::Office, BatchTarget::NoRate, false, false)
        .unwrap();
    assert_eq!(quote.changed, 3);
    assert!(
        ids.iter().all(|&id| !unit(&sim, id).no_rate),
        "a quote is pure"
    );
    assert_eq!(batch(&mut sim, Kind::Office, BatchTarget::NoRate), quote);
    assert!(ids.iter().all(|&id| unit(&sim, id).no_rate));
    let tenant = unit(&sim, ids[0]);
    assert_eq!((tenant.state, tenant.occupants), (UnitState::Occupied, 6));
    assert_eq!(price(&sim, ids[0]), 0.0);
    let again = sim
        .compute_batch(Kind::Office, BatchTarget::NoRate, false, false)
        .unwrap();
    assert_eq!(again.changed, 0);
}

#[test]
fn a_no_rate_batch_skips_a_sold_condo_and_modern_refuses_it_whole() {
    let (mut sim, ids) = rooms(GameMode::Classic, 3, Kind::Condo, 2, 16);
    sell(&mut sim, ids[0]);
    let r = batch(&mut sim, Kind::Condo, BatchTarget::NoRate);
    assert_eq!((r.skipped_sold, r.changed), (1, 1));
    assert!(!unit(&sim, ids[0]).no_rate);
    assert!(unit(&sim, ids[1]).no_rate);
    let mut modern = Simulation::new_game(3, GameMode::Modern);
    assert!(modern
        .compute_batch(Kind::Office, BatchTarget::NoRate, false, false)
        .is_none());
    assert!(modern
        .apply_rent_batch(Kind::Office, BatchTarget::NoRate, false)
        .is_none());
}

#[test]
fn on_the_ladder_the_default_only_filter_means_still_on_average() {
    let (mut sim, ids) = rooms(GameMode::Classic, 4, Kind::Office, 3, 9);
    sim.price_unit(ids[0], 15_000.0);
    sim.tower.get_unit_mut(ids[1]).unwrap().no_rate = true;
    let r = sim
        .apply_rent_batch(Kind::Office, BatchTarget::Price(5_000.0), true)
        .unwrap();
    assert_eq!((r.skipped_custom, r.changed), (2, 1));
    assert_eq!(price(&sim, ids[0]), 15_000.0);
    assert!(
        unit(&sim, ids[1]).no_rate,
        "the filter leaves an off-market room off"
    );
    assert_eq!(price(&sim, ids[2]), 5_000.0);
}

#[test]
fn a_classic_step_moves_one_rung_and_stops_at_each_end() {
    let (mut sim, ids) = rooms(GameMode::Classic, 5, Kind::Office, 1, 9);
    let id = ids[0];
    assert_eq!(price(&sim, id), 10_000.0, "a new build starts on Average");
    for (dir, want) in [
        (1, 15_000.0),
        (1, 15_000.0),
        (-1, 10_000.0),
        (-1, 5_000.0),
        (-1, 2_000.0),
        (-1, 2_000.0),
    ] {
        assert_eq!(sim.adjust_rent(id, dir), Some(want));
    }
}
