//! Save fields the engine reads and writes back. Each test takes the save
//! of a small office tower after five hours, edits one field the way a
//! newer build, an older build or a hand edit would, loads the edited text
//! and reads the save it writes: the VIP's pending day and visit count
//! survive (a forged count clamps, an older save adopts the visits its
//! flags prove), forged star, money and clock values fall back to sane ones
//! while legal ones stay untouched, an overflowing literal or a clock that
//! is no number is refused, and a version 3 save climbs the ladder with its
//! tower intact.
use serde_json::{json, Value};

use crate::clock::GameMode;
use crate::facilities::{Kind, LOT_WIDTH};
use crate::load::unfolded_units;
use crate::load_stream::load_save_text;
use crate::sim::{Simulation, STARTING_MONEY};
use crate::tower::UnitState;

use super::seeded_game;

/// A saved number, whether it was written whole or with a fraction.
fn num(v: &Value) -> Option<f64> {
    v.as_f64()
}

fn played_save() -> Value {
    let mut sim = seeded_game(42, GameMode::Classic);
    let x0 = LOT_WIDTH / 2 - 20;
    for i in 0..12 {
        assert!(sim.tower.place(Kind::Floor, 2, x0 + i).ok);
    }
    assert!(sim.build_transport(Kind::ElevatorStandard, x0, 1, 2).ok);
    assert!(sim.build(Kind::Office, 2, x0).ok);
    sim.money = 1_234_567.0;
    sim.tick(5.0 * 60.0);
    sim.serialize()
}

fn load(save: &Value) -> Simulation {
    load_save_text(save.to_string().as_bytes()).unwrap()
}

fn resave(save: &Value) -> Value {
    load(save).serialize()
}

fn with(save: &Value, key: &str, value: Value) -> Value {
    let mut out = save.clone();
    out[key] = value;
    out
}

fn without(save: &Value, key: &str) -> Value {
    let mut out = save.clone();
    out.as_object_mut().unwrap().remove(key);
    out
}

/// The save text with `key` written as an overflowing literal, the way a
/// hand edit spells an infinity.
fn overflowing(save: &Value, key: &str, negative: bool) -> String {
    let marked = with(save, key, json!("__overflow__")).to_string();
    let literal = if negative { "-1e999" } else { "1e999" };
    marked.replace("\"__overflow__\"", literal)
}

#[test]
fn the_vips_pending_day_survives_a_load() {
    let save = played_save();
    let due = load(&save).clock.day() + 3;
    assert_eq!(
        num(&resave(&with(&save, "vipVisitDay", json!(due)))["vipVisitDay"]),
        Some(due as f64)
    );
}

#[test]
fn the_vip_count_survives_clamps_a_forgery_and_adopts_what_old_flags_prove() {
    let mut save = with(&played_save(), "vipFavorable", json!(true));
    for (forged, want) in [
        (json!(3), 3),
        (json!(-5), 0),
        (Value::Null, 0),
        (json!(1e300), 1_000_000),
        (json!(0), 0),
    ] {
        let got = resave(&with(&save, "vipVisits", forged.clone()))["vipVisits"].clone();
        assert_eq!(num(&got), Some(want as f64), "vipVisits {forged}");
    }
    save = without(&save, "vipVisits");
    assert_eq!(
        num(&resave(&save)["vipVisits"]),
        Some(1.0),
        "a favorable review"
    );
    save = with(&save, "evaluatedTower", json!(true));
    assert_eq!(
        num(&resave(&save)["vipVisits"]),
        Some(2.0),
        "and a won tower"
    );
    save = with(&save, "evaluatedTower", json!(false));
    save = with(&save, "vipFavorable", json!(false));
    assert_eq!(num(&resave(&save)["vipVisits"]), Some(0.0));
}

#[test]
fn legal_star_money_and_clock_values_load_untouched() {
    let save = with(
        &with(&played_save(), "star", json!(3)),
        "money",
        json!(1_234_567),
    );
    let clean = resave(&save);
    assert_eq!(num(&clean["star"]), Some(3.0));
    assert_eq!(num(&clean["money"]), Some(1_234_567.0));
    assert_eq!(num(&clean["minutes"]), num(&save["minutes"]));
    assert_eq!(
        num(&resave(&with(&save, "money", json!(-50_000)))["money"]),
        Some(-50_000.0)
    );
    assert_eq!(
        num(&resave(&with(&save, "minutes", json!(420.5)))["minutes"]),
        Some(420.5)
    );
}

#[test]
fn a_forged_star_clamps_onto_the_ladder() {
    let save = played_save();
    for (forged, want) in [
        (Value::Null, 1),
        (json!(0), 1),
        (json!(99), 6),
        (json!(-4), 1),
        (json!("5"), 1),
    ] {
        assert_eq!(
            num(&resave(&with(&save, "star", forged.clone()))["star"]),
            Some(want as f64),
            "star {forged}"
        );
    }
}

#[test]
fn forged_money_falls_back_to_the_starting_balance() {
    let save = played_save();
    for forged in [Value::Null, json!("1000")] {
        assert_eq!(
            num(&resave(&with(&save, "money", forged.clone()))["money"]),
            Some(STARTING_MONEY),
            "money {forged}"
        );
    }
    assert_eq!(
        num(&resave(&without(&save, "money"))["money"]),
        Some(STARTING_MONEY)
    );
}

#[test]
fn a_negative_clock_opens_at_the_founding_hour_and_keeps_running() {
    let mut sim = load(&with(&played_save(), "minutes", json!(-100)));
    assert_eq!(num(&sim.serialize()["minutes"]), Some(420.0));
    sim.tick(60.0);
}

#[test]
fn a_clock_that_is_no_number_or_an_overflowing_literal_is_refused() {
    let save = played_save();
    for forged in [Value::Null, json!("600")] {
        let err = load_save_text(
            with(&save, "minutes", forged.clone())
                .to_string()
                .as_bytes(),
        )
        .err();
        assert!(
            err.is_some_and(|e| e.contains("no game clock")),
            "minutes {forged}"
        );
    }
    let err = load_save_text(without(&save, "minutes").to_string().as_bytes()).err();
    assert!(err.is_some_and(|e| e.contains("no game clock")));
    for key in ["minutes", "money"] {
        for negative in [false, true] {
            let err = load_save_text(overflowing(&save, key, negative).as_bytes()).err();
            assert!(
                err.is_some_and(|e| e.contains("number out of range")),
                "{key} {negative}"
            );
        }
    }
}

#[test]
fn a_version_3_save_climbs_the_ladder_with_its_tower_intact() {
    let mut played = played_save();
    for u in played["units"].as_array_mut().unwrap() {
        if u["kind"] == "office" {
            u["state"] = json!("occupied");
            u["occupants"] = json!(6);
        }
    }
    let mut save = resave(&played);
    let population = load(&save).tower.total_population();
    assert!(population > 0, "the office is tenanted");
    // A version 3 save lists every floor tile on its own.
    let tiles: Vec<Value> = unfolded_units(&save)
        .unwrap()
        .map(|u| u.into_owned())
        .collect();
    save["units"] = Value::Array(tiles.clone());
    save["version"] = json!(3);
    let sim = load(&save);
    let after = sim.serialize();
    assert_eq!(num(&after["version"]), Some(11.0), "the floor-run version");
    assert_eq!(num(&after["money"]), num(&save["money"]));
    assert_eq!(sim.tower.total_population(), population);
    let folded = after["units"].as_array().unwrap().len();
    assert!(folded < tiles.len(), "the floor row folds into a run");
    let unfolded: Vec<Value> = unfolded_units(&after)
        .unwrap()
        .map(|u| u.into_owned())
        .collect();
    assert_eq!(unfolded, tiles);
    assert_eq!(after["transports"], save["transports"]);
    let office = sim
        .tower
        .units
        .iter()
        .find(|u| u.kind == Kind::Office)
        .unwrap();
    assert_eq!(office.state, UnitState::Occupied);
}
