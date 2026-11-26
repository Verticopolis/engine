//! An office on notice for elevator waits: its notice holds until the waits
//! are fixed.
//!
//! A room recovers `SERVED_RECOVERY` in every hour its floor is not
//! crowded, so an office given notice in the morning rush used to climb back
//! to the rescind bar overnight, with the elevators as bad as before, and the
//! notice was cancelled every evening. Bad elevators never cost a tenant.
//! While an office is on notice for congestion, it now recovers only in the
//! hours its workers ride (weekdays, from the first arrivals to the last
//! departures), the hours that show whether the waits were fixed. A real fix
//! still reaches the rescind bar within one working day. On a weekend nobody
//! rides, so the notice clock stands still and a Friday notice is judged on
//! Monday.

use crate::crowd::workplace::{ARRIVE_FROM, LEAVE_BY};
use crate::facilities::Kind;
use crate::tower::{Unit, UnitState};

/// The vacate reason an elevator-wait notice carries.
const WAIT_REASON: &str = "congestion";

/// Minutes between two satisfaction passes: the notice clock stands still
/// this long for each weekend pass.
const PASS_MINUTES: f64 = 60.0;

/// Whether `u` is an office on notice for elevator waits.
pub(super) fn on_wait_notice(u: &Unit) -> bool {
    u.kind == Kind::Office
        && u.state == UnitState::Vacating
        && u.extras().vacate_reason == Some(WAIT_REASON)
}

/// Whether the pass at `hour` judges an hour an office's workers rode: a
/// pass judges the hour that just ended, so the weekday passes from the hour
/// after the first arrivals through the hour the last departures end.
pub(super) fn commute_hour(weekend: bool, hour: i64) -> bool {
    !weekend && (ARRIVE_FROM + 1..=LEAVE_BY).contains(&hour)
}

/// The satisfaction an office on wait notice keeps after a pass that took
/// it from `current` to `next`: outside its workers' hours it may fall but
/// never rise.
pub(super) fn held_satisfaction(current: f64, next: f64, weekend: bool, hour: i64) -> f64 {
    if commute_hour(weekend, hour) {
        next
    } else {
        next.min(current)
    }
}

/// The notice deadline after a pass at `vacate_at`: a weekend pass moves it
/// one pass later, so the weekend does not count against the tenant.
pub(super) fn held_deadline(vacate_at: f64, weekend: bool) -> f64 {
    if weekend {
        vacate_at + PASS_MINUTES
    } else {
        vacate_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_office_recovers_only_while_its_workers_ride() {
        assert_eq!(held_satisfaction(0.1, 0.15, false, 12), 0.15);
        assert_eq!(held_satisfaction(0.1, 0.15, false, ARRIVE_FROM), 0.1);
        assert_eq!(held_satisfaction(0.1, 0.15, false, ARRIVE_FROM + 1), 0.15);
        assert_eq!(held_satisfaction(0.1, 0.15, false, LEAVE_BY), 0.15);
        assert_eq!(held_satisfaction(0.1, 0.15, false, LEAVE_BY + 1), 0.1);
        assert_eq!(held_satisfaction(0.1, 0.15, false, 2), 0.1);
        assert_eq!(held_satisfaction(0.1, 0.15, true, 12), 0.1);
    }

    #[test]
    fn a_fall_still_counts_outside_the_workers_hours() {
        assert_eq!(held_satisfaction(0.3, 0.2, false, 23), 0.2);
        assert_eq!(held_satisfaction(0.3, 0.2, true, 12), 0.2);
    }

    #[test]
    fn the_notice_clock_stands_still_on_a_weekend() {
        assert_eq!(held_deadline(1_000.0, true), 1_060.0);
        assert_eq!(held_deadline(1_000.0, false), 1_000.0);
    }
}
