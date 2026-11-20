//! A shop on a floor that is served but past Classic's stair-climb budget
//! draws no visitors: its retail line never takes a positive cent (only its
//! own overhead), it earns strictly less than the same shop made reachable,
//! and building an elevator that shortcuts the climb turns its trade on.
//! Modern sets no climb budget, so the same stairs reach it.
use crate::clock::GameMode;
use crate::facilities::{Kind, LOT_WIDTH};
use crate::ledger::LEDGER_CATS;
use crate::sim::Simulation;
use crate::tower::UnitState;

const C: i64 = LOT_WIDTH / 2;
const X0: i64 = C - 15;
const X1: i64 = C + 45;
const DAY: f64 = 24.0 * 60.0;
const SHOP_FLOOR: i64 = 40;

/// An elevator to floor 30, then a stair chain to 45, two rented offices on
/// floor 2 for the shop's demand pool, and a rented shop on floor 40, ten
/// flights above the elevator.
fn stranded_shop(seed: u32, mode: GameMode) -> Simulation {
    let mut sim = Simulation::new_game(seed, mode);
    sim.money = 1e12;
    for x in X0..=X1 {
        sim.tower.place(Kind::Lobby, 1, x);
        for floor in 2..=45 {
            sim.tower.place(Kind::Floor, floor, x);
        }
    }
    assert!(
        sim.tower
            .place_transport(Kind::ElevatorStandard, C, 1, 30)
            .ok
    );
    for floor in 30..45 {
        assert!(
            sim.tower
                .place_transport(Kind::Stairs, C + 20, floor, floor + 1)
                .ok
        );
    }
    for x in [X0, C + 8] {
        let r = sim.tower.place(Kind::Office, 2, x);
        assert!(r.ok, "{:?}", r.reason);
        sim.tower.get_unit_mut(r.unit_id.unwrap()).unwrap().state = UnitState::Occupied;
    }
    let r = sim.tower.place(Kind::Shop, SHOP_FLOOR, C + 30);
    assert!(r.ok, "{:?}", r.reason);
    sim.tower.get_unit_mut(r.unit_id.unwrap()).unwrap().state = UnitState::Occupied;
    sim
}

fn shortcut(sim: &mut Simulation) {
    assert!(
        sim.build_transport(Kind::ElevatorStandard, C - 10, 30, 45)
            .ok
    );
}

fn retail_average(sim: &Simulation) -> f64 {
    let at = LEDGER_CATS.iter().position(|c| *c == "retail").unwrap();
    sim.ledger.average_per_day()[at]
}

fn days(sim: &mut Simulation, n: usize) {
    for _ in 0..n {
        sim.tick(DAY);
    }
}

#[test]
fn only_classic_strands_the_shop_up_the_stairs() {
    for (mode, reachable) in [(GameMode::Classic, false), (GameMode::Modern, true)] {
        let mut sim = stranded_shop(21, mode);
        assert!(sim.tower.is_floor_served(SHOP_FLOOR), "{mode:?}");
        assert_eq!(sim.floor_reachable(SHOP_FLOOR), reachable, "{mode:?}");
    }
}

#[test]
fn a_stranded_shop_takes_no_positive_retail_until_a_shortcut_reaches_it() {
    let mut sim = stranded_shop(21, GameMode::Classic);
    days(&mut sim, 4);
    let stranded = retail_average(&sim);
    assert!(stranded <= 0.0, "a stranded shop took {stranded}");
    shortcut(&mut sim);
    assert!(sim.floor_reachable(SHOP_FLOOR));
    days(&mut sim, 4);
    assert!(retail_average(&sim) > stranded);
}

#[test]
fn a_stranded_shop_earns_less_than_the_same_shop_made_reachable() {
    let mut stranded = stranded_shop(22, GameMode::Classic);
    let mut reachable = stranded_shop(22, GameMode::Classic);
    shortcut(&mut reachable);
    assert!(!stranded.floor_reachable(SHOP_FLOOR));
    assert!(reachable.floor_reachable(SHOP_FLOOR));
    days(&mut stranded, 5);
    days(&mut reachable, 5);
    assert!(retail_average(&reachable) > retail_average(&stranded));
}
