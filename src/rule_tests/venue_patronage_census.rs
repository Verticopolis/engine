//! A Modern venue whose customers no meal trip, show or walk-in brings in
//! person (the amusements, boutique bay, nightclub, spa, sky bar and
//! daycare while walk-ins are off) keeps a patronage tally like a shop, shows
//! it on its card, and counts the customers it served the day before toward
//! the population, held to its size (owner, 2026-11-25). Its card no longer
//! offers a live head count that nobody ever raises: a player reported spas
//! that always read 0 visitors.
use crate::clock::{CalendarKind, GameMode};
use crate::economy::visitors::{census_counts_patronage, census_follows_the_day, patronage_census};
use crate::facilities::{Kind, FACILITIES};
use crate::sim::Simulation;
use crate::tower::UnitState;

use super::{at_time, room, served_tower, LEFT};

/// The venues that count their patronage while walk-ins are off.
const PATRONAGE_VENUES: [Kind; 6] = [
    Kind::Amusements,
    Kind::BoutiqueBay,
    Kind::Nightclub,
    Kind::Spa,
    Kind::SkyBar,
    Kind::Daycare,
];

/// A Modern tower to floor 6 with offices and hotel rooms for demand, a spa
/// and a nightclub on floor 2 and a fast food on floor 6, all open.
fn venue_tower() -> (Simulation, i64, i64, i64) {
    let mut sim = served_tower(GameMode::Modern, CalendarKind::RealWorld, 6);
    for i in 0..6 {
        room(
            &mut sim,
            Kind::Office,
            3,
            LEFT + 4 + i * 9,
            UnitState::Occupied,
        );
        room(
            &mut sim,
            Kind::HotelSingle,
            4,
            LEFT + 4 + i * 4,
            UnitState::Occupied,
        );
    }
    let spa = room(&mut sim, Kind::Spa, 2, LEFT + 4, UnitState::Occupied);
    let club = room(&mut sim, Kind::Nightclub, 2, LEFT + 30, UnitState::Occupied);
    let food = room(&mut sim, Kind::FastFood, 6, LEFT + 4, UnitState::Occupied);
    (sim, spa, club, food)
}

fn census(sim: &Simulation, id: i64) -> i64 {
    sim.tower.get_unit(id).unwrap().census_share()
}

#[test]
fn only_the_venues_nobody_visits_in_person_count_their_patronage() {
    for f in FACILITIES.iter() {
        assert_eq!(
            census_counts_patronage(f.kind),
            PATRONAGE_VENUES.contains(&f.kind),
            "{}",
            f.name
        );
    }
}

#[test]
fn a_venue_counts_whole_customers_held_to_its_size() {
    assert_eq!(patronage_census(Kind::Spa, 7.9), 7);
    assert_eq!(
        patronage_census(Kind::Spa, 500.0),
        Kind::Spa.facility().population
    );
    assert_eq!(
        patronage_census(Kind::Nightclub, 500.0),
        Kind::Nightclub.facility().population
    );
    assert_eq!(patronage_census(Kind::Spa, -3.0), 0);
    assert_eq!(patronage_census(Kind::Spa, f64::NAN), 0);
}

#[test]
fn a_spa_counts_yesterdays_customers_toward_the_population() {
    let (mut sim, spa, club, _) = venue_tower();
    at_time(&mut sim, 0, 8);
    // Through the first day nothing has closed, so the spa counts nobody.
    for _ in 0..15 {
        sim.tick(60.0);
    }
    assert_eq!(sim.clock.hour(), 23);
    assert_eq!(census(&sim, spa), 0);
    let served = sim
        .tower
        .get_unit(spa)
        .unwrap()
        .extras()
        .patronage_today
        .unwrap();
    assert!(served > 0.0, "the spa's demand sends it customers");
    let before = sim.tower.total_population();
    // Past midnight the day's customers carry into the census.
    for _ in 0..2 {
        sim.tick(60.0);
    }
    let u = sim.tower.get_unit(spa).unwrap();
    let carried = patronage_census(Kind::Spa, u.extras().patronage_yest.unwrap());
    assert!(carried > 0);
    assert_eq!(u.visitors_yest, Some(carried as u8));
    assert_eq!(census(&sim, spa), carried);
    assert!(
        census(&sim, club) > 0,
        "the nightclub counts its patronage too"
    );
    assert!(sim.tower.total_population() > before);
}

#[test]
fn a_saved_spa_keeps_its_tally_and_its_census() {
    let (mut sim, spa, _, _) = venue_tower();
    for _ in 0..30 {
        sim.tick(60.0);
    }
    let before = sim.tower.get_unit(spa).unwrap().clone();
    assert!(before.extras().patronage_yest.is_some());
    let loaded = crate::load::deserialize(&sim.serialize()).unwrap();
    let after = loaded.tower.get_unit(spa).unwrap();
    assert_eq!(after.visitors_yest, before.visitors_yest);
    assert_eq!(
        after.extras().patronage_today,
        before.extras().patronage_today
    );
    assert_eq!(
        after.extras().patronage_yest,
        before.extras().patronage_yest
    );
    assert_eq!(
        loaded.tower.total_population(),
        sim.tower.total_population()
    );
}

#[test]
fn the_card_shows_a_live_count_only_where_people_come_in_person() {
    let (mut sim, spa, _, food) = venue_tower();
    let detail = |sim: &mut Simulation, id| sim.unit_detail(id).unwrap()["customersIn"].clone();
    // A fast food nobody has reached yet reads 0 people inside, not null.
    assert_eq!(sim.tower.get_unit(food).unwrap().customers_in, None);
    assert_eq!(detail(&mut sim, food), 0);
    // A spa reads null even with a count on the unit: nobody is sent to one in
    // person, so the card has no live head count to show.
    let (u, at) = sim.tower.marked_by_id(spa).unwrap();
    u.set_customers_in(at, Some(5));
    assert!(detail(&mut sim, spa).is_null());
    for _ in 0..12 {
        sim.tick(60.0);
    }
    let facts = serde_json::to_value(sim.service_diagnostic_facts(spa).unwrap()).unwrap();
    assert_eq!(
        facts["retail"]["active"], true,
        "the spa's card shows its patronage"
    );
    assert!(facts["retail"]["today"].as_i64().unwrap() > 0);
}

#[test]
fn with_walk_ins_on_the_venues_count_their_live_customers_again() {
    // Walk-ins are off in play (`WALK_INS_ON`); this covers them on.
    crate::facilities::set_walk_ins_for_test(true);
    for kind in [
        Kind::Amusements,
        Kind::BoutiqueBay,
        Kind::Nightclub,
        Kind::Spa,
        Kind::SkyBar,
    ] {
        assert!(!census_counts_patronage(kind), "{kind:?}");
        assert!(!census_follows_the_day(kind), "{kind:?}");
    }
    // Nobody brings a daycare's children as walk-ins, so it still counts
    // its patronage.
    assert!(census_counts_patronage(Kind::Daycare));
    crate::facilities::set_walk_ins_for_test(false);
}

#[test]
fn a_venues_patronage_adds_nothing_to_the_recycling_plan() {
    // Its customers mostly live or work in the tower, as with a food hall's
    // (#358), so recycling plans for the population without them.
    let (mut sim, spa, _, _) = venue_tower();
    at_time(&mut sim, 0, 8);
    for _ in 0..18 {
        sim.tick(60.0);
    }
    assert!(census(&sim, spa) > 0);
    assert_eq!(sim.tower.get_unit(spa).unwrap().recycling_census_share(), 0);
}
