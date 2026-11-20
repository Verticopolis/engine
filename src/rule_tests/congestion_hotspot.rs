//! The per-floor congestion reading the ratings, the Map's Traffic view and
//! the morning complaint read: a local jam reads past the churn line at the
//! peak while the average hides it, the hotspot names the jammed floor and
//! is the true per-floor max, a lobby-served bank's bottleneck is named at
//! the lobby with its one weak shaft as the cause, a well-built tower stays
//! low, and a tower with nothing congested names no floor.
use crate::clock::GameMode;
use crate::facilities::{Kind, LOT_WIDTH};
use crate::sim::Simulation;
use crate::tower::UnitState;

use super::seeded_game;

const W: i64 = LOT_WIDTH;
const C: i64 = W / 2;
/// Where a floor's tenants start to leave (`CONGESTION_CHURN`).
const CHURN: f64 = 1.0;
const HEALTHY_PEAK: f64 = 0.4;

fn tower(seed: u32, top: i64) -> Simulation {
    let mut sim = seeded_game(seed, GameMode::Classic);
    sim.money = 1e12;
    let xs: Vec<i64> = (C..W).chain((0..C).rev()).collect();
    for floor in 1..=top {
        let kind = if floor == 1 { Kind::Lobby } else { Kind::Floor };
        for &x in &xs {
            sim.tower.place(kind, floor, x);
        }
    }
    sim
}

fn lift(sim: &mut Simulation, x: i64, bottom: i64, top: i64, cars: i64) -> i64 {
    let r = sim
        .tower
        .place_transport(Kind::ElevatorStandard, x, bottom, top);
    assert!(r.ok, "{:?}", r.reason);
    let id = r.unit_id.unwrap();
    assert!(sim.tower.set_cars(id, cars));
    id
}

fn fill(sim: &mut Simulation, floor: i64, count: usize) {
    let mut placed = 0;
    let mut x = 0;
    while x + 9 <= W && placed < count {
        let r = sim.tower.place(Kind::Office, floor, x);
        if r.ok {
            sim.tower.get_unit_mut(r.unit_id.unwrap()).unwrap().state = UnitState::Occupied;
            placed += 1;
        }
        x += 9;
    }
}

fn floor_reading(sim: &Simulation, floor: i64) -> f64 {
    sim.spatial_congestion_by_floor()
        .get(&floor)
        .copied()
        .unwrap_or(0.0)
}

/// Eight light floors on a strong shaft, and three packed floors on one
/// weak shaft above a transfer at floor 10.
fn jammed_zone() -> Simulation {
    let mut sim = tower(1, 20);
    lift(&mut sim, W - 6, 1, 10, 8);
    lift(&mut sim, W - 12, 10, 20, 1);
    for floor in 2..=9 {
        fill(&mut sim, floor, 12);
    }
    for floor in [11, 12, 13] {
        fill(&mut sim, floor, 30);
    }
    sim
}

#[test]
fn a_local_jam_reads_past_churn_at_the_peak_where_the_average_hides_it() {
    let sim = jammed_zone();
    let (peak, _) = sim.peak_congestion();
    assert!(peak > sim.congestion());
    assert!(peak > CHURN, "{peak}");
}

#[test]
fn the_hotspot_is_the_jammed_floor_and_the_true_per_floor_max() {
    let sim = jammed_zone();
    let (ratio, floor) = sim.peak_congestion();
    let floor = floor.expect("a floor is named");
    assert!([11, 12, 13].contains(&floor), "{floor}");
    assert_eq!(floor_reading(&sim, floor), ratio);
    for f in 2..=20 {
        assert!(floor_reading(&sim, f) <= ratio, "floor {f}");
    }
}

#[test]
fn a_lobby_served_bottleneck_is_named_at_the_lobby_with_its_shaft_as_the_cause() {
    let mut sim = tower(7, 10);
    let weak = lift(&mut sim, W - 6, 1, 10, 1);
    for floor in 2..=10 {
        fill(&mut sim, floor, 30);
    }
    let (ratio, floor) = sim.peak_congestion();
    assert_eq!(floor, Some(1), "the lobby, not 2F");
    assert_eq!(floor_reading(&sim, 1), ratio);
    assert!(ratio > CHURN, "{ratio}");
    assert_eq!(sim.congestion_cause(1), vec![weak]);
}

#[test]
fn a_well_built_tower_peaks_low() {
    let mut sim = tower(2, 12);
    for i in 0..4 {
        let r = sim
            .tower
            .place_transport(Kind::ElevatorStandard, W - 6 - i * 6, 1, 12);
        if r.ok {
            sim.tower.set_cars(r.unit_id.unwrap(), 8);
        }
    }
    for floor in 2..=12 {
        fill(&mut sim, floor, 12);
    }
    let (peak, _) = sim.peak_congestion();
    assert!(peak < HEALTHY_PEAK, "{peak}");
}

#[test]
fn nothing_congested_names_no_floor() {
    let sim = tower(3, 1);
    assert_eq!(sim.peak_congestion(), (0.0, None));
    let spot = sim.peak_congestion_hotspot();
    assert!(spot["floor"].is_null(), "never floor 0, a real floor");
}
