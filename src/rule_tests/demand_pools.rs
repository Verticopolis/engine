//! The commercial demand pools: the origins' demand is split among the
//! venues they reach and capped at each venue's capacity, and an attendance
//! venue earns from its live fill instead. Coverage of the unmet-demand
//! drain is `demand_tests`; the memo is
//! `diagnostics::memo_tests::the_demand_memo_rebuilds_on_each_key_input_and_holds_within_the_hour`.
use crate::clock::{Clock, GameMode};
use crate::demand::{compute_demand_map, origin_demand, unmet_coverage};
use crate::economy::{commercial_daily_income, commercial_demand_bid, daily_traffic_income};
use crate::facilities::{Kind, LOT_WIDTH};
use crate::rules::DEMAND_FLOOR_MODERN;
use crate::sim::Simulation;
use crate::tower::UnitState;

use super::MID as C;

/// Lays every tile of `floor` from the center out.
fn lay(sim: &mut Simulation, kind: Kind, floor: i64) {
    for x in (C..LOT_WIDTH).chain((0..C).rev()) {
        if sim.tower.structure_kind_at(floor, x) != Some(kind) {
            assert!(sim.tower.place(kind, floor, x).ok, "{kind:?} {floor}/{x}");
        }
    }
}

fn occupied(sim: &mut Simulation, kind: Kind, floor: i64, x: i64) -> i64 {
    let placed = sim.tower.place(kind, floor, x);
    assert!(placed.ok, "{kind:?} {floor}/{x}: {:?}", placed.reason);
    let id = placed.unit_id.unwrap();
    sim.tower.get_unit_mut(id).unwrap().state = UnitState::Occupied;
    id
}

/// A full lobby and floor 2, served by one elevator at the center.
fn served_tower(mode: GameMode) -> Simulation {
    let mut sim = Simulation::new_game(1, mode);
    sim.money = 1e12;
    sim.star = 5;
    lay(&mut sim, Kind::Lobby, 1);
    lay(&mut sim, Kind::Floor, 2);
    assert!(sim.build_transport(Kind::ElevatorStandard, C, 1, 2).ok);
    sim
}

/// `n` occupied offices from column 20 and one shop right of the shaft.
fn offices_and_shop(mode: GameMode, n: i64, spacing: i64, first: i64) -> (Simulation, i64) {
    let mut sim = served_tower(mode);
    for i in 0..n {
        occupied(&mut sim, Kind::Office, 2, first + i * spacing);
    }
    let shop = occupied(&mut sim, Kind::Shop, 2, C + 20);
    (sim, shop)
}

fn fraction(sim: &mut Simulation, id: i64) -> f64 {
    compute_demand_map(sim).fraction_by_unit[&id]
}

#[test]
fn the_pool_is_conserved_across_the_venues_it_reaches() {
    let (mut sim, _) = offices_and_shop(GameMode::Classic, 4, 12, 20);
    let pool = 4.0 * origin_demand(&sim, Kind::Office, None, 1.0);
    let delivered: f64 = compute_demand_map(&mut sim)
        .delivered_by_unit
        .sorted()
        .into_iter()
        .map(|(_, v)| v)
        .sum();
    assert!((delivered - pool).abs() < 1e-6, "{delivered} of {pool}");
}

#[test]
fn a_second_identical_venue_halves_the_first() {
    let (mut one, a) = offices_and_shop(GameMode::Classic, 4, 12, 20);
    let (mut two, b) = offices_and_shop(GameMode::Classic, 4, 12, 20);
    occupied(&mut two, Kind::Shop, 2, C + 40);
    assert!((fraction(&mut two, b) - fraction(&mut one, a) / 2.0).abs() < 1e-9);
}

#[test]
fn both_modes_cap_an_over_demanded_venue_on_the_same_scale() {
    // Classic pays the 1994 chart's figure but bids for demand at Modern's
    // (web#2631), so the same offices fill a shop in both modes.
    let pool = |sim: &Simulation| 20.0 * origin_demand(sim, Kind::Office, None, 1.0);
    for mode in [GameMode::Modern, GameMode::Classic] {
        let (mut sim, shop) = offices_and_shop(mode, 20, 9, 0);
        assert!(pool(&sim) > daily_traffic_income(Kind::Shop).unwrap());
        let map = compute_demand_map(&mut sim);
        assert_eq!(map.fraction_by_unit[&shop], 1.0, "{mode:?}");
        // Coverage still reads the pool against the income on offer.
        let income = commercial_daily_income(mode, Kind::Shop).unwrap();
        assert!((map.share - pool(&sim) / income).abs() < 1e-9, "{mode:?}");
    }
    assert!(
        commercial_daily_income(GameMode::Classic, Kind::Shop).unwrap()
            > daily_traffic_income(Kind::Shop).unwrap(),
        "Classic still pays its larger 1994 figure"
    );
}

#[test]
fn a_lone_classic_shop_covers_its_tenants_on_the_1994_scale() {
    // The owner kept Classic's coverage on the 1994 chart (engine#141): the
    // tenants' unmet-demand cap fires once the pool is twice what the shops
    // offer, so a lone shop's offices hit it at about 223 of them, its
    // $20,000 of 1994 income, and not at the 28 its bid would give.
    let (sim, _) = offices_and_shop(GameMode::Classic, 0, 0, 0);
    let per_office = origin_demand(&sim, Kind::Office, None, 1.0);
    let income = commercial_daily_income(GameMode::Classic, Kind::Shop).unwrap();
    let bid = daily_traffic_income(Kind::Shop).unwrap();
    assert_eq!(
        (2.0 * income / per_office).ceil(),
        223.0,
        "on the 1994 scale"
    );
    assert_eq!((2.0 * bid / per_office).ceil(), 28.0, "on the bid scale");
    for mode in [GameMode::Classic, GameMode::Modern] {
        let (mut sim, shop) = offices_and_shop(mode, 20, 9, 0);
        let office = sim
            .tower
            .units
            .iter()
            .find(|u| u.kind == Kind::Office)
            .cloned()
            .unwrap();
        let map = compute_demand_map(&mut sim);
        assert_eq!(
            map.fraction_by_unit[&shop], 1.0,
            "{mode:?}: the shop is full"
        );
        let covered = unmet_coverage(&map, &office).unwrap();
        match mode {
            GameMode::Classic => assert_eq!(covered, 1.0, "Classic: well short of 223"),
            GameMode::Modern => assert!(covered < 1.0, "Modern: the shop runs short"),
        }
    }
}

#[test]
fn a_stranded_origin_reaches_no_venue_and_an_asleep_guest_is_an_origin() {
    let mut sim = served_tower(GameMode::Classic);
    lay(&mut sim, Kind::Floor, 3);
    let reached = occupied(&mut sim, Kind::Office, 2, 20);
    let stranded = occupied(&mut sim, Kind::Office, 3, 20);
    let hotel = occupied(&mut sim, Kind::HotelSingle, 2, 40);
    sim.tower.get_unit_mut(hotel).unwrap().state = UnitState::Asleep;
    let shop = occupied(&mut sim, Kind::Shop, 2, C + 20);
    assert!(!sim.tower.is_floor_served(3));
    let map = compute_demand_map(&mut sim);
    assert_eq!(map.reachable_venues_by_origin[&reached], 1);
    assert_eq!(map.reachable_venues_by_origin[&stranded], 0);
    assert_eq!(map.reachable_venues_by_origin[&hotel], 1);
    assert!(map.fraction_by_unit[&shop] > 0.0);
}

#[test]
fn classic_starves_an_empty_towers_venue_and_modern_keeps_its_floor() {
    let (mut classic, shop) = offices_and_shop(GameMode::Classic, 0, 0, 0);
    assert_eq!(fraction(&mut classic, shop), 0.0);
    let (mut modern, shop) = offices_and_shop(GameMode::Modern, 0, 0, 0);
    assert_eq!(fraction(&mut modern, shop), DEMAND_FLOOR_MODERN);
}

#[test]
fn below_the_cap_both_modes_earn_the_same_share() {
    let (mut classic, c_shop) = offices_and_shop(GameMode::Classic, 7, 12, 20);
    let (mut modern, m_shop) = offices_and_shop(GameMode::Modern, 7, 12, 20);
    let (c_map, m_map) = (
        compute_demand_map(&mut classic),
        compute_demand_map(&mut modern),
    );
    let (c_frac, m_frac) = (
        c_map.fraction_by_unit[&c_shop],
        m_map.fraction_by_unit[&m_shop],
    );
    assert!((c_frac - m_frac).abs() < 1e-9);
    assert!(m_map.share > DEMAND_FLOOR_MODERN && m_map.share < 1.0);
    assert!((m_frac - m_map.share).abs() < 1e-9);
    // Classic's coverage share reads the same pool against its 1994 income.
    let scale = daily_traffic_income(Kind::Shop).unwrap()
        / commercial_daily_income(GameMode::Classic, Kind::Shop).unwrap();
    assert!((c_map.share - m_map.share * scale).abs() < 1e-9);
}

#[test]
fn every_kind_that_earns_a_daily_income_bids_for_demand() {
    // A venue with an income but no bid would drop out of the pool and earn
    // nothing, with no error.
    for mode in [GameMode::Classic, GameMode::Modern] {
        for f in crate::facilities::FACILITIES {
            let (income, bid) = (
                commercial_daily_income(mode, f.kind),
                commercial_demand_bid(mode, f.kind),
            );
            assert_eq!(bid.is_some(), income.is_some(), "{mode:?} {:?}", f.kind);
            if mode == GameMode::Modern {
                assert_eq!(bid, income, "Modern bids what it earns: {:?}", f.kind);
            }
        }
    }
}

#[test]
fn a_cinema_neither_joins_the_pool_nor_dilutes_a_shop() {
    let (mut base, base_shop) = offices_and_shop(GameMode::Classic, 4, 12, 20);
    let (mut with, shop) = offices_and_shop(GameMode::Classic, 4, 12, 20);
    lay(&mut with, Kind::Floor, 3);
    let cinema = occupied(&mut with, Kind::Cinema, 2, C + 40);
    let map = compute_demand_map(&mut with);
    assert!(!map.fraction_by_unit.contains_key(&cinema));
    assert!(!map.delivered_by_unit.contains_key(&cinema));
    assert!((map.fraction_by_unit[&shop] - fraction(&mut base, base_shop)).abs() < 1e-9);
}

/// What a cinema with `customers` in its seats earns in one noon collection.
fn cinema_takings(customers: i64, blockbuster: bool) -> f64 {
    let mut sim = Simulation::new_game(1, GameMode::Classic);
    for x in 0..40 {
        assert!(sim.tower.place(Kind::Lobby, 1, x).ok);
        assert!(sim.tower.place(Kind::Floor, 2, x).ok);
        assert!(sim.tower.place(Kind::Floor, 3, x).ok);
    }
    assert!(
        sim.tower
            .place_transport(Kind::ElevatorStandard, 0, 1, 2)
            .ok
    );
    let cinema = occupied(&mut sim, Kind::Cinema, 2, 5);
    sim.tower.get_unit_mut(cinema).unwrap().customers_in = Some(customers);
    if blockbuster {
        sim.blockbusters.push(cinema as f64);
    }
    sim.clock = Clock::new(12.0 * 60.0, sim.clock.calendar);
    sim.money = 0.0;
    sim.collect_traffic_income();
    sim.money
}

#[test]
fn a_cinema_earns_from_its_fill_capped_at_a_full_house_with_the_blockbuster_on_top() {
    let seats = Kind::Cinema.attendance_cap().unwrap();
    assert_eq!(cinema_takings(0, false), 0.0);
    let half = cinema_takings(seats / 2, false);
    let full = cinema_takings(seats, false);
    assert!(half > 0.0 && full > half);
    assert_eq!(cinema_takings(seats * 2, false), full);
    let block = cinema_takings(seats, true);
    assert!((block / full - 2.2).abs() < 0.05, "{block} / {full}");
    assert_eq!(cinema_takings(seats * 2, true), block);
}

#[test]
fn the_venue_card_reads_its_local_demand_share() {
    let (mut sim, shop) = offices_and_shop(GameMode::Classic, 4, 12, 20);
    // Rounded down, so the percent never claims a threshold the advice
    // has not reached (#549).
    let pct = (fraction(&mut sim, shop) * 100.0 + 1e-9).floor() as i64;
    assert!(pct > 0);
    let facts = serde_json::to_value(sim.service_diagnostic_facts(shop).unwrap()).unwrap();
    assert_eq!(facts["retail"]["localDemandPct"], pct);
}
