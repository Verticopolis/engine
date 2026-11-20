//! What a fire leaves behind: a contained blaze costs no repair fee and
//! leaves a gutted shell that
//! never re-lets, re-ignites or earns (a shop included, and after a reload
//! too), a gutted room is not operational, and the emergency tallies the
//! shell reads count an outbreak once, a room gutted by fire, and a bomb
//! only when it goes off (its losses are not fire losses).
use crate::facilities::Kind;
use crate::load_stream::load_save_text;
use crate::sim::Simulation;
use crate::tower::UnitState;

use super::simulation_economy::{built_tower, X0};

/// A served floor-2 tower whose offices (the only flammable rooms) are
/// rented, and their ids.
fn offices(seed: u32, n: i64) -> (Simulation, Vec<i64>) {
    let mut sim = built_tower(seed);
    let mut ids = Vec::new();
    for i in 0..n {
        assert!(sim.build(Kind::Office, 2, X0 + i * 9).ok);
        let id = sim.tower.unit_at(2, X0 + i * 9).unwrap().id;
        let u = sim.tower.get_unit_mut(id).unwrap();
        u.state = UnitState::Occupied;
        u.ever_occupied = true;
        ids.push(id);
    }
    (sim, ids)
}

/// Security and Medical on the floor, so a fire is contained within days.
fn defend(sim: &mut Simulation) {
    sim.star = 4;
    for i in 40..80 {
        sim.tower.place(Kind::Lobby, 1, X0 + i);
        sim.tower.place(Kind::Floor, 2, X0 + i);
    }
    let at = X0 + 20;
    assert!(sim.tower.place(Kind::Security, 2, at).ok);
    assert!(
        sim.tower
            .place(Kind::Medical, 2, at + Kind::Security.facility().width)
            .ok
    );
}

fn burn_down(sim: &mut Simulation, id: i64) {
    defend(sim);
    assert!(sim.start_fire_at(id));
    for _ in 0..20 {
        if sim.events.active.is_empty() {
            break;
        }
        sim.tick(24.0 * 60.0);
    }
    assert!(sim.events.active.is_empty(), "the fire was contained");
    assert_eq!(sim.tower.get_unit(id).unwrap().state, UnitState::Gutted);
}

fn state(sim: &Simulation, id: i64) -> UnitState {
    sim.tower.get_unit(id).unwrap().state
}

#[test]
fn a_gutted_room_is_not_operational_and_never_catches_again() {
    let (mut sim, ids) = offices(11, 1);
    burn_down(&mut sim, ids[0]);
    assert!(!sim.tower.get_unit(ids[0]).unwrap().is_operational());
    for _ in 0..5 {
        sim.start_fire();
    }
    assert_eq!(state(&sim, ids[0]), UnitState::Gutted);
    assert!(!sim.start_fire_at(ids[0]));
}

#[test]
fn only_construction_fire_and_gutted_stop_a_room_operating() {
    let (mut sim, ids) = offices(11, 1);
    for (s, operating) in [
        (UnitState::Occupied, true),
        (UnitState::Empty, true),
        (UnitState::Asleep, true),
        (UnitState::Construction, false),
        (UnitState::Fire, false),
        (UnitState::Gutted, false),
    ] {
        sim.tower.get_unit_mut(ids[0]).unwrap().state = s;
        assert_eq!(
            sim.tower.get_unit(ids[0]).unwrap().is_operational(),
            operating,
            "{s:?}"
        );
    }
}

#[test]
fn a_gutted_room_stays_a_shell_through_a_reload_and_twenty_days() {
    let (mut sim, ids) = offices(11, 1);
    burn_down(&mut sim, ids[0]);
    let mut loaded = load_save_text(sim.save_text().as_bytes()).unwrap();
    assert_eq!(state(&loaded, ids[0]), UnitState::Gutted);
    assert!(loaded.events.active.is_empty());
    for _ in 0..20 {
        loaded.tick(24.0 * 60.0);
    }
    assert_eq!(state(&loaded, ids[0]), UnitState::Gutted);
    assert_eq!(loaded.tower.get_unit(ids[0]).unwrap().occupants, 0);
}

#[test]
fn a_gutted_shop_is_never_revived_to_earn() {
    let mut sim = built_tower(11);
    assert!(sim.tower.place(Kind::Shop, 2, X0).ok);
    let shop = sim.tower.unit_at(2, X0).unwrap().id;
    sim.tower.get_unit_mut(shop).unwrap().state = UnitState::Occupied;
    burn_down(&mut sim, shop);
    for _ in 0..5 {
        sim.tick(24.0 * 60.0);
    }
    assert_eq!(state(&sim, shop), UnitState::Gutted);
    assert_eq!(sim.tower.get_unit(shop).unwrap().occupants, 0);
}

#[test]
fn the_tallies_start_at_zero_and_count_one_outbreak_and_one_gutted_room() {
    let (mut sim, ids) = offices(7, 1);
    let tallies = |s: &Simulation| {
        (
            s.events.fires_started,
            s.events.fires_gut_rooms,
            s.events.bombs_detonated,
        )
    };
    assert_eq!(tallies(&sim), (0, 0, 0));
    burn_down(&mut sim, ids[0]);
    assert_eq!(tallies(&sim), (1, 1, 0));
}

#[test]
fn a_bomb_counts_only_when_it_goes_off_and_its_losses_are_not_fire_losses() {
    let (mut exposed, _) = offices(5, 2);
    exposed.bomb_threat();
    assert_eq!(exposed.events.bombs_detonated, 1);
    assert_eq!(exposed.events.fires_gut_rooms, 0);

    let (mut guarded, _) = offices(5, 2);
    assert!(guarded.tower.place(Kind::Security, 2, X0 + 20).ok);
    guarded.bomb_threat();
    assert_eq!(guarded.events.bombs_detonated, 0);
}

#[test]
fn containment_charges_no_repair_fee() {
    let (mut sim, ids) = offices(11, 1);
    defend(&mut sim);
    assert!(sim.start_fire_at(ids[0]));
    let before = sim.money;
    sim.tick(24.0 * 60.0);
    assert!(sim.events.active.is_empty(), "contained within the day");
    // Only the day's upkeep: the room is destroyed rather than repaired.
    assert!(
        before - sim.money < 10_000.0,
        "spent {}",
        before - sim.money
    );
}
