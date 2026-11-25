//! Routing on a split floor whose runs both reach the lobby (#647, #662),
//! from the retired `segmentRoutingReachable.integration.test.ts`: a maid
//! sent to a room on the far run rides round to it and never stands in the
//! gap, a stranded venue on a half-reachable floor draws no demand share and
//! no patrons, shaft balancing never moves a rider onto a shaft their run
//! cannot reach, and a shaft straddling a gap serves both runs and sets its
//! riders down on the one they asked for. The meal and return legs on the
//! same kind of floor are the lunch-rush tests at the end.
use crate::clock::GameMode;
use crate::crowd::motion::advance;
use crate::crowd::{PState, CROWD_SECONDS_PER_MINUTE};
use crate::demand::compute_demand_map;
use crate::facilities::Kind;
use crate::housekeeping_shift::MaidSent;
use crate::sim::Simulation;
use crate::tower::UnitState;

use super::{at_time, place_seated, shaft, strip_tower};

/// A Modern lobby `width` wide and floor 2 laid over `runs` only.
fn split_floor(seed: u32, width: i64, runs: &[std::ops::Range<i64>]) -> Simulation {
    split_floor_in(GameMode::Modern, seed, width, runs)
}

/// [`split_floor`] in a `mode` tower.
fn split_floor_in(
    mode: GameMode,
    seed: u32,
    width: i64,
    runs: &[std::ops::Range<i64>],
) -> Simulation {
    let mut sim = strip_tower(seed, mode, width, 1);
    for run in runs {
        for x in run.clone() {
            assert!(sim.tower.place(Kind::Floor, 2, x).ok, "floor 2 at {x}");
        }
    }
    sim
}

fn center(sim: &Simulation, id: i64) -> f64 {
    let u = sim.tower.get_unit(id).unwrap();
    u.x as f64 + u.width as f64 / 2.0
}

/// Sends a maid seated at `crew` to clean `room` (one game-minute of work) and
/// steps the crowd until the job reports. Returns whether it completed and
/// every column a maid stood on upstairs.
fn send_maid(sim: &mut Simulation, crew: i64, room: i64, steps: usize) -> (bool, Vec<i64>) {
    sim.seat_crews();
    let mut free = sim.free_maids();
    let sent = sim.assign_maid(&mut free, crew, (room, 2, center(sim, room)));
    assert_eq!(sent, MaidSent::Sent, "a route to the room's run exists");
    // One game-minute of work keeps the walk the test.
    for p in sim
        .crowd
        .people
        .iter_mut()
        .filter(|p| p.clean_unit_id == Some(room))
    {
        p.linger_for = Some(CROWD_SECONDS_PER_MINUTE);
    }
    let mut columns = Vec::new();
    for _ in 0..steps {
        advance(&mut sim.crowd, 3.0, &mut sim.tower);
        for p in sim.crowd.people.iter().filter(|p| p.staff && p.floor >= 2) {
            columns.push(p.x.round() as i64);
        }
        if let Some(done) = sim
            .crowd
            .take_staff_results()
            .iter()
            .find(|r| r.unit_id == room)
        {
            return (done.ok, columns);
        }
    }
    panic!("the cleaning job never reported");
}

#[test]
fn a_maid_rides_round_to_a_room_on_the_far_run() {
    const GAP: std::ops::RangeInclusive<i64> = 25..=34;
    let mut sim = split_floor(77, 60, &[0..25, 35..60]);
    shaft(&mut sim, Kind::Stairs, 0, 1, 2);
    shaft(&mut sim, Kind::Stairs, 52, 1, 2);
    let crew = place_seated(&mut sim, Kind::Housekeeping, 2, 10, UnitState::Empty, 0);
    let room = place_seated(&mut sim, Kind::HotelSingle, 2, 40, UnitState::Empty, 0);
    let (ok, columns) = send_maid(&mut sim, crew, room, 400);
    assert!(ok, "the maid reached the room");
    assert!(!columns.iter().any(|c| GAP.contains(c)), "never in the gap");
    assert!(
        columns.iter().any(|&c| c >= 35),
        "she stood on the room's run"
    );
}

/// Floor 2's left run (0 to 29) has the only elevator; its right run (40 to
/// 59) has no transport. An office and a shop on the left, a shop on the
/// right. Returns the sim, the reachable shop and the stranded one.
fn stranded_venue() -> (Simulation, i64, i64) {
    stranded_venue_in(GameMode::Modern)
}

/// [`stranded_venue`] in a `mode` tower.
fn stranded_venue_in(mode: GameMode) -> (Simulation, i64, i64) {
    let mut sim = split_floor_in(mode, 9, 60, &[0..30, 40..60]);
    shaft(&mut sim, Kind::ElevatorStandard, 0, 1, 2);
    let staff = Kind::Office.facility().population;
    place_seated(&mut sim, Kind::Office, 2, 5, UnitState::Occupied, staff);
    let near = place_seated(&mut sim, Kind::Shop, 2, 15, UnitState::Occupied, 0);
    let far = place_seated(&mut sim, Kind::Shop, 2, 44, UnitState::Occupied, 0);
    (sim, near, far)
}

#[test]
fn a_stranded_run_reads_unreachable_beside_a_reachable_one() {
    let (mut sim, near, far) = stranded_venue();
    let (near_x, far_x) = (
        sim.tower.get_unit(near).unwrap().x,
        sim.tower.get_unit(far).unwrap().x,
    );
    assert!(sim.tower.is_floor_served(2));
    assert!(sim.position_reachable(2, near_x));
    assert!(!sim.position_reachable(2, far_x));
}

#[test]
fn only_the_reachable_venue_shares_the_demand_pool() {
    let (mut sim, near, far) = stranded_venue();
    let map = compute_demand_map(&mut sim);
    assert_eq!(map.retail_venue_count, 2);
    assert!(map.fraction_by_unit.get(&near).is_some_and(|&f| f > 0.0));
    assert!(!map.fraction_by_unit.contains_key(&far));
}

/// Shoppers come in person (`crowd::walk_ins`): the traffic pass books
/// walk-ins for the open, reachable shop and none for the stranded one,
/// which nobody can reach, and both draw only who is inside.
#[test]
fn traffic_sends_shoppers_to_the_reachable_venue_and_none_to_the_stranded_one() {
    // Shoppers walk in from the street in Classic only
    // (`GameMode::shoppers_walk_in`).
    let (mut sim, near, far) = stranded_venue_in(GameMode::Classic);
    for hour in 10..18 {
        at_time(&mut sim, 0, hour);
        sim.collect_traffic_income();
    }
    let booked = |id: i64| {
        sim.crowd
            .walk_ins
            .iter()
            .filter(|w| w.unit_id == id)
            .count()
    };
    assert!(
        booked(near) > 0,
        "the reachable shop's shoppers are on their way"
    );
    assert_eq!(booked(far), 0, "nobody is sent to the stranded shop");
    assert_eq!(
        sim.tower.get_unit(near).unwrap().occupants,
        0,
        "nobody inside yet"
    );
    assert_eq!(sim.tower.get_unit(far).unwrap().occupants, 0);
}

/// A rider's shafts from the lobby to floor 2 at column `to_x`, asked 60
/// times so shaft balancing has its chances to re-pick.
fn shafts_to(sim: &mut Simulation, to_x: f64) -> Vec<Vec<i64>> {
    (0..60)
        .map(|_| {
            let r = sim.crowd.route(&sim.tower, 1, 2, Some(25.0), Some(to_x));
            r.expect("the run is reachable").shafts
        })
        .collect()
}

#[test]
fn balancing_never_moves_a_rider_to_the_other_wings_shaft() {
    let mut sim = split_floor(1, 50, &[0..20, 30..50]);
    let left = shaft(&mut sim, Kind::ElevatorStandard, 2, 1, 2);
    let right = shaft(&mut sim, Kind::ElevatorStandard, 42, 1, 2);
    assert!(shafts_to(&mut sim, 4.0).iter().all(|s| *s == vec![left]));
    assert!(shafts_to(&mut sim, 44.0).iter().all(|s| *s == vec![right]));
}

#[test]
fn an_elevator_straddling_a_gap_serves_both_runs() {
    let mut sim = split_floor(1, 60, &[0..11, 13..26]);
    let ev = shaft(&mut sim, Kind::ElevatorStandard, 10, 1, 2);
    assert!(!sim.tower.has_structure(2, 11) && !sim.tower.has_structure(2, 12));
    assert!(sim.position_reachable(2, 2), "run A");
    assert!(sim.position_reachable(2, 18), "run B, past the gap");
    for to_x in [2.0, 18.0] {
        let r = sim
            .crowd
            .route(&sim.tower, 1, 2, Some(25.0), Some(to_x))
            .unwrap();
        assert_eq!(r.shafts, vec![ev], "to column {to_x}");
    }
}

/// Floor 2 split at column 15 under an 8-wide stair at x 10 (center 14, on
/// run A), lobby to floor 2.
fn straddled_stair() -> (Simulation, i64) {
    let mut sim = split_floor(31, 41, &[0..15, 16..31]);
    let stairs = shaft(&mut sim, Kind::Stairs, 10, 1, 2);
    assert!(!sim.tower.has_structure(2, 15));
    (sim, stairs)
}

#[test]
fn a_rider_steps_off_a_straddling_stair_onto_the_run_they_asked_for() {
    let (mut sim, _) = straddled_stair();
    let crew = place_seated(&mut sim, Kind::Housekeeping, 2, 0, UnitState::Empty, 0);
    let room = place_seated(&mut sim, Kind::HotelSingle, 2, 20, UnitState::Empty, 0);
    let (ok, columns) = send_maid(&mut sim, crew, room, 600);
    assert!(columns.iter().any(|&c| c >= 16), "she reached run B");
    assert!(ok, "the cleaning job completed");
}

#[test]
fn a_straddling_shaft_never_banks_with_a_one_run_shaft() {
    let (mut sim, stairs) = straddled_stair();
    shaft(&mut sim, Kind::ElevatorStandard, 2, 1, 2);
    assert!(shafts_to(&mut sim, 20.0).iter().all(|s| *s == vec![stairs]));
}

/// A Modern strip lobby 60 wide with floor 2 laid over `runs`, a standard
/// elevator at each of `shafts`, and an occupied condo and fast food on
/// floor 2 at `condo_x` and `venue_x`.
fn meal_tower(
    runs: &[std::ops::Range<i64>],
    shafts: [i64; 2],
    condo_x: i64,
    venue_x: i64,
) -> (Simulation, i64, i64) {
    let mut sim = split_floor(4242, 60, runs);
    sim.money = 5_000_000.0;
    for x in shafts {
        assert!(
            sim.tower
                .place_transport(Kind::ElevatorStandard, x, 1, 2)
                .ok
        );
    }
    let household = Kind::Condo.facility().population;
    let condo = place_seated(
        &mut sim,
        Kind::Condo,
        2,
        condo_x,
        UnitState::Occupied,
        household,
    );
    let venue = place_seated(&mut sim, Kind::FastFood, 2, venue_x, UnitState::Occupied, 0);
    (sim, condo, venue)
}

/// A lunch rush held at noon: the clock is put back to 12:00 and the condo
/// and venue kept open before every one-minute tick, so the condo keeps
/// sending diners. `visit` sees every person after each tick; the answer is
/// whether anyone ever stood upstairs in `gap`.
fn meal_rush(
    sim: &mut Simulation,
    (condo, venue): (i64, i64),
    ticks: usize,
    gap: std::ops::RangeInclusive<i64>,
    mut visit: impl FnMut(&crate::crowd::Person),
) -> bool {
    let household = Kind::Condo.facility().population;
    let mut in_gap = false;
    for _ in 0..ticks {
        let c = sim.tower.get_unit_mut(condo).unwrap();
        (c.state, c.occupants) = (UnitState::Occupied, household);
        sim.tower.get_unit_mut(venue).unwrap().state = UnitState::Occupied;
        sim.clock = crate::clock::Clock::new(12.0 * 60.0, sim.clock.calendar);
        sim.tick(1.0);
        for p in &sim.crowd.people {
            in_gap |= p.floor >= 2 && gap.contains(&(p.x.round() as i64));
            visit(p);
        }
    }
    in_gap
}

/// Moved here from the game's meal-rush check, like the two below: diners
/// bound for a venue on the far run ride down and back up to it, and none
/// strolls the near run toward it.
#[test]
fn a_meal_trip_lands_on_the_venues_own_run() {
    let (mut sim, condo, venue) = meal_tower(&[0..25, 35..60], [0, 56], 6, 38);
    let (mut seated, mut wrong_run) = (false, false);
    let in_gap = meal_rush(&mut sim, (condo, venue), 300, 25..=34, |p| {
        if p.meal_venue_id != Some(venue) || p.floor != 2 {
            return;
        }
        let x = p.x.round() as i64;
        seated |= p.state == PState::Dwelling && x >= 35;
        wrong_run |= !p.returning && p.state == PState::ToDest && x <= 24;
    });
    assert!(!in_gap, "nobody stands in the gap");
    assert!(seated, "the far-run venue seated a diner on its own run");
    assert!(!wrong_run, "no diner strolls the near run toward it");
}

#[test]
fn a_diner_stays_on_the_run_it_set_out_from() {
    let (mut sim, condo, venue) = meal_tower(&[0..40, 50..60], [38, 52], 2, 20);
    let (mut any, mut crossed) = (false, false);
    meal_rush(&mut sim, (condo, venue), 300, 40..=49, |p| {
        if p.meal_venue_id != Some(venue) {
            return;
        }
        any = true;
        crossed |= p.floor == 2 && p.x.round() as i64 >= 50;
    });
    assert!(any, "the venue drew diners");
    assert!(!crossed, "no near-run diner appears on the far run");
}

#[test]
fn a_return_leg_lands_on_the_tenants_own_run() {
    let (mut sim, condo, venue) = meal_tower(&[0..25, 35..60], [0, 56], 38, 6);
    let (mut home, mut stranded) = (false, false);
    let in_gap = meal_rush(&mut sim, (condo, venue), 400, 25..=34, |p| {
        if !(p.returning && p.origin_unit_id == Some(condo)) || p.floor != 2 {
            return;
        }
        let x = p.x.round() as i64;
        home |= p.state == PState::ToDest && x >= 35;
        stranded |= p.state == PState::ToDest && x <= 24;
    });
    assert!(!in_gap, "nobody stands in the gap");
    assert!(home, "a far-run tenant came home to its own run");
    assert!(!stranded, "no returning tenant ends on the near run");
}
