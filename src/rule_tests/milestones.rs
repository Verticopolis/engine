//! The optional goals: a milestone fires once on the day it is met, with one
//! headline, and never again; a reload keeps it without announcing it again;
//! a load adopts every milestone already met without a headline; Smooth
//! Operator waits until every occupied floor is served; and No Vacancy
//! counts an empty condo as a vacancy. Moved here from the game's milestone
//! check.
use crate::clock::GameMode;
use crate::facilities::{Kind, LOT_WIDTH};
use crate::gameplay::RemovalMethod;
use crate::load::deserialize;
use crate::sim::Simulation;
use crate::tower::UnitState;

use super::lay_row;

const DAY: f64 = 60.0 * 24.0;

/// Offices across `floor` from x 0, leaving the right edge for a shaft,
/// stamped occupied when `occupy` is set.
fn offices(sim: &mut Simulation, floor: i64, occupy: bool) {
    let width = Kind::Office.facility().width;
    let mut x = 0;
    while x + width <= LOT_WIDTH - 6 {
        let r = sim.tower.place(Kind::Office, floor, x);
        if occupy && r.ok {
            sim.tower.get_unit_mut(r.unit_id.unwrap()).unwrap().state = UnitState::Occupied;
        }
        x += width;
    }
}

/// A lobby and office floors 2 to `top`, the first `occupied` of them
/// stamped occupied, served by one standard elevator on the right edge up to
/// `shaft_top` (none when it is 1).
fn office_tower(seed: u32, top: i64, occupied: i64, shaft_top: i64) -> Simulation {
    let mut sim = Simulation::new_game(seed, GameMode::Classic);
    sim.money = 1e9;
    lay_row(&mut sim, Kind::Lobby, 1);
    for floor in 2..=top {
        lay_row(&mut sim, Kind::Floor, floor);
        offices(&mut sim, floor, floor <= occupied + 1);
    }
    if shaft_top > 1 {
        let shaft = sim.build_transport(Kind::ElevatorStandard, LOT_WIDTH - 4, 1, shaft_top);
        assert!(shaft.ok, "{:?}", shaft.reason);
    }
    sim
}

fn has(sim: &Simulation, id: &str) -> bool {
    sim.milestones.iter().any(|m| m == id)
}

fn headlines(sim: &Simulation) -> usize {
    sim.log.iter().filter(|e| e.text.contains("🏅")).count()
}

fn reload(sim: &Simulation, edit: impl FnOnce(&mut serde_json::Value)) -> Simulation {
    let mut raw = sim.serialize();
    edit(&mut raw);
    deserialize(&raw).unwrap()
}

#[test]
fn a_milestone_fires_once_with_one_headline() {
    let mut sim = office_tower(1, 4, 2, 4);
    assert!(sim.population() < 500, "below the bar at the start");
    assert!(!has(&sim, "pop-500"));
    sim.tick(DAY);
    assert!((500..2500).contains(&sim.population()));
    assert!(has(&sim, "pop-500"));
    let log = |sim: &Simulation| {
        sim.log
            .iter()
            .filter(|e| e.text == "🏅 Milestone: Getting Started")
            .count()
    };
    assert_eq!(log(&sim), 1);
    let achieved = sim.milestones.len();
    sim.tick(DAY);
    assert_eq!(sim.milestones.len(), achieved);
    assert_eq!(log(&sim), 1, "announced once");
}

#[test]
fn a_reload_keeps_a_milestone_and_announces_nothing() {
    let mut sim = office_tower(1, 4, 3, 4);
    sim.tick(DAY);
    assert!(has(&sim, "pop-500"));
    let mut back = reload(&sim, |_| {});
    assert!(has(&back, "pop-500"));
    let (achieved, before) = (back.milestones.len(), headlines(&back));
    back.tick(DAY);
    assert_eq!(back.milestones.len(), achieved);
    assert_eq!(headlines(&back), before, "no headline again");
}

#[test]
fn a_load_adopts_a_met_milestone_without_a_headline() {
    let sim = office_tower(1, 4, 3, 4);
    assert!(sim.population() >= 500);
    let back = reload(&sim, |raw| {
        raw.as_object_mut().unwrap().remove("milestones");
    });
    assert!(has(&back, "pop-500"), "adopted at the load");
    assert_eq!(headlines(&back), 0, "without a headline");
}

#[test]
fn smooth_operator_waits_for_every_occupied_floor_to_be_served() {
    let mut sim = office_tower(2, 25, 24, 24);
    assert!(sim.population() >= 5000);
    sim.adopt_milestones();
    assert!(!has(&sim, "well-served"), "floor 25 is not served");
    let shaft = sim.tower.transports[0].id;
    assert!(sim.resize_transport(shaft, 1, 25).ok);
    assert!(!has(&sim, "well-served"), "judged when the day turns");
    sim.tick(DAY);
    assert!(has(&sim, "well-served"));
}

#[test]
fn an_empty_condo_is_a_vacancy_for_no_vacancy() {
    let mut sim = office_tower(3, 12, 11, 1);
    lay_row(&mut sim, Kind::Floor, 13);
    let condo = sim.tower.place(Kind::Condo, 13, 0).unit_id.unwrap();
    assert!(sim.population() >= 2000);
    sim.adopt_milestones();
    assert!(!has(&sim, "full-house"), "the unsold condo is a vacancy");
    assert!(sim.remove_facility(condo, RemovalMethod::Bulldoze).ok);
    sim.adopt_milestones();
    assert!(has(&sim, "full-house"));
}
