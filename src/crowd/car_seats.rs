//! The seats in the cars: which cars are full where full cars pass hall
//! calls, and the seat a rider leaves when they get off or give up.
use super::Crowd;
use crate::tower::Tower;

/// Notes that car `key` (shaft, car) is full where full cars pass hall calls
/// (`full_cars`), until a seat is left (`release_seat`). Rare, so cold.
#[cold]
#[inline(never)]
pub(super) fn mark_full(crowd: &mut Crowd, tower: &Tower, key: (i64, i64)) {
    if !tower.mode.full_cars_pass_hall_calls() {
        return;
    }
    if let Err(at) = crowd.full_cars.binary_search(&key) {
        crowd.full_cars.insert(at, key);
    }
}

pub(super) fn release_seat(crowd: &mut Crowd, i: usize) {
    let p = &mut crowd.people[i];
    let (Some(ci), Some(sid)) = (p.car_index, p.shaft_id) else {
        return;
    };
    let key = (sid, ci);
    let n = crowd.car_riders.get(&key).copied().unwrap_or(1);
    crowd.car_riders.insert(key, (n - 1).max(0));
    if !crowd.full_cars.is_empty() {
        if let Ok(at) = crowd.full_cars.binary_search(&key) {
            crowd.full_cars.remove(at);
        }
    }
    crowd.people[i].car_index = None;
}
