//! The attendance tallies against the live crowd through a mixed day and the
//! teardowns a player can cause mid-visit (Verticopolis/web#302): a bulldozed
//! hotel room whose guest is out, a venue bulldozed under counted visitors and
//! rebuilt, the only elevator cut, and a fire that guts the venue. At every
//! check each room's `customersIn`, `hotelCustomersIn` and `outForMeal`
//! equals the live people carrying the matching stamp. Moved here from the
//! game's tripwire suite.
use super::attendance::{mixed_tower, unit_id};
use crate::clock::Clock;
use crate::crowd::PState;
use crate::facilities::Kind;
use crate::gameplay::RemovalMethod;
use crate::sim::Simulation;
use crate::tower::UnitState;

fn set_clock(sim: &mut Simulation, day: i64, hour: i64) {
    sim.clock = Clock::new(((day * 24 + hour) * 60) as f64, sim.clock.calendar);
}

fn live(sim: &Simulation, pick: impl Fn(&crate::crowd::Person) -> bool) -> i64 {
    sim.crowd
        .people
        .iter()
        .filter(|p| p.state != PState::Done && pick(p))
        .count() as i64
}

/// Every room's three tallies against its live people.
fn reconcile(sim: &Simulation) {
    for u in sim.tower.room_units() {
        let id = u.id;
        assert_eq!(
            u.customers_in.unwrap_or(0),
            live(sim, |p| p.venue_unit_id == Some(id)),
            "customersIn on {:?} #{id}",
            u.kind
        );
        assert_eq!(
            u.hotel_customers_in.unwrap_or(0),
            live(sim, |p| p.venue_unit_id == Some(id)
                && p.counted_hotel_guest),
            "hotelCustomersIn on {:?} #{id}",
            u.kind
        );
        assert_eq!(
            u.out_for_meal.unwrap_or(0),
            live(sim, |p| p.origin_unit_id == Some(id)),
            "outForMeal on {:?} #{id}",
            u.kind
        );
    }
}

fn customers(sim: &Simulation, id: i64) -> i64 {
    sim.tower.get_unit(id).unwrap().customers_in.unwrap_or(0)
}

/// The evening until the party hall seats a counted audience.
fn hall_with_audience() -> (Simulation, i64) {
    let mut sim = mixed_tower();
    set_clock(&mut sim, 0, 18);
    let hall = unit_id(&sim, Kind::PartyHall);
    for _ in 0..450 {
        if customers(&sim, hall) > 0 {
            break;
        }
        sim.tick(1.0);
    }
    assert!(customers(&sim, hall) > 0);
    assert_eq!(
        live(&sim, |p| p.venue_unit_id == Some(hall)),
        customers(&sim, hall)
    );
    (sim, hall)
}

#[test]
fn the_tallies_match_the_live_crowd_through_a_mixed_day() {
    let mut sim = mixed_tower();
    let (mut out_for_meal, mut hotel_split, mut any) = (false, false, false);
    for (hour, day) in [(11, 0), (18, 1)] {
        set_clock(&mut sim, day, hour);
        for m in 0..480 {
            sim.tick(1.0);
            if m % 10 == 0 {
                let rooms = || sim.tower.units.iter();
                out_for_meal |= rooms().any(|u| u.out_for_meal.unwrap_or(0) > 0);
                hotel_split |= rooms().any(|u| u.hotel_customers_in.unwrap_or(0) > 0);
                any |= rooms().any(|u| u.customers_in.unwrap_or(0) > 0);
                reconcile(&sim);
            }
        }
    }
    assert!(any, "a venue counted someone");
    assert!(out_for_meal, "a tenant was out for a meal");
    assert!(hotel_split, "a hotel guest was counted at a venue");
}

#[test]
fn a_bulldozed_hotel_room_leaves_its_guest_to_drain_in_balance() {
    let mut sim = mixed_tower();
    set_clock(&mut sim, 0, 18);
    let hall = unit_id(&sim, Kind::PartyHall);
    let room = unit_id(&sim, Kind::HotelSingle);
    let mingling = |sim: &Simulation| {
        live(sim, |p| {
            p.origin_unit_id == Some(room)
                && p.venue_unit_id == Some(hall)
                && p.state == PState::Dwelling
        }) > 0
    };
    for _ in 0..480 {
        if mingling(&sim) {
            break;
        }
        sim.tick(1.0);
    }
    assert!(mingling(&sim), "a hotel guest is counted at the hall");
    reconcile(&sim);
    assert!(sim.remove_facility(room, RemovalMethod::Bulldoze).ok);
    reconcile(&sim);
    assert!(
        live(&sim, |p| p.origin_unit_id == Some(room)) > 0,
        "the guest survives the bulldoze"
    );
    set_clock(&mut sim, 1, 2);
    assert!(customers(&sim, hall) > 0);
    for _ in 0..240 {
        if customers(&sim, hall) == 0 {
            break;
        }
        sim.tick(1.0);
    }
    assert_eq!(customers(&sim, hall), 0);
    reconcile(&sim);
}

#[test]
fn a_venue_bulldozed_under_its_audience_takes_its_tally_with_it() {
    let (mut sim, hall) = hall_with_audience();
    let (floor, x) = {
        let u = sim.tower.get_unit(hall).unwrap();
        (u.floor, u.x)
    };
    assert!(sim.remove_facility(hall, RemovalMethod::Bulldoze).ok);
    assert!(sim.tower.get_unit(hall).is_none());
    reconcile(&sim);
    assert!(sim.build(Kind::FastFood, floor, x).ok);
    let fresh = sim.tower.unit_at(floor, x).unwrap().id;
    assert_ne!(fresh, hall);
    // Cut the way home too: the riders' seats are released through finish().
    let shaft = sim.tower.transports[0].id;
    assert!(sim.remove_facility(shaft, RemovalMethod::Bulldoze).ok);
    assert!(sim.tower.transports.is_empty());
    set_clock(&mut sim, 1, 2);
    assert!(
        live(&sim, |p| p.venue_unit_id == Some(hall)) > 0,
        "ghost visitors remain"
    );
    for _ in 0..220 {
        if live(&sim, |p| p.venue_unit_id == Some(hall)) == 0 {
            break;
        }
        sim.tick(1.0);
        let u = sim.tower.get_unit(fresh).unwrap();
        assert_eq!(
            (u.customers_in, u.hotel_customers_in, u.out_for_meal),
            (None, None, None),
            "the rebuilt venue takes no ghost decrement"
        );
    }
    assert_eq!(live(&sim, |p| p.venue_unit_id == Some(hall)), 0);
    reconcile(&sim);
}

#[test]
fn a_fire_guts_the_venue_and_leaves_the_tally_to_drain() {
    let (mut sim, hall) = hall_with_audience();
    let state = |sim: &Simulation| sim.tower.get_unit(hall).unwrap().state;
    assert!(sim.start_fire_at(hall));
    assert_eq!(state(&sim), UnitState::Fire);
    assert_eq!(
        sim.events.pending.as_ref().map(|p| p.kind),
        Some("fireRescue")
    );
    for _ in 0..30 {
        sim.tick(1.0);
    }
    assert_eq!(state(&sim), UnitState::Fire);
    reconcile(&sim);
    assert!(customers(&sim, hall) > 0);
    sim.resolve_choice(true);
    for _ in 0..60 {
        if state(&sim) != UnitState::Fire {
            break;
        }
        sim.tick(1.0);
    }
    assert_eq!(state(&sim), UnitState::Gutted);
    assert_eq!(sim.tower.get_unit(hall).unwrap().occupants, 0);
    reconcile(&sim);
    // A gutted shell refuses a second outbreak.
    assert!(!sim.start_fire_at(hall));
    set_clock(&mut sim, 1, 2);
    for _ in 0..240 {
        if customers(&sim, hall) == 0 {
            break;
        }
        sim.tick(1.0);
        assert_eq!(
            sim.tower.get_unit(hall).unwrap().occupants,
            0,
            "the shell seats nobody"
        );
    }
    assert_eq!(customers(&sim, hall), 0);
    reconcile(&sim);
}
