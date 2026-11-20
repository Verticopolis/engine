//! The HUD's room counts follow a running simulation: after every step and
//! every command on three real saves, `tower_stats_record` agrees with a
//! walk over every unit in the tower's list, so the room index it reads
//! never falls behind a fire, a departure, a rate change, a batch reprice, a
//! sale or a rebuild.
use crate::facilities::Kind;
use crate::load::deserialize;
use crate::panels::stats::TOWER_STATS;
use crate::rent::BatchTarget;
use crate::sim::Simulation;
use crate::tower::{Unit, UnitState};

use super::fixture_save;

/// Thirty game hours in two-minute steps: an evening, a morning checkout
/// and the hourly passes.
const STEPS: usize = 900;

/// The counts the walk checks, by their names in [`TOWER_STATS`].
const COUNTED: [&str; 13] = [
    "population",
    "parkingSpaces",
    "offices",
    "occupiedOffices",
    "condos",
    "soldCondos",
    "hotelRooms",
    "occupiedHotel",
    "dirty",
    "shops",
    "restaurants",
    "vacant",
    "vacantNoRate",
];

/// The counts as a walk over every unit, structural tiles included, with
/// no index or kept list behind it.
fn walked(sim: &Simulation) -> [f64; COUNTED.len()] {
    let mut c = [0.0; COUNTED.len()];
    let mut add = |name: &str| c[COUNTED.iter().position(|n| *n == name).unwrap()] += 1.0;
    for u in &sim.tower.units {
        match u.kind {
            Kind::Office => {
                add("offices");
                if u.is_tenanted() {
                    add("occupiedOffices");
                }
                if u.state == UnitState::Empty {
                    add("vacant");
                    if u.no_rate {
                        add("vacantNoRate");
                    }
                }
            }
            Kind::Condo => {
                add("condos");
                if u.ever_occupied {
                    add("soldCondos");
                }
            }
            k if k.is_hotel() => {
                add("hotelRooms");
                if u.state == UnitState::Asleep {
                    add("occupiedHotel");
                }
                if u.state == UnitState::Dirty {
                    add("dirty");
                }
            }
            Kind::Shop => add("shops"),
            Kind::Restaurant | Kind::FastFood => add("restaurants"),
            Kind::Parking => add("parkingSpaces"),
            _ => {}
        }
    }
    c[0] = sim
        .tower
        .units
        .iter()
        .filter(|u| u.is_present())
        .map(Unit::census_count)
        .sum::<i64>() as f64;
    c
}

/// The same counts as the HUD reads them.
fn reported(sim: &Simulation) -> [f64; COUNTED.len()] {
    let record = sim.tower_stats_record();
    COUNTED.map(|name| record[TOWER_STATS.iter().position(|n| *n == name).unwrap()])
}

/// A seeded pick among the rooms `keep` accepts.
fn pick(sim: &Simulation, n: usize, keep: impl Fn(&Unit) -> bool) -> Option<(i64, Kind, i64, i64)> {
    let rooms: Vec<&Unit> = sim.tower.units.iter().filter(|u| keep(u)).collect();
    let u = rooms.get(n % rooms.len().max(1))?;
    Some((u.id, u.kind, u.floor, u.x))
}

/// Runs the save with the counts checked after every step and command, and
/// gives how often each count moved between checks, and how many sales
/// were rebuilt.
fn run(name: &str) -> ([usize; COUNTED.len()], usize) {
    let mut sim = deserialize(&fixture_save(name)).unwrap();
    sim.money = 1e12;
    let mut moved = [0; COUNTED.len()];
    let mut last = reported(&sim);
    let mut rebuilds = 0;
    let mut check = |sim: &Simulation, label: String| {
        let now = reported(sim);
        assert_eq!(now, walked(sim), "{name}, {label}");
        for (k, (a, b)) in now.iter().zip(last).enumerate() {
            if *a != b {
                moved[k] += 1;
            }
        }
        last = now;
    };
    for step in 1..=STEPS {
        sim.tick(2.0);
        check(&sim, format!("step {step}"));
        if step % 97 == 0 {
            sim.start_fire();
            check(&sim, format!("fire at {step}"));
        }
        if step % 61 == 0 {
            // A tenant leaves, and the office goes off the market or back on.
            let office = pick(&sim, step, |u| {
                u.kind == Kind::Office && u.state != UnitState::Construction
            });
            if let Some((id, ..)) = office {
                let i = sim.tower.units.iter().position(|u| u.id == id).unwrap();
                let no_rate = sim.tower.units[i].no_rate;
                sim.vacate(i, "access");
                check(&sim, format!("vacate at {step}"));
                if no_rate {
                    sim.price_unit(id, 10_000.0);
                } else {
                    sim.set_no_rate(id);
                }
                check(&sim, format!("rate at {step}"));
            }
            // The batch takes a kind off the market and, the next time round
            // for that kind, back on at the default rate.
            let k = step / 61;
            let kind = if k % 2 == 0 {
                Kind::Office
            } else {
                Kind::Condo
            };
            let target = if (k / 2) % 2 == 0 {
                BatchTarget::NoRate
            } else {
                BatchTarget::Default
            };
            sim.apply_rent_batch(kind, target, false);
            check(&sim, format!("batch at {step}"));
        }
        if step % 113 == 0 {
            let room = pick(&sim, step, |u| {
                matches!(u.kind, Kind::Office | Kind::Condo | Kind::Shop)
            });
            if let Some((_, kind, floor, x)) = room {
                let sold = sim.sell_at(floor, x);
                check(&sim, format!("sale at {step}"));
                if sold && sim.build(kind, floor, x).ok {
                    rebuilds += 1;
                }
                check(&sim, format!("build at {step}"));
            }
        }
    }
    (moved, rebuilds)
}

#[test]
fn the_hud_counts_match_a_walk_over_every_unit_after_every_step_and_command() {
    let mut totals = [0; COUNTED.len()];
    let mut rebuilds = 0;
    for name in ["sixseven_2", "split-tower-4", "towerone-star4"] {
        let (moved, built) = run(name);
        for (t, m) in totals.iter_mut().zip(moved) {
            *t += m;
        }
        rebuilds += built;
    }
    // What the three runs reached together: every condition count and the
    // population moved, and a sale and a build went through.
    for name in [
        "population",
        "occupiedOffices",
        "vacant",
        "vacantNoRate",
        "soldCondos",
        "occupiedHotel",
        "dirty",
    ] {
        let k = COUNTED.iter().position(|n| *n == name).unwrap();
        assert!(totals[k] > 0, "{name} moved");
    }
    assert!(rebuilds > 0, "a sale was rebuilt");
}
