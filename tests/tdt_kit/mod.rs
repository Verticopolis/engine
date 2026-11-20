//! Shared helpers for the export tests: the sample tower, a hand-built save,
//! and raw walks over a written file. The walks read the bytes in file order,
//! without the importer's merge and sort, so a test can see what the writer
//! put where.
#![allow(dead_code)]

pub mod builder;

use std::io::Read;
use std::path::PathBuf;

use serde_json::{json, Value};
use verticopolis_engine::sim::SAVE_VERSION;
use verticopolis_engine::tdt::export::{build_tdt, BuiltLegacyTower};
use verticopolis_engine::tdt::format::{
    TDT_FLOOR_COUNT, TDT_FLOOR_INDEX_ENTRIES, TDT_HEADER_SIZE, TDT_TENANT_RECORD_SIZE,
};
use verticopolis_engine::tdt::parse::{parse_tdt, ParsedLegacyTower};

/// The bytes of the conformance lock's import case `id`. The lock's import
/// cases are synthetic `.TDT` files, one per shape the importer must read.
pub fn case_bytes(id: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("conformance/tdt-cases.json");
    let lock: Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("lock")).expect("json");
    let case = lock["import"]
        .as_array()
        .expect("import cases")
        .iter()
        .find(|c| c["id"] == id)
        .unwrap_or_else(|| panic!("the lock holds no import case {id}"));
    let packed = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        case["bytes"].as_str().expect("bytes"),
    )
    .expect("base64");
    let mut bytes = Vec::new();
    flate2::read::DeflateDecoder::new(&packed[..])
        .read_to_end(&mut bytes)
        .expect("inflate");
    bytes
}

/// The sample tower the conformance lock's import cases start from: its
/// `.TDT` file pulled through the importer, so it carries rooms, hotel
/// states, transports, paving and parking in the shape the game saves.
pub fn sample_save() -> Value {
    import(&case_bytes("sample")).save
}

/// A room or paving tile for a hand-built save.
pub fn unit(id: i64, kind: &str, floor: i64, x: i64, width: i64) -> Value {
    json!({
        "id": id, "kind": kind, "floor": floor, "x": x, "width": width,
        "state": "empty", "satisfaction": 1, "occupants": 0, "everOccupied": false,
        "pendingIncome": 0, "label": "",
    })
}

/// [`unit`] in a given state.
pub fn unit_in(id: i64, kind: &str, floor: i64, x: i64, width: i64, state: &str) -> Value {
    let mut u = unit(id, kind, floor, x, width);
    u["state"] = json!(state);
    u
}

/// An elevator or walkway with every car at its bottom floor.
pub fn shaft(id: i64, kind: &str, x: i64, bottom: i64, top: i64, cars: usize) -> Value {
    let width = match kind {
        "elevatorExpress" | "stairs" | "escalator" => 8,
        _ => 4,
    };
    json!({
        "id": id, "kind": kind, "x": x, "width": width, "bottom": bottom, "top": top,
        "cars": cars, "carPositions": vec![bottom; cars], "carDir": vec![0; cars], "load": 0,
    })
}

/// A Classic save holding `units` and `transports`.
pub fn tower(units: Vec<Value>, transports: Vec<Value>) -> Value {
    json!({
        "version": SAVE_VERSION, "seed": 1, "money": 2_000_000, "star": 1, "minutes": 420,
        "mode": "classic", "nextId": units.len() + 1, "towerName": "Test",
        "builtWeddingHall": false, "evaluatedTower": false,
        "units": units, "transports": transports,
    })
}

pub fn build(save: &Value) -> BuiltLegacyTower {
    build_tdt(save).unwrap_or_else(|e| panic!("the export was refused: {}", e.0))
}

pub fn import(bytes: &[u8]) -> ParsedLegacyTower {
    parse_tdt(bytes, "S.TDT").unwrap_or_else(|e| panic!("the import was refused: {}", e.0))
}

/// The importer's notes on a file: its header notes, then the walk's
/// warnings. Our own output must raise none.
pub fn notes(bytes: &[u8]) -> Vec<String> {
    let back = import(bytes);
    [back.header_notes, back.warnings].concat()
}

/// The save's units, as a list.
pub fn units(save: &Value) -> &Vec<Value> {
    save["units"].as_array().expect("units")
}

pub fn transports(save: &Value) -> &Vec<Value> {
    save["transports"].as_array().expect("transports")
}

pub fn units_mut(save: &mut Value) -> &mut Vec<Value> {
    save["units"].as_array_mut().expect("units")
}

pub fn transports_mut(save: &mut Value) -> &mut Vec<Value> {
    save["transports"].as_array_mut().expect("transports")
}

pub fn u16_at(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

pub fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

pub fn i32_at(bytes: &[u8], at: usize) -> i32 {
    u32_at(bytes, at) as i32
}

/// One tenant record as it sits in the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Record {
    pub left: i64,
    pub right: i64,
    pub type_id: i64,
    pub status: u8,
    pub rent_class: u8,
    pub subtype: u8,
}

/// A row of the floor map: the header's extent and its records in file order.
pub struct Row {
    pub left: i64,
    pub right: i64,
    pub records: Vec<Record>,
}

/// Where the people block starts: right after the floor map.
pub fn people_at(bytes: &[u8]) -> usize {
    let mut at = TDT_HEADER_SIZE;
    for _ in 0..TDT_FLOOR_COUNT {
        at += 6 + u16_at(bytes, at) as usize * TDT_TENANT_RECORD_SIZE + TDT_FLOOR_INDEX_ENTRIES * 2;
    }
    at
}

/// The row at a `.TDT` floor index, walked straight from the bytes.
pub fn row(bytes: &[u8], tdt_index: i64) -> Row {
    let mut at = TDT_HEADER_SIZE;
    for index in 0..TDT_FLOOR_COUNT {
        let count = u16_at(bytes, at) as usize;
        let base = at + 6;
        if index == tdt_index {
            let records = (0..count)
                .map(|k| {
                    let r = base + k * TDT_TENANT_RECORD_SIZE;
                    Record {
                        left: u16_at(bytes, r) as i64,
                        right: u16_at(bytes, r + 2) as i64,
                        type_id: bytes[r + 4] as i8 as i64,
                        status: bytes[r + 5],
                        rent_class: bytes[r + 16],
                        subtype: bytes[r + 17],
                    }
                })
                .collect();
            return Row {
                left: u16_at(bytes, at + 2) as i64,
                right: u16_at(bytes, at + 4) as i64,
                records,
            };
        }
        at = base + count * TDT_TENANT_RECORD_SIZE + TDT_FLOOR_INDEX_ENTRIES * 2;
    }
    Row {
        left: 0,
        right: 0,
        records: vec![],
    }
}

/// The tiles a row's paving records (empty floor 0, lobby 24) cover.
pub fn paved(row: &Row) -> std::collections::BTreeSet<i64> {
    row.records
        .iter()
        .filter(|r| r.type_id == 0 || r.type_id == 24)
        .flat_map(|r| r.left..r.right)
        .collect()
}
