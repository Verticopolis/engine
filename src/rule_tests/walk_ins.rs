//! Shops and restaurants take real customers from outside the tower
//! (`crowd::walk_ins`): each hour books its share of the day's visitors,
//! up to the 1994 game's daily visitors, each one comes in at the ground
//! lobby (or a Metro platform) and rides to the venue, and Today's patronage
//! still follows the demand. The venue draws only who is inside, and the
//! walk-ins never crowd the tenants' sample out.
use crate::clock::GameMode;
use crate::crowd::motion::advance;
use crate::crowd::spawn::add;
use crate::crowd::walk_ins::{start_walk_ins, WalkIn, WALK_IN_GIVE_UP};
use crate::crowd::{PState, MAX_PEOPLE};
use crate::facilities::Kind;
use crate::sim::Simulation;
use crate::tower::UnitState;

use super::{at_time, place_seated, shaft, strip_tower};

const SHOP_FLOOR: i64 = 4;

/// A Classic strip to floor 4 with an elevator at x 36, a row of staffed
/// offices on floor 2 (the shop's demand) and an open shop on floor 4.
/// Returns the sim and the shop.
fn shop_tower(seed: u32) -> (Simulation, i64) {
    let mut sim = strip_tower(seed, GameMode::Classic, 40, SHOP_FLOOR);
    sim.star = 3;
    shaft(&mut sim, Kind::ElevatorStandard, 36, 1, SHOP_FLOOR);
    let staff = Kind::Office.facility().population;
    for x in [0, 9, 18, 27] {
        place_seated(&mut sim, Kind::Office, 2, x, UnitState::Occupied, staff);
    }
    let shop = place_seated(&mut sim, Kind::Shop, SHOP_FLOOR, 0, UnitState::Occupied, 0);
    (sim, shop)
}

fn shop_customers(sim: &Simulation, shop: i64) -> (i64, i64, f64) {
    let u = sim.tower.get_unit(shop).unwrap();
    (
        u.customers_in.unwrap_or(0),
        u.occupants,
        u.extras().patronage_today.unwrap_or(0.0),
    )
}

#[test]
fn shoppers_walk_in_from_the_lobby_and_patronage_follows_the_demand() {
    let (mut sim, shop) = shop_tower(7);
    at_time(&mut sim, 0, 10);
    let mut seen = std::collections::HashSet::new();
    let mut arrived = std::collections::HashSet::new();
    for _ in 0..(3 * 60) {
        sim.tick(1.0);
        for p in sim.crowd.people.iter().filter(|p| p.walk_in) {
            if seen.insert(p.id) {
                assert_eq!(p.floor, 1, "a walk-in first stands in the lobby");
                assert_eq!(p.meal_venue_id, Some(shop));
            }
            if p.state == PState::Dwelling {
                arrived.insert(p.id);
            }
        }
        let (inside, drawn, _) = shop_customers(&sim, shop);
        assert_eq!(drawn, inside, "the shop draws who is inside");
        // Inside means there now: a shopper on the way home is out.
        let browsing = sim
            .crowd
            .people
            .iter()
            .filter(|p| p.state == PState::Dwelling && p.venue_unit_id == Some(shop))
            .count() as i64;
        assert_eq!(inside, browsing, "the shop counts only who is in it");
    }
    assert!(!arrived.is_empty(), "shoppers reached the shop");
    let (_, _, patronage) = shop_customers(&sim, shop);
    assert!(
        patronage >= arrived.len() as f64,
        "the shoppers who came are some of Today's {patronage}"
    );
}

/// A shop on floor 2 under `office_floors` floors of offices. Returns the
/// walk-ins booked for it from 10:00 to 20:00, each hour's bookings read on
/// their own.
fn day_of_bookings(seed: u32, office_floors: i64) -> usize {
    let top = 2 + office_floors;
    let mut sim = strip_tower(seed, GameMode::Classic, 40, top);
    shaft(&mut sim, Kind::ElevatorStandard, 36, 1, top);
    let shop = place_seated(&mut sim, Kind::Shop, 2, 0, UnitState::Occupied, 0);
    let staff = Kind::Office.facility().population;
    for floor in 3..=top {
        for x in [0, 9, 18, 27] {
            place_seated(&mut sim, Kind::Office, floor, x, UnitState::Occupied, staff);
        }
    }
    let mut booked = 0;
    for hour in 10..21 {
        at_time(&mut sim, 0, hour);
        sim.crowd.walk_ins.clear();
        sim.collect_traffic_income();
        booked += sim
            .crowd
            .walk_ins
            .iter()
            .filter(|w| w.unit_id == shop)
            .count();
    }
    booked
}

#[test]
fn a_day_books_the_shoppers_the_1994_game_served() {
    // About 25 a weekday for a shop: each hour books its share of the day,
    // rounded up or down at random, however many offices stand above it.
    let day = crate::crowd::walk_ins::daily_walk_ins(Kind::Shop, false).unwrap();
    assert_eq!(day, 25.0);
    for seed in 0..6 {
        for office_floors in [1, 6] {
            let booked = day_of_bookings(seed, office_floors) as f64;
            assert!((booked - day).abs() <= 5.0, "{booked} booked");
        }
    }
    // A shop nobody in the tower shops at books nobody.
    assert_eq!(day_of_bookings(0, 0), 0);
}

#[test]
fn walk_ins_leave_room_for_the_tenants_sample() {
    let (mut sim, shop) = shop_tower(3);
    at_time(&mut sim, 0, 12);
    for _ in 0..30 {
        sim.tick(1.0);
    }
    let walk_ins = sim.crowd.people.iter().filter(|p| p.walk_in).count() as i64;
    assert!(walk_ins > 0, "shoppers are on their way to {shop}");
    assert_eq!(sim.crowd.walk_in_count, walk_ins, "the count follows them");
    let sample = sim.crowd.people.len() as i64 - walk_ins;
    assert!(sample <= MAX_PEOPLE as i64);
}

#[test]
fn a_shop_that_closes_takes_no_more_bookings_and_its_shoppers_go_home() {
    let (mut sim, shop) = shop_tower(5);
    at_time(&mut sim, 0, 20);
    for _ in 0..(3 * 60) {
        sim.tick(1.0);
    }
    assert!(!Kind::Shop.is_open_at(sim.clock.hour()));
    assert!(sim.crowd.walk_ins.iter().all(|w| w.unit_id != shop));
    assert!(
        sim.crowd.people.iter().all(|p| !p.walk_in),
        "every shopper has gone home by the lobby"
    );
    assert_eq!(sim.crowd.walk_in_count, 0);
    assert_eq!(shop_customers(&sim, shop).0, 0);
}

#[test]
fn shoppers_split_between_the_lobby_and_a_served_metro_platform() {
    use super::metro_shoppers::{b, metro_tower};
    let mut sim = metro_tower(GameMode::Classic, true);
    for x in [10, 30, 50, 70] {
        let id = super::put(&mut sim, Kind::Office, 2, x);
        sim.tower.get_unit_mut(id).unwrap().state = UnitState::Occupied;
    }
    let shop = super::put(&mut sim, Kind::Shop, b(7), super::MID + 10);
    sim.tower.get_unit_mut(shop).unwrap().state = UnitState::Occupied;
    let (mut by_metro, mut by_lobby) = (0, 0);
    let mut seen = std::collections::HashSet::new();
    for day in 0..3 {
        at_time(&mut sim, day, 10);
        for _ in 0..(11 * 60) {
            sim.tick(1.0);
            for p in sim.crowd.people.iter().filter(|p| p.walk_in) {
                if seen.insert(p.id) {
                    if p.from_metro {
                        by_metro += 1;
                    } else {
                        assert_eq!(p.floor, 1, "the rest come in at the lobby");
                        by_lobby += 1;
                    }
                }
            }
        }
    }
    // The lobby's two elevators carry twice what the platform's one does.
    assert!(
        by_metro > 0 && by_lobby > by_metro,
        "{by_lobby} / {by_metro}"
    );
}

/// Books `n` walk-ins to `shop` for this minute and starts the ones the
/// door lets in. Returns how many started.
fn start_now(sim: &mut Simulation, shop: i64, n: usize) -> usize {
    let before = sim.crowd.walk_in_count;
    let now = sim.clock.minutes;
    sim.crowd.walk_ins.clear();
    for _ in 0..n {
        sim.crowd.walk_ins.push(WalkIn {
            at: now,
            unit_id: shop,
        });
    }
    // The door reads the step's queue view; a test that moved people by
    // hand builds it again, as the next step would.
    sim.crowd.queue_cache = None;
    start_walk_ins(&mut sim.crowd, &sim.tower, &sim.clock);
    (sim.crowd.walk_in_count - before) as usize
}

#[test]
fn a_full_lobby_line_keeps_the_shoppers_away() {
    let (mut sim, shop) = shop_tower(9);
    let lift = sim
        .tower
        .transports
        .iter()
        .find(|t| t.kind.is_elevator())
        .unwrap();
    assert_eq!(lift.cars, 1, "one car takes one load from the lobby");
    let load = Kind::ElevatorStandard.car_capacity() as usize;
    at_time(&mut sim, 0, 12);
    // A car's load less one already waits in the lobby: one shopper joins
    // the line, and the rest stay away.
    for _ in 0..load - 1 {
        let i = add(&mut sim.crowd, &sim.tower, 1, 2, None, None).unwrap();
        sim.crowd.people[i].state = PState::Waiting;
    }
    assert_eq!(start_now(&mut sim, shop, 5), 1);
    assert!(
        sim.crowd.walk_ins.is_empty(),
        "the turned-away bookings are spent"
    );
    // With the line gone, the shoppers come in again.
    for p in sim.crowd.people.iter_mut().filter(|p| !p.walk_in) {
        p.state = PState::Done;
    }
    assert_eq!(start_now(&mut sim, shop, 5), 5);
}

#[test]
fn a_shopper_who_waits_too_long_at_the_door_walks_back_out() {
    let (mut sim, shop) = shop_tower(4);
    at_time(&mut sim, 0, 12);
    assert_eq!(start_now(&mut sim, shop, 1), 1);
    let tenant = add(&mut sim.crowd, &sim.tower, 1, 2, None, None).unwrap();
    let shopper = sim.crowd.people.iter().position(|p| p.walk_in).unwrap();
    // Both have waited past a shopper's patience, well short of a tenant's
    // (`GIVE_UP`).
    for i in [tenant, shopper] {
        let p = &mut sim.crowd.people[i];
        p.state = PState::Waiting;
        p.age = WALK_IN_GIVE_UP + 1.0;
    }
    // The shopper's long wait would sour the tower's mood if it counted;
    // the tenant has only just come to the line.
    sim.crowd.people[shopper].wait = WALK_IN_GIVE_UP + 1.0;
    sim.crowd.people[tenant].wait = 0.0;
    sim.crowd.frustration = 0.0;
    advance(&mut sim.crowd, 0.1, &mut sim.tower);
    assert!(sim.crowd.people[shopper].gave_up, "the shopper turns back");
    assert!(
        !sim.crowd.people[tenant].gave_up,
        "the tenant keeps waiting"
    );
    assert_eq!(
        sim.crowd.frustration, 0.0,
        "only tenants' waits weigh on the mood"
    );
}

#[test]
fn shoppers_on_their_way_to_the_car_count_in_its_line() {
    let (mut sim, shop) = shop_tower(9);
    at_time(&mut sim, 0, 12);
    let load = Kind::ElevatorStandard.car_capacity() as usize;
    // One step lets in a full load, and no more.
    assert_eq!(start_now(&mut sim, shop, load + 3), load);
    assert!(sim
        .crowd
        .people
        .iter()
        .filter(|p| p.walk_in)
        .all(|p| p.state == PState::ToShaft));
    // The next step still sees them walking to the car, so nobody joins.
    assert_eq!(start_now(&mut sim, shop, 3), 0);
}

#[test]
fn a_shop_up_a_flight_of_stairs_forms_no_line_at_the_door() {
    // No elevator stops here, only stairs, where nobody queues.
    let mut sim = strip_tower(5, GameMode::Classic, 40, 2);
    sim.star = 3;
    shaft(&mut sim, Kind::Stairs, 30, 1, 2);
    let shop = place_seated(&mut sim, Kind::Shop, 2, 0, UnitState::Occupied, 0);
    at_time(&mut sim, 0, 12);
    let load = Kind::ElevatorStandard.car_capacity() as usize;
    assert_eq!(start_now(&mut sim, shop, load + 3), load + 3);
}

#[test]
fn a_modern_save_with_walk_ins_in_flight_keeps_a_tenants_patience() {
    // A Modern save from before walk-ins went off may still carry some.
    let (mut sim, shop) = shop_tower(4);
    at_time(&mut sim, 0, 12);
    assert_eq!(start_now(&mut sim, shop, 1), 1);
    sim.tower.mode = GameMode::Modern;
    let shopper = sim.crowd.people.iter().position(|p| p.walk_in).unwrap();
    let p = &mut sim.crowd.people[shopper];
    p.state = PState::Waiting;
    p.age = WALK_IN_GIVE_UP + 1.0;
    p.wait = WALK_IN_GIVE_UP + 1.0;
    sim.crowd.frustration = 0.0;
    advance(&mut sim.crowd, 0.1, &mut sim.tower);
    assert!(!sim.crowd.people[shopper].gave_up, "they wait as before");
    assert!(
        sim.crowd.frustration > 0.0,
        "and their wait counts, as before"
    );
}
