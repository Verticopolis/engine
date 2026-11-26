//! A commercial venue's hourly head count: a meal venue shows the customers
//! inside it while open and nobody once they leave, a shop shows its
//! shoppers the same way (and its full population while walk-ins are off,
//! since no trip is sent to it then), every venue shows 0 outside its hours, a never-visited venue opens
//! in the hour traffic first reaches it, and the floor congestion reads the
//! live customers rather than the catalog population.
use crate::clock::GameMode;
use crate::facilities::Kind;
use crate::sim::Simulation;
use crate::tower::UnitState;

use super::{at_time, place_seated, shaft, strip_tower};

const FLOOR: i64 = 3;

/// A Classic strip to floor 5 served by an elevator, with one tenanted
/// `kind` on floor 3 and the clock at `hour` on a Monday.
fn venue(kind: Kind, hour: i64, state: UnitState) -> (Simulation, i64) {
    let mut sim = strip_tower(2024, GameMode::Classic, 40, 5);
    shaft(&mut sim, Kind::ElevatorStandard, 30, 1, 5);
    let id = place_seated(&mut sim, kind, FLOOR, 0, state, 0);
    at_time(&mut sim, 0, hour);
    (sim, id)
}

fn seat(sim: &mut Simulation, id: i64, n: i64) {
    let mode = sim.tower.mode;
    let (u, at) = sim.tower.marked_by_id(id).unwrap();
    u.set_customers_in(at, Some(n));
    u.sync_visitor_occupants(at, mode);
}

fn customers(sim: &Simulation, id: i64) -> i64 {
    sim.tower.get_unit(id).unwrap().customers_in.unwrap_or(0)
}

fn occupants(sim: &Simulation, id: i64) -> i64 {
    sim.tower.get_unit(id).unwrap().occupants
}

fn next_hour(sim: &mut Simulation) {
    sim.tick(60.0);
}

#[test]
fn venue_populations_scale_with_footprint() {
    assert_eq!(Kind::Shop.facility().population, 20);
    assert_eq!(Kind::FastFood.facility().population, 25);
    assert_eq!(Kind::Restaurant.facility().population, 35);
}

#[test]
fn an_open_meal_venue_shows_its_customers_not_its_population() {
    let (mut sim, food) = venue(Kind::FastFood, 11, UnitState::Occupied);
    seat(&mut sim, food, 3);
    next_hour(&mut sim);
    assert!(occupants(&sim, food) > 0);
    assert_eq!(occupants(&sim, food), customers(&sim, food));
    assert!(occupants(&sim, food) < Kind::FastFood.facility().population);

    let (mut sim, restaurant) = venue(Kind::Restaurant, 11, UnitState::Occupied);
    next_hour(&mut sim);
    assert_eq!(occupants(&sim, restaurant), customers(&sim, restaurant));
}

#[test]
fn an_open_shop_shows_its_shoppers_not_its_population() {
    let (mut sim, shop) = venue(Kind::Shop, 13, UnitState::Occupied);
    seat(&mut sim, shop, 2);
    next_hour(&mut sim);
    assert_eq!(occupants(&sim, shop), customers(&sim, shop));
    assert!(occupants(&sim, shop) < Kind::Shop.facility().population);
}

#[test]
fn an_open_shop_draws_its_full_population_while_walk_ins_are_off() {
    // Walk-ins are on in play (`WALK_INS_ON`); this covers them off.
    crate::facilities::set_walk_ins_for_test(false);
    let (mut sim, shop) = venue(Kind::Shop, 13, UnitState::Occupied);
    seat(&mut sim, shop, 2);
    next_hour(&mut sim);
    assert_eq!(customers(&sim, shop), 2);
    assert_eq!(occupants(&sim, shop), Kind::Shop.facility().population);
}

#[test]
fn a_tenanted_venue_shows_nobody_outside_its_hours() {
    for (kind, hour) in [
        (Kind::FastFood, 3),
        (Kind::Restaurant, 8),
        (Kind::Restaurant, 2),
        (Kind::Shop, 21),
    ] {
        let (mut sim, id) = venue(kind, hour, UnitState::Occupied);
        next_hour(&mut sim);
        assert_eq!(sim.tower.get_unit(id).unwrap().state, UnitState::Occupied);
        assert_eq!(occupants(&sim, id), 0, "{kind:?} at {}:00", hour + 1);
    }
}

#[test]
fn a_fast_food_empties_with_its_customers_and_reopens_empty() {
    let (mut sim, food) = venue(Kind::FastFood, 20, UnitState::Occupied);
    seat(&mut sim, food, 4);
    next_hour(&mut sim);
    assert_eq!(occupants(&sim, food), customers(&sim, food));
    seat(&mut sim, food, 0);
    next_hour(&mut sim);
    assert_eq!(occupants(&sim, food), 0);
    sim.tick(8.0 * 60.0);
    assert_eq!(sim.clock.hour(), 6);
    next_hour(&mut sim);
    assert_eq!(occupants(&sim, food), customers(&sim, food));
    assert!(occupants(&sim, food) < Kind::FastFood.facility().population);
}

#[test]
fn a_never_visited_venue_opens_in_the_hour_traffic_first_reaches_it() {
    let (mut sim, food) = venue(Kind::FastFood, 6, UnitState::Empty);
    next_hour(&mut sim);
    assert_eq!(sim.tower.get_unit(food).unwrap().state, UnitState::Occupied);
    assert_eq!(occupants(&sim, food), customers(&sim, food));
    assert!(occupants(&sim, food) < Kind::FastFood.facility().population);

    let (mut sim, shop) = venue(Kind::Shop, 9, UnitState::Empty);
    next_hour(&mut sim);
    assert_eq!(sim.tower.get_unit(shop).unwrap().state, UnitState::Occupied);
    assert_eq!(occupants(&sim, shop), customers(&sim, shop));
}

#[test]
fn floor_congestion_reads_live_customers_never_the_catalog_population() {
    let mut sim = strip_tower(2024, GameMode::Modern, 40, 2);
    let lift = shaft(&mut sim, Kind::ElevatorStandard, 30, 1, 2);
    sim.tower.set_cars(lift, 1);
    let food = place_seated(&mut sim, Kind::FastFood, 2, 0, UnitState::Occupied, 0);
    at_time(&mut sim, 0, 12);
    let reading = |sim: &Simulation| {
        sim.spatial_congestion_by_floor()
            .get(&2)
            .copied()
            .unwrap_or(0.0)
    };
    assert_eq!(reading(&sim), 0.0);
    seat(&mut sim, food, 25);
    assert!(reading(&sim) > 0.0);
}
