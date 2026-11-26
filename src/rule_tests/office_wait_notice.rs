//! Elevator waits cost tenants: an office on notice for congestion recovers
//! only in its workers' hours, so a quiet night no longer cancels the notice
//! while the elevators stay as bad, and a real fix still saves the tenant.
use crate::clock::GameMode;
use crate::facilities::Kind;
use crate::sim::Simulation;
use crate::tower::UnitState;

use super::{lay_row, MID};

const TOP: i64 = 12;
const LEFT: i64 = MID - 45;
const OFFICE_WIDTH: i64 = 9;
const OFFICES: usize = 9 * (TOP as usize - 1);

/// A full lobby and floors 2 to 12, one standard elevator, and nine offices
/// on each floor, rented on two cars over four days.
fn rented_office_tower() -> (Simulation, i64) {
    let mut sim = Simulation::new_game(1102, GameMode::Classic);
    sim.money = 1e12;
    lay_row(&mut sim, Kind::Lobby, 1);
    for floor in 2..=TOP {
        lay_row(&mut sim, Kind::Floor, floor);
    }
    assert!(
        sim.build_transport(Kind::ElevatorStandard, LEFT + 3, 1, TOP)
            .ok
    );
    let shaft = sim.tower.transports[0].id;
    assert_eq!(
        sim.tower.transports[0].cars, 2,
        "a new elevator comes with two cars"
    );
    for floor in 2..=TOP {
        for n in 0..9 {
            let x = LEFT + 9 + n * OFFICE_WIDTH;
            assert!(
                sim.tower.place(Kind::Office, floor, x).ok,
                "office at {floor}/{x}"
            );
        }
    }
    for _ in 0..4 * 24 {
        sim.tick(60.0);
    }
    assert_eq!(tenanted(&sim), OFFICES, "every office rents on two cars");
    (sim, shaft)
}

fn tenanted(sim: &Simulation) -> usize {
    sim.tower
        .units
        .iter()
        .filter(|u| u.kind == Kind::Office)
        .filter(|u| matches!(u.state, UnitState::Occupied | UnitState::Vacating))
        .count()
}

fn on_notice(sim: &Simulation) -> usize {
    let vacating = |u: &&crate::tower::Unit| u.state == UnitState::Vacating;
    sim.tower
        .units
        .iter()
        .filter(|u| u.kind == Kind::Office)
        .filter(vacating)
        .count()
}

#[test]
fn offices_leave_a_tower_whose_one_car_never_keeps_up() {
    let (mut sim, shaft) = rented_office_tower();
    assert!(sim.tower.set_cars(shaft, 1));
    let mut left = 0;
    for _ in 0..6 * 24 {
        let before: Vec<i64> = vacating_ids(&sim);
        sim.tick(60.0);
        left += before
            .iter()
            .filter(|&&id| sim.tower.get_unit(id).unwrap().state == UnitState::Empty)
            .count();
    }
    assert!(left > 0, "a quiet night cancelled every notice");
}

fn vacating_ids(sim: &Simulation) -> Vec<i64> {
    let vacating =
        |u: &&crate::tower::Unit| u.kind == Kind::Office && u.state == UnitState::Vacating;
    sim.tower
        .units
        .iter()
        .filter(vacating)
        .map(|u| u.id)
        .collect()
}

#[test]
fn more_cars_after_the_notices_keep_the_tenants() {
    let (mut sim, shaft) = rented_office_tower();
    assert!(sim.tower.set_cars(shaft, 1));
    let mut hours = 0;
    while on_notice(&sim) == 0 {
        sim.tick(60.0);
        hours += 1;
        assert!(hours < 3 * 24, "nobody gave notice on one car");
    }
    assert!(sim.tower.set_cars(shaft, 8));
    for _ in 0..6 * 24 {
        sim.tick(60.0);
    }
    assert_eq!(
        tenanted(&sim),
        OFFICES,
        "the fix saved every tenant on notice"
    );
    assert_eq!(on_notice(&sim), 0, "every notice was rescinded");
}
