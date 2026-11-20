//! A 1994 tower's elevator settings survive the trip through this game
//! (#305): each shaft's 56-byte schedule block comes in with the import,
//! stays on the shaft through a save and a load, and goes back out in an
//! export to 1994 as it came. Read from the observation saves the 1997 game
//! wrote (`conformance/fixtures/tdt-observed/`), whose shafts carry blocks
//! the game set, not our exporter's default.

use std::path::PathBuf;

use super::export_tdt;
use super::format::{parse_tdt_binary, tdt_elevator_schedule_default};
use super::parse::parse_tdt;
use crate::load::{deserialize, deserialize_tdt};
use crate::tower::LEGACY_SCHEDULE_SAVE_VERSION;

const SAVES: [&str; 7] = [
    "T5O01.TDT",
    "T5O35.TDT",
    "T7O01.TDT",
    "T7O76.TDT",
    "FRESH0.TDT",
    "FRESH3A.TDT",
    "FRESH9A.TDT",
];

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../conformance/fixtures/tdt-observed")
        .join(name);
    std::fs::read(path).expect("fixture")
}

/// Every built shaft's block in a file, sorted, so two files compare
/// whatever order their tables list the shafts in.
fn blocks(bytes: &[u8]) -> Vec<[u8; 56]> {
    let tower = parse_tdt_binary(bytes).expect("parses");
    let mut out: Vec<[u8; 56]> = tower
        .tail
        .elevators
        .expect("the elevator table reads")
        .iter()
        .map(|e| e.schedule)
        .collect();
    out.sort_unstable();
    out
}

fn is_default(block: &[u8; 56]) -> bool {
    let default = tdt_elevator_schedule_default();
    block.iter().zip(&default).all(|(&b, &d)| i64::from(b) == d)
}

#[test]
fn every_shafts_1994_settings_go_back_out_as_they_came() {
    let mut set_by_the_game = 0;
    for name in SAVES {
        let source = fixture(name);
        let parsed = parse_tdt(&source, name).expect("imports");
        let sim = deserialize_tdt(&parsed.save).expect("loads");
        // The trip a player takes: a save in this game's format, a load of
        // it, then the export.
        let reloaded = deserialize(&sim.serialize()).expect("reloads");
        let exported = export_tdt(&reloaded.save_text()).expect("exports");
        let want = blocks(&source);
        assert_eq!(blocks(&exported), want, "{name}");
        set_by_the_game += want.iter().filter(|b| !is_default(b)).count();
    }
    // The test means something only if the saves hold blocks the exporter
    // would not write on its own.
    assert!(set_by_the_game > 0);
}

#[test]
fn a_tower_keeping_1994_settings_is_saved_at_the_version_that_reads_them() {
    let name = "T7O01.TDT";
    let parsed = parse_tdt(&fixture(name), name).expect("imports");
    // The import itself is stamped with it, so a build that cannot read the
    // blocks refuses the import's save instead of dropping them.
    assert_eq!(parsed.save["version"], LEGACY_SCHEDULE_SAVE_VERSION);
    let sim = deserialize_tdt(&parsed.save).expect("loads");
    assert!(sim.tower.keeps_a_legacy_schedule());
    let save = sim.serialize();
    assert_eq!(save["version"], LEGACY_SCHEDULE_SAVE_VERSION);
    assert_eq!(
        sim.serialize_unfolded()["version"],
        LEGACY_SCHEDULE_SAVE_VERSION
    );
    // Each kept block is on an elevator, in the saved form a load reads.
    let kept: Vec<&str> = save["transports"]
        .as_array()
        .expect("transports")
        .iter()
        .filter_map(|t| t.get("tdtSchedule").and_then(|v| v.as_str()))
        .collect();
    assert!(!kept.is_empty());
    assert!(kept.iter().all(|s| s.len() == 112));
}

#[test]
fn a_shaft_with_the_default_block_keeps_nothing() {
    // Our own export writes the default block on every shaft, so importing
    // it back adds no field and leaves the save version alone.
    let name = "T7O01.TDT";
    let parsed = parse_tdt(&fixture(name), name).expect("imports");
    let mut save = parsed.save.clone();
    for t in save["transports"].as_array_mut().expect("transports") {
        t.as_object_mut().expect("object").remove("tdtSchedule");
    }
    let sim = deserialize_tdt(&save).expect("loads");
    assert!(!sim.tower.keeps_a_legacy_schedule());
    let exported = export_tdt(&sim.save_text()).expect("exports");
    let reimported = parse_tdt(&exported, "BACK.TDT").expect("reimports");
    assert!(reimported.save["version"].as_i64() < Some(LEGACY_SCHEDULE_SAVE_VERSION));
    let back = deserialize_tdt(&reimported.save).expect("loads");
    assert!(!back.tower.keeps_a_legacy_schedule());
    assert!(blocks(&exported).iter().all(is_default));
    assert!(back.serialize()["version"].as_i64() < Some(LEGACY_SCHEDULE_SAVE_VERSION));
}
