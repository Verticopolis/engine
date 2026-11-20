//! The hourly elevator telemetry the WASM host adopts (the demand curves the
//! schedule dialog reads, the utilization average and the boarding-origin
//! rings), yesterday's housekeeping result and the per-floor commute-stress
//! averages, on every conformance fixture hour by hour through two days and
//! a weekend. The save carries none of them, so neither the conformance
//! view nor the frame-day golden holds them. Every hour's document has the
//! shape the host adopts and names only shafts in the tower, an hour inside
//! a weekday leaves the weekend rings alone (and the other way round), the
//! commute stress comes by floor, and the runs fill every store with real
//! values.
use serde_json::Value;

use crate::sim::Simulation;
use crate::vctower_import::import_vctower;

use super::log_text::FIXTURES;

const DAY: f64 = 1440.0;
const HOUR: f64 = 60.0;

#[derive(Default, Debug, PartialEq)]
struct Seen {
    weekday: bool,
    weekend: bool,
    housekeeping: bool,
    commute: bool,
}

fn is_num(v: &Value) -> bool {
    v.is_number()
}

fn is_pairs(v: &Value) -> bool {
    v.as_array().is_some_and(|ps| {
        ps.iter().all(|p| {
            p.as_array()
                .is_some_and(|p| p.len() == 2 && p.iter().all(is_num))
        })
    })
}

fn is_days(v: &Value, slot: fn(&Value) -> bool) -> bool {
    ["weekday", "weekend"]
        .iter()
        .all(|k| v[*k].as_array().is_some_and(|r| r.iter().all(slot)))
}

fn is_entries(v: &Value, value: &dyn Fn(&Value) -> bool) -> bool {
    v.as_array().is_some_and(|es| {
        es.iter().all(|e| {
            e.as_array()
                .is_some_and(|e| e.len() == 2 && is_num(&e[0]) && value(&e[1]))
        })
    })
}

/// The document `seedElevatorTelemetry` takes, all the way down.
fn telemetry(sim: &Simulation) -> Value {
    let doc = sim.telemetry_json();
    assert!(
        is_entries(&doc["util"], &is_num)
            && is_entries(&doc["hourly"], &|x| is_days(x, is_num))
            && is_entries(&doc["origins"], &|x| is_days(x, is_pairs)),
        "the telemetry is the document the host adopts: {doc}"
    );
    doc
}

/// Every shaft's recorded load on `ring`: each demand hour and each origin
/// slot that holds anything, keyed by store and shaft.
fn ring_of(doc: &Value, ring: &str) -> Vec<(String, String)> {
    let mut load = Vec::new();
    for (store, empty) in [
        ("hourly", Value::from(0)),
        ("origins", Value::Array(vec![])),
    ] {
        for e in doc[store].as_array().unwrap() {
            let held: Vec<(usize, &Value)> = e[1][ring]
                .as_array()
                .unwrap()
                .iter()
                .enumerate()
                .filter(|(_, v)| **v != empty && v.as_f64() != Some(0.0))
                .collect();
            if !held.is_empty() {
                load.push((format!("{store} of shaft {}", e[0]), format!("{held:?}")));
            }
        }
    }
    load.sort();
    load
}

fn warmed(doc: &Value, ring: &str) -> bool {
    doc["hourly"].as_array().unwrap().iter().any(|e| {
        e[1][ring]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v.as_f64().is_some_and(|v| v > 0.0))
    })
}

fn day_of(minutes: f64) -> i64 {
    (minutes / DAY).floor() as i64
}

fn run_hours(sim: &mut Simulation, hours: usize, seen: &mut Seen, label: &str) {
    let week = sim.clock.calendar;
    let weekend = |day: i64| day % week.week_days >= week.week_days - week.weekend_days;
    for h in 0..hours {
        let before = telemetry(sim);
        let start_day = day_of(sim.clock.minutes);
        sim.tick(HOUR);
        let doc = telemetry(sim);
        let at = format!("{label} hour {h}");
        let shafts: Vec<i64> = sim.tower.transports.iter().map(|t| t.id).collect();
        for store in ["util", "hourly", "origins"] {
            for e in doc[store].as_array().unwrap() {
                let id = e[0].as_i64().unwrap();
                assert!(shafts.contains(&id), "{at}: shaft {id} is in the tower");
            }
        }
        let minutes = sim.clock.minutes;
        if day_of(minutes) == start_day && minutes % DAY != 0.0 {
            let other = if weekend(start_day) {
                "weekday"
            } else {
                "weekend"
            };
            assert_eq!(
                ring_of(&doc, other),
                ring_of(&before, other),
                "{at}: the hour leaves the {other} rings alone"
            );
        }
        let commute = sim.crowd.commute_stress_json();
        let floors: Vec<f64> = commute
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p[0].as_f64().unwrap())
            .collect();
        assert!(floors.windows(2).all(|w| w[0] <= w[1]), "{at}: by floor");
        seen.commute |= !floors.is_empty();
        seen.weekday |= warmed(&doc, "weekday");
        seen.weekend |= warmed(&doc, "weekend");
        seen.housekeeping |= sim
            .housekeeping
            .yesterday
            .is_some_and(|(cleaned, leftover)| cleaned > 0 || leftover > 0);
    }
}

#[test]
fn every_fixture_hour_by_hour_through_a_weekday_and_a_weekend() {
    let mut seen = Seen::default();
    for (name, text) in FIXTURES {
        let mut sim = import_vctower(text).unwrap_or_else(|e| panic!("{name}: {e}"));
        run_hours(&mut sim, 48, &mut seen, name);
        let week = sim.clock.calendar;
        let minutes = sim.clock.minutes;
        let mut day = day_of(minutes) + 1;
        while day % week.week_days != week.week_days - week.weekend_days
            || day as f64 * DAY - HOUR <= minutes
        {
            day += 1;
        }
        let delta = day as f64 * DAY - HOUR - minutes;
        assert!(delta > 0.0, "{name} reaches the weekend ahead");
        sim.clock.advance(delta);
        run_hours(&mut sim, 26, &mut seen, &format!("{name} weekend"));
    }
    assert_eq!(
        seen,
        Seen {
            weekday: true,
            weekend: true,
            housekeeping: true,
            commute: true
        }
    );
}
