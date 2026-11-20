//! The inspector's service diagnostics on a small tower: a stall a ramp
//! reaches reads connected and one past a gap reads disconnected with no
//! counts, a ramp on fire cuts its stall off, the parking shortage follows
//! each mode's rule a space at a time, an inert housekeeping station or
//! ramp reads its sections as not applicable, a dirty or infested room's
//! hotel verdicts come back the same after a save, and a recycling center's
//! capacity, fill and verdict hold at the capacity edge.
use serde_json::Value;

use crate::clock::GameMode;
use crate::demand::RECYCLING_POP_PER_CENTER;
use crate::facilities::Kind;
use crate::load::deserialize;
use crate::services::{GARBAGE_COLLECT_HOUR, PARKING_WORKERS_PER_SPACE};
use crate::sim::Simulation;
use crate::tower::UnitState;

use super::put;

/// The people an occupied office holds.
const OFFICE_POPULATION: i64 = 6;

struct Rooms {
    sim: Simulation,
    hotel: i64,
    housekeeping: i64,
    ramp: i64,
    connected: i64,
    disconnected: i64,
}

/// A hotel room, its housekeeping station, an office, and a parking ramp
/// with one stall beside it and one past a gap no ramp reaches, all in use.
fn rooms(mode: GameMode) -> Rooms {
    let mut sim = Simulation::new_game(7, mode);
    sim.money = 1e9;
    sim.star = 3;
    for x in 150..275 {
        for (kind, floor) in [(Kind::Lobby, 1), (Kind::Floor, 2), (Kind::Floor, 0)] {
            sim.tower.place(kind, floor, x);
        }
    }
    let office = put(&mut sim, Kind::Office, 2, 160);
    let hotel = put(&mut sim, Kind::HotelSingle, 2, 185);
    let housekeeping = put(&mut sim, Kind::Housekeeping, 2, 215);
    let ramp = put(&mut sim, Kind::ParkingRamp, 0, 225);
    let connected = put(&mut sim, Kind::Parking, 0, 241);
    let disconnected = put(&mut sim, Kind::Parking, 0, 260);
    for id in [office, hotel, housekeeping, ramp, connected, disconnected] {
        sim.tower.get_unit_mut(id).unwrap().state = UnitState::Occupied;
    }
    Rooms {
        sim,
        hotel,
        housekeeping,
        ramp,
        connected,
        disconnected,
    }
}

fn facts(sim: &mut Simulation, id: i64) -> Value {
    serde_json::to_value(sim.service_diagnostic_facts(id).unwrap()).unwrap()
}

/// The tower as a save would bring it back.
fn reload(sim: &Simulation) -> Simulation {
    deserialize(&sim.serialize()).unwrap()
}

#[test]
fn a_stall_past_a_gap_reads_disconnected_and_a_burning_ramp_cuts_its_stall_off() {
    let mut r = rooms(GameMode::Modern);
    let mut sim = reload(&r.sim);
    assert_eq!(
        facts(&mut sim, r.connected)["parking"]["status"],
        "connected"
    );
    let off = &facts(&mut sim, r.disconnected)["parking"];
    assert_eq!(off["status"], "disconnected");
    assert!(off["have"].is_null() && off["shortage"].is_null());
    assert_eq!(facts(&mut sim, r.ramp)["parking"]["status"], "ramp");
    r.sim.tower.get_unit_mut(r.ramp).unwrap().state = UnitState::Fire;
    let mut sim = reload(&r.sim);
    assert_eq!(
        facts(&mut sim, r.connected)["parking"]["status"],
        "disconnected"
    );
}

#[test]
fn the_parking_shortage_follows_each_modes_rule_a_space_at_a_time() {
    let per_space = PARKING_WORKERS_PER_SPACE as i64;
    assert_eq!(
        per_space % OFFICE_POPULATION,
        0,
        "offices fill a space exactly"
    );
    // The tower's own office plus enough for one space's worth of workers,
    // then one office more. Modern counts the cars, a space per 24 workers;
    // Classic asks for one free stall for every office (#1970) and counts
    // one (#2024), so it never runs short here.
    let at_one_space = per_space / OFFICE_POPULATION - 1;
    let width = Kind::Office.facility().width;
    for mode in [GameMode::Classic, GameMode::Modern] {
        for (added, over) in [(at_one_space, false), (at_one_space + 1, true)] {
            let mut r = rooms(mode);
            for x in 160..160 + added * width {
                assert!(r.sim.tower.place(Kind::Floor, 3, x).ok);
            }
            for i in 0..added {
                let id = put(&mut r.sim, Kind::Office, 3, 160 + i * width);
                r.sim.tower.get_unit_mut(id).unwrap().state = UnitState::Occupied;
            }
            let mut sim = reload(&r.sim);
            assert_eq!(facts(&mut sim, r.ramp)["parking"]["status"], "ramp");
            let short = over && mode == GameMode::Modern;
            let need = if short { 2.0 } else { 1.0 };
            let p = &facts(&mut sim, r.connected)["parking"];
            assert_eq!(p["status"], "connected", "{mode:?} +{added}");
            assert_eq!(p["shortage"], short, "{mode:?} +{added}");
            assert_eq!(p["have"], 1);
            assert_eq!(p["total"], need, "{mode:?} +{added}");
            assert_eq!(p["offices"], need, "{mode:?} +{added}");
            assert_eq!(p["suites"], 0.0);
        }
    }
}

#[test]
fn an_inert_housekeeping_station_or_ramp_reads_its_sections_as_not_applicable() {
    for mode in [GameMode::Classic, GameMode::Modern] {
        for state in [UnitState::Construction, UnitState::Fire, UnitState::Gutted] {
            let mut r = rooms(mode);
            for id in [r.housekeeping, r.ramp] {
                r.sim.tower.get_unit_mut(id).unwrap().state = state;
            }
            let mut sim = reload(&r.sim);
            for id in [r.housekeeping, r.ramp] {
                let f = facts(&mut sim, id);
                assert_eq!(f["housekeeping"]["status"], "notApplicable", "{state:?}");
                assert_eq!(f["parking"]["status"], "notApplicable", "{state:?}");
            }
        }
    }
}

#[test]
fn a_dirty_or_infested_rooms_verdicts_come_back_after_a_save() {
    for mode in [GameMode::Classic, GameMode::Modern] {
        for (state, name) in [
            (UnitState::Dirty, "dirty"),
            (UnitState::Infested, "infested"),
        ] {
            let mut r = rooms(mode);
            // Four stars, where a hotel's guests stop counting toward the
            // rating, and the room in that state.
            r.sim.star = 4;
            r.sim.tower.get_unit_mut(r.hotel).unwrap().state = state;
            let before = facts(&mut r.sim, r.hotel)["hotel"].clone();
            let mut sim = reload(&r.sim);
            let hotel = facts(&mut sim, r.hotel)["hotel"].clone();
            assert_eq!(hotel, before, "{mode:?} {name}");
            assert_eq!(hotel["rating"], "excluded");
            assert_eq!(hotel["condition"], name);
            if state == UnitState::Dirty {
                assert_eq!(hotel["infestDays"], 3);
            } else {
                assert_eq!(hotel["recovery"], mode == GameMode::Modern);
            }
        }
    }
}

/// One recycling center under `offices` occupied offices filled floor by
/// floor from floor 2, with `singles` occupied single rooms after them, read
/// a minute before the collection hour, when the bins are fullest.
fn recycling_tower(mode: GameMode, offices: i64, singles: i64) -> Simulation {
    let mut sim = Simulation::new_game(13, mode);
    sim.money = 1e12;
    sim.star = 5;
    for x in 100..280 {
        assert!(sim.tower.place(Kind::Lobby, 1, x).ok);
        for floor in [0, -1].into_iter().chain(2..=25) {
            assert!(sim.tower.place(Kind::Floor, floor, x).ok);
        }
    }
    assert!(
        sim.tower
            .place_transport(Kind::ElevatorStandard, 102, -1, 25)
            .ok
    );
    put(&mut sim, Kind::Recycling, -1, 150);
    let width = Kind::Office.facility().width;
    let per_floor = (280 - 110) / width;
    let at = |i: i64| (2 + i / per_floor, 110 + (i % per_floor) * width);
    for i in 0..offices {
        let (floor, x) = at(i);
        let id = put(&mut sim, Kind::Office, floor, x);
        sim.tower.get_unit_mut(id).unwrap().state = UnitState::Occupied;
    }
    let (floor, x) = at(offices);
    let single = Kind::HotelSingle.facility().width;
    for i in 0..singles {
        let id = put(&mut sim, Kind::HotelSingle, floor, x + i * single);
        sim.tower.get_unit_mut(id).unwrap().state = UnitState::Occupied;
    }
    let read = GARBAGE_COLLECT_HOUR * 60.0 - 1.0;
    sim.clock.minutes += (read - sim.clock.minute_of_day() + 1440.0) % 1440.0;
    assert_eq!(sim.clock.minute_of_day(), read);
    reload(&sim)
}

#[test]
fn a_recycling_center_holds_its_verdict_at_the_capacity_edge() {
    let per_floor = (280 - 110) / Kind::Office.facility().width;
    // (offices, singles, population, met, the fill clamped at 1)
    let cases = [
        (24 * per_floor, 0, 2_592, false, true),
        (416, 5, 2_501, false, false),
        (416, 4, 2_500, true, false),
        (per_floor, 0, 108, true, false),
        (0, 0, 0, true, false),
    ];
    for mode in [GameMode::Classic, GameMode::Modern] {
        for (offices, singles, people, met, clamped) in cases {
            let sim = recycling_tower(mode, offices, singles);
            let label = format!("{mode:?}, {people} people");
            assert_eq!(sim.population(), people, "{label}");
            assert_eq!(
                sim.recycling_capacity(),
                RECYCLING_POP_PER_CENTER,
                "{label}"
            );
            assert_eq!(sim.recycling_demand_met(), met, "{label}");
            let fill = sim.recycling_fill();
            assert_eq!(fill > 0.0, people > 0, "{label}: fill {fill}");
            assert_eq!(fill == 1.0, clamped, "{label}: fill {fill}");
        }
    }
}
