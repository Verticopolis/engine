//! Reading a built spec through the importer, and the parts of a parse the
//! importer tests look at.

use serde_json::Value;
use verticopolis_engine::tdt::parse::{parse_tdt, ParsedLegacyTower};

use super::builder::{build, Floor, Spec};
use super::units;

pub fn parse(spec: &Spec) -> ParsedLegacyTower {
    parse_tdt(&build(spec), "TOWER.TDT").unwrap_or_else(|e| panic!("refused: {}", e.0))
}

pub fn refusal(spec: &Spec) -> String {
    match parse_tdt(&build(spec), "TOWER.TDT") {
        Ok(_) => panic!("the file was read"),
        Err(e) => e.0,
    }
}

pub fn with_floors(floors: Vec<Floor>) -> Spec {
    Spec {
        floors,
        ..Spec::default()
    }
}

pub fn of_kind<'a>(save: &'a Value, kind: &str) -> Vec<&'a Value> {
    units(save).iter().filter(|u| u["kind"] == kind).collect()
}

pub fn rooms(save: &Value) -> Vec<&Value> {
    units(save)
        .iter()
        .filter(|u| u["kind"] != "floor" && u["kind"] != "lobby")
        .collect()
}

pub fn notes(parsed: &ParsedLegacyTower) -> String {
    [&parsed.header_notes[..], &parsed.warnings[..]]
        .concat()
        .join(" ")
}

pub fn i(v: &Value) -> i64 {
    v.as_i64().unwrap_or_else(|| panic!("not an integer: {v}"))
}
