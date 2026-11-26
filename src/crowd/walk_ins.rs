//! Customers who come in from outside the tower to a shop or restaurant (a
//! kind that [`Kind::takes_walk_ins`]), as many a day as the 1994 game
//! served ([`daily_walk_ins`]): about 25 to 30 for a shop and 35 to 50 for
//! a restaurant or fast food (party, 2026-11-25, Verticopolis/web#2631).
//! Classic towers only for now (`GameMode::shoppers_walk_in`).
//! Full demand in person jammed the elevators of older towers, so it was
//! switched off on 2026-11-14; the original's counts are the load the 1994
//! game ran under.
//!
//! Each hour the traffic pass works out how many customers each open venue
//! should see that hour, holds that to the hour's share of the day's walk-in
//! count, and books that many arrivals at random minutes of the hour
//! (`Crowd::book_walk_ins`). The venue's income and its patronage still
//! follow the demand; the walk-ins are the share of those customers a
//! player sees arrive. Every step starts the arrivals whose minute has
//! come (`start_walk_ins`): each one enters at the ground lobby, or at a
//! Metro platform the tower's riders can reach, and walks and rides to the
//! venue like anyone else. They split between the lobby and the stations by
//! what each entrance's transport can carry.
//!
//! The door keeps them out of the way of the tenants: a walk-in whose
//! first car already has a full load of riders waiting for it at the way
//! in stays away (each lobby of a split lot boards its own cars), and one
//! who waits [`WALK_IN_GIVE_UP`] at the door walks back out it
//! (`motion`). Neither ever vanishes on a car or a floor partway up.
//!
//! They are not part of the 140-person sample (`MAX_PEOPLE`); only
//! `MAX_WALK_INS` bounds them, a ceiling a working tower never reaches.
//! The bookings are saved with the crowd (`save.rs`), so a reload keeps
//! the rest of the hour's arrivals. A save without them starts with none,
//! and the next hour books again.
//!
//! None of this runs while [`crate::facilities::WALK_INS_ON`] is off.

use super::fasthash::FastMap;
use super::spawn::{inside_x, make_person, plan, venue_has_room};
use super::{Crowd, CROWD_SECONDS_PER_MINUTE};
use crate::clock::Clock;
use crate::economy::visitors::daily_visitor_cap;
use crate::facilities::Kind;
use crate::tower::Tower;

/// The most walk-ins in the tower at once, on their way or inside. A
/// ceiling for a tower whose transport has seized up, far above what a
/// working tower holds: the crowd benchmark moves 5,000 people for about
/// 3 ms a step in the browser.
pub const MAX_WALK_INS: i64 = 4_000;

/// The crowd seconds a walk-in waits for a car at the door before walking
/// back out of it: fifteen game minutes, a visitor's patience, where a
/// tenant waits an hour (`GIVE_UP`).
pub const WALK_IN_GIVE_UP: f64 = 15.0 * CROWD_SECONDS_PER_MINUTE;

/// How many customers come in person to a `kind` on a day: the 1994 game's
/// daily visitors for a shop, fast food or restaurant
/// ([`daily_visitor_cap`]). `None` for a kind that takes none.
pub fn daily_walk_ins(kind: Kind, weekend: bool) -> Option<f64> {
    daily_visitor_cap(kind, weekend).map(|n| n as f64)
}

/// An arrival booked for `unit_id` at clock minute `at`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WalkIn {
    pub at: f64,
    pub unit_id: i64,
}

impl Crowd {
    /// Books `expected` customers (a mean, its fraction rounded up by a
    /// draw) for `unit_id` at random minutes from `from` to `until`.
    pub fn book_walk_ins(&mut self, unit_id: i64, expected: f64, from: f64, until: f64) {
        if !expected.is_finite() || expected <= 0.0 || until <= from {
            return;
        }
        let whole = expected.floor();
        let extra = self.rng.chance(expected - whole);
        let count = whole as i64 + i64::from(extra);
        for _ in 0..count {
            let at = from + self.rng.next() * (until - from);
            self.walk_ins.push(WalkIn { at, unit_id });
        }
    }

    /// Orders the bookings latest first, so the next one due is last and
    /// each start pops it off the end. Run once after an hour's bookings.
    pub fn sort_walk_ins(&mut self) {
        self.walk_ins.sort_by(|a, b| {
            b.at.total_cmp(&a.at)
                .then_with(|| b.unit_id.cmp(&a.unit_id))
        });
    }
}

/// Starts every booked walk-in whose minute has come. A booking more than
/// an hour late (a bomb search held everyone still) is dropped, so a long
/// freeze does not end in a flood.
pub fn start_walk_ins(crowd: &mut Crowd, tower: &Tower, clock: &Clock) {
    let now = clock.minutes;
    if crowd.walk_ins.last().is_none_or(|w| w.at > now) {
        return;
    }
    let (hour, minute) = (clock.hour(), clock.minute_of_day());
    let doors = entrances(tower);
    crowd.queue_view(tower);
    let mut lines = DoorLines::default();
    while let Some(w) = crowd.walk_ins.last().copied() {
        if w.at > now {
            break;
        }
        crowd.walk_ins.pop();
        if now - w.at > 60.0 || crowd.walk_in_count >= MAX_WALK_INS {
            continue;
        }
        start_walk_in(crowd, tower, &doors, &mut lines, w.unit_id, (hour, minute));
    }
}

/// The walk-ins started this step, by the elevator they head for and the
/// floor they board it on. The riders already waiting or on their way come
/// from the step's queue view (`Crowd::queue_view`), so the door walks no
/// crowd of its own.
#[derive(Default)]
struct DoorLines {
    started: FastMap<(i64, i64), f64>,
}

/// The riders waiting for elevator `shaft` on `floor` at the start of the
/// step, with the walk-ins still walking to it from the door.
fn door_line(crowd: &Crowd, shaft: i64, floor: i64) -> f64 {
    let Some((_, view)) = crowd.queue_cache.as_ref() else {
        return 0.0;
    };
    let waiting = view
        .landings
        .get(&shaft)
        .and_then(|floors| floors.get(&floor))
        .map_or(0, |l| l.count);
    let heading = view
        .walk_ins_heading
        .get(&(shaft, floor))
        .copied()
        .unwrap_or(0);
    (waiting + heading) as f64
}

fn start_walk_in(
    crowd: &mut Crowd,
    tower: &Tower,
    doors: &Entrances,
    lines: &mut DoorLines,
    unit_id: i64,
    (hour, minute): (i64, f64),
) {
    let Some(venue) = tower.get_unit(unit_id) else {
        return;
    };
    if !venue.kind.takes_walk_ins(tower.mode)
        || !venue.is_tenanted()
        || !venue.open_for_business(hour, tower.mode)
        || !venue_has_room(venue, tower.mode)
    {
        return;
    }
    let (vx, vw, vf) = (venue.x, venue.width, venue.floor);
    let station = doors.pick(crowd);
    // Nobody steps onto a platform no train stands at: that walk-in stays
    // away (`metro_train`).
    if station.is_some_and(|sid| !super::metro_train::train_in_at(tower, minute, sid)) {
        return;
    }
    // From the station's own platform, or the ground lobby nearest the
    // venue (`plan` picks it), so a lot of separate towers is entered under
    // the venue.
    let (from, from_x) = match station.and_then(|sid| tower.get_unit(sid)) {
        Some(st) => (st.floor + 1, Some(st.x + st.width / 2)),
        None => (1, None),
    };
    let Some((route, route_from_x, route_to_x)) = plan(crowd, tower, from, vf, from_x, Some(vx))
    else {
        return;
    };
    // The tenants come first: a shopper whose first elevator already has
    // a full load waiting for it here (one load for each of its cars), or
    // on the way from the door, stays away, so a walk-in never joins a line
    // the elevator cannot clear at once. A route that starts on foot or by
    // stairs or escalator forms no line.
    let first = route.shafts.first().and_then(|&s| tower.get_transport(s));
    if let Some(car) = first.filter(|t| t.kind.is_elevator()) {
        let started = *lines.started.get_or_insert_with((car.id, from), || 0.0);
        if door_line(crowd, car.id, from) + started >= load_of(car) {
            return;
        }
        *lines.started.get_or_insert_with((car.id, from), || 0.0) += 1.0;
    }
    let i = make_person(crowd, tower, route, route_to_x as f64, Some(route_from_x));
    crowd.people[i].dest_x = crowd.rng.int(vx, vx + vw - 1) as f64;
    crowd.people[i].meal_venue_id = Some(unit_id);
    crowd.people[i].walk_in = true;
    crowd.walk_in_count += 1;
    if let Some(sid) = station {
        let st = tower.get_unit(sid).unwrap();
        crowd.people[i].x = inside_x(crowd, st, 2) as f64;
        crowd.people[i].start_x = crowd.people[i].x;
        crowd.people[i].from_metro = true;
    }
}

/// The ways in from the street and how many people an hour each can take
/// up into the tower: the ground lobby, and each Metro station whose
/// platform the riders' transport serves. Walk-ins split between them by
/// that capacity, so a station with one small elevator gets a small share.
pub(super) struct Entrances {
    lobby: f64,
    stations: Vec<Station>,
    total: f64,
}

/// A Metro station walk-ins can come in by: its id and the riders its
/// transport takes in one load.
struct Station {
    id: i64,
    load: f64,
}

impl Entrances {
    /// The station a walk-in comes in by, or `None` for the ground lobby.
    pub(super) fn pick(&self, crowd: &mut Crowd) -> Option<i64> {
        if self.stations.is_empty() || self.total <= 0.0 {
            return None;
        }
        let mut r = crowd.rng.next() * self.total - self.lobby;
        if r < 0.0 {
            return None;
        }
        for s in &self.stations {
            if r < s.load {
                return Some(s.id);
            }
            r -= s.load;
        }
        self.stations.last().map(|s| s.id)
    }
}

pub(super) fn entrances(tower: &Tower) -> Entrances {
    let lobby = boarding_capacity(tower, 1);
    let stations: Vec<Station> = tower
        .metro_stations()
        .filter(|u| tower.is_metro_platform_served(u))
        .map(|u| Station {
            id: u.id,
            load: boarding_capacity(tower, u.floor + 1),
        })
        .filter(|s| s.load > 0.0)
        .collect();
    let total = lobby + stations.iter().map(|s| s.load).sum::<f64>();
    Entrances {
        lobby,
        stations,
        total,
    }
}

/// The riders the passenger transport at `floor` holds in one load: every
/// car of each elevator that stops there, plus each flight of stairs and
/// escalator. Staff-only transport takes no customers.
fn boarding_capacity(tower: &Tower, floor: i64) -> f64 {
    tower
        .transports
        .iter()
        .filter(|t| !t.kind.facility().staff_only && t.stops_at(floor))
        .map(load_of)
        .sum()
}

/// The riders transport `t` takes in one load: each of its cars once.
fn load_of(t: &crate::tower::Transport) -> f64 {
    let cars = if t.kind.is_elevator() { t.cars } else { 1 };
    cars as f64 * t.kind.car_capacity()
}
