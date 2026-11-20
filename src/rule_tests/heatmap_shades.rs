//! The overlay shades the panels test at their middles, at their ends: a
//! fully leased office floor reads green and a fully vacant one red, a
//! tinted floor's cell spans its built columns, and the cleanliness overlay
//! shades a hotel room by its crew's reach and its state (clean green,
//! dirty amber, infested hotter still, unreached red; every room red with
//! no crew), with no cell for an office, a crew station or a condo still
//! being built.
use crate::clock::GameMode;
use crate::facilities::{Kind, LOT_WIDTH};
use crate::panels::heatmap::{HeatCell, HeatmapMode};
use crate::sim::Simulation;
use crate::tower::UnitState;

use super::seeded_game;

const C: i64 = LOT_WIDTH / 2;

fn tower(seed: u32, top: i64) -> Simulation {
    let mut sim = seeded_game(seed, GameMode::Classic);
    sim.money = 1e12;
    // Out from the middle, so every tile has a neighbor to join.
    let xs: Vec<i64> = (C..LOT_WIDTH).chain((0..C).rev()).collect();
    for floor in 1..=top {
        let kind = if floor == 1 { Kind::Lobby } else { Kind::Floor };
        for &x in &xs {
            sim.tower.place(kind, floor, x);
        }
    }
    sim
}

fn room(sim: &mut Simulation, kind: Kind, floor: i64, x: i64, state: UnitState) -> i64 {
    let r = sim.tower.place(kind, floor, x);
    assert!(r.ok, "{kind:?} at {floor}/{x}: {:?}", r.reason);
    let id = r.unit_id.unwrap();
    sim.tower.get_unit_mut(id).unwrap().state = state;
    id
}

fn cell_on(cells: &[HeatCell], floor: i64) -> &HeatCell {
    cells
        .iter()
        .find(|c| c.floor == floor)
        .unwrap_or_else(|| panic!("a cell on floor {floor}"))
}

#[test]
fn a_fully_leased_office_floor_reads_green_and_a_fully_vacant_one_red() {
    let mut sim = tower(51, 3);
    assert!(sim.build_transport(Kind::ElevatorStandard, C, 1, 3).ok);
    for x in [0, 9] {
        room(&mut sim, Kind::Office, 2, x, UnitState::Occupied);
        room(&mut sim, Kind::Office, 3, x, UnitState::Empty);
    }
    let cells = sim.floor_heatmap(HeatmapMode::Occupancy);
    assert_eq!(cell_on(&cells, 2).severity, 0.0);
    assert_eq!(cell_on(&cells, 3).severity, 1.0);
}

#[test]
fn a_tinted_floors_cell_spans_its_built_columns() {
    let mut sim = tower(52, 2);
    room(&mut sim, Kind::Office, 2, 40, UnitState::Occupied);
    let cells = sim.floor_heatmap(HeatmapMode::Occupancy);
    let cell = cell_on(&cells, 2);
    assert!(cell.min_x <= 40 && cell.max_x >= 48, "{cell:?}");
}

#[test]
fn the_cleanliness_overlay_shades_each_hotel_room_by_reach_and_state() {
    let mut sim = tower(57, 6);
    assert!(sim.build_transport(Kind::ElevatorStandard, C, 1, 6).ok);
    // Staff ride floors 2 to 5 only, so floor 6 has no staff route.
    assert!(
        sim.tower
            .place_transport(Kind::ElevatorService, C + 8, 2, 5)
            .ok
    );
    room(&mut sim, Kind::Housekeeping, 2, C - 8, UnitState::Occupied);
    room(&mut sim, Kind::HotelDouble, 3, C - 20, UnitState::Asleep);
    room(&mut sim, Kind::HotelDouble, 4, C - 20, UnitState::Dirty);
    room(&mut sim, Kind::HotelDouble, 5, C - 20, UnitState::Infested);
    room(&mut sim, Kind::HotelDouble, 6, C - 20, UnitState::Asleep);
    room(&mut sim, Kind::Office, 2, C - 20, UnitState::Occupied);
    let condo = room(&mut sim, Kind::Condo, 6, C - 40, UnitState::Empty);
    let building = room(&mut sim, Kind::Condo, 6, C + 12, UnitState::Construction);

    let cells = sim.floor_heatmap(HeatmapMode::Cleanliness);
    assert_eq!(cell_on(&cells, 3).severity, 0.0, "reached and clean");
    assert!((cell_on(&cells, 4).severity - 0.6).abs() < 1e-6, "dirty");
    let infested = cell_on(&cells, 5);
    assert!((infested.severity - 0.85).abs() < 1e-6);
    assert_eq!(infested.tint, Some("infested"));
    let unreached = cells
        .iter()
        .find(|c| c.floor == 6 && c.tint.is_none())
        .expect("the unreached room");
    assert_eq!(unreached.severity, 1.0);
    let at = |id: i64| sim.tower.get_unit(id).unwrap().x;
    let condo_cell = cells
        .iter()
        .find(|c| c.floor == 6 && c.min_x == at(condo))
        .expect("the condo's cell");
    assert_eq!(condo_cell.tint, Some("na"));
    assert!(!cells
        .iter()
        .any(|c| c.floor == 6 && c.min_x == at(building)));
    assert!(!cells.iter().any(|c| c.floor == 2), "office and crew");
}

#[test]
fn with_no_crew_every_hotel_room_reads_unreached() {
    let mut sim = tower(58, 2);
    assert!(sim.build_transport(Kind::ElevatorStandard, C, 1, 2).ok);
    room(&mut sim, Kind::HotelDouble, 2, C - 20, UnitState::Asleep);
    let cells = sim.floor_heatmap(HeatmapMode::Cleanliness);
    assert_eq!(cell_on(&cells, 2).severity, 1.0);
}
