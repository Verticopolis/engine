//! What the `.TDT` importer makes of each room's own bytes: construction,
//! occupancy, hotel stays, rent classes and geometry, then the tower's star,
//! its identity fields and the facts the import report reads. The report's
//! wording is the game's; these tests pin the counts and notes it reads.

mod tdt_kit;

use serde_json::Value;
use tdt_kit::builder::{floor, sample, tenant, Spec, Tenant};
use tdt_kit::reading::{i, notes, of_kind, parse, rooms, with_floors};
use tdt_kit::{transports, units};
use verticopolis_engine::facilities::Kind;
use verticopolis_engine::sim::PLAIN_SAVE_VERSION;
use verticopolis_engine::tdt::parse::{parse_tdt, ParsedLegacyTower};

/// One tenant on `.TDT` row 20 (our floor 11).
fn one(left: i64, right: i64, type_id: i64, status: u8) -> Spec {
    with_floors(vec![floor(
        20,
        vec![Tenant {
            status,
            ..tenant(left, right, type_id)
        }],
    )])
}

/// The one room `spec` imports.
fn room(spec: &Spec) -> Value {
    let save = parse(spec).save;
    let got = rooms(&save);
    assert_eq!(got.len(), 1, "one room expected");
    got[0].clone()
}

fn hotel(status: u8) -> Value {
    room(&one(100, 104, 3, status))
}

fn named(spec: &Spec, filename: &str) -> ParsedLegacyTower {
    parse_tdt(&tdt_kit::builder::build(spec), filename).unwrap_or_else(|e| panic!("{}", e.0))
}

#[test]
fn an_unknown_tenant_type_is_dropped_and_counted() {
    let parsed = parse(&one(100, 109, 50, 0));
    assert!(rooms(&parsed.save).is_empty());
    assert_eq!(parsed.counts.unknown, 1);
}

#[test]
fn a_negative_type_is_under_construction_and_finishes_later() {
    let parsed = parse(&one(100, 109, -7, 0));
    let office = &of_kind(&parsed.save, "office")[0];
    assert_eq!(office["state"], "construction");
    let complete_at = office["completeAt"].as_f64().expect("a finite completeAt");
    assert!(complete_at > parsed.save["minutes"].as_f64().unwrap());
    assert_eq!(parsed.counts.construction, 1);
}

#[test]
fn a_tenanted_office_or_condo_imports_occupied_and_an_empty_one_vacant() {
    let office = room(&one(100, 109, 7, 1));
    assert_eq!(office["state"], "occupied");
    assert_eq!(office["everOccupied"], true);
    assert_eq!(i(&office["occupants"]), Kind::Office.facility().population);
    assert_eq!(office["satisfaction"], 1);
    let condo = room(&one(100, 113, 9, 1));
    assert_eq!(condo["state"], "occupied");
    assert_eq!(condo["everOccupied"], true);
    assert_eq!(i(&condo["occupants"]), Kind::Condo.facility().population);
    let vacant = room(&one(100, 109, 7, 0));
    assert_eq!(vacant["state"], "empty");
    assert_eq!(vacant["everOccupied"], false);
    assert_eq!(i(&vacant["occupants"]), 0);
}

#[test]
fn a_venue_opens_occupied_with_nobody_inside_yet() {
    // 12 fast food, 10 shop, 6 restaurant.
    for (left, right, type_id) in [(100, 116, 12), (100, 112, 10), (100, 124, 6)] {
        let venue = room(&one(left, right, type_id, 0));
        assert_eq!(venue["state"], "occupied", "type {type_id}");
        assert_eq!(venue["everOccupied"], true, "type {type_id}");
        assert_eq!(venue["customersIn"].as_i64().unwrap_or(0), 0);
    }
    let building = room(&one(100, 116, -12, 0));
    assert_eq!(building["state"], "construction");
    assert_eq!(i(&building["occupants"]), 0);
}

#[test]
fn hotel_status_flags_read_as_asleep_dirty_or_empty_and_infested_is_counted() {
    // The header clock is 7:00 AM, before checkout, so asleep guests stay.
    let asleep = hotel(16 | 2);
    assert_eq!(asleep["state"], "asleep");
    assert_eq!(i(&asleep["occupants"]), 1);
    assert_eq!(asleep["everOccupied"], true);
    let dirty = hotel(32);
    assert_eq!(dirty["state"], "dirty");
    assert_eq!(dirty["everOccupied"], true);
    let infested = parse(&one(100, 104, 3, 64));
    assert_eq!(of_kind(&infested.save, "hotelSingle")[0]["state"], "dirty");
    assert_eq!(infested.counts.infested, 1);
    let fresh = hotel(0);
    assert_eq!(fresh["state"], "empty");
    assert_eq!(fresh["everOccupied"], false);
}

#[test]
fn the_hotel_statuses_a_game_save_holds_decode() {
    // Booked (1) with the guests out; asleep with a count (17 to 19).
    let booked = hotel(1);
    assert_eq!(booked["state"], "empty");
    assert_eq!(booked["everOccupied"], true);
    for status in [17, 18, 19] {
        let room = hotel(status);
        assert_eq!(room["state"], "asleep", "status {status}");
        assert_eq!(i(&room["occupants"]), 1, "status {status}");
        assert_eq!(room["everOccupied"], true, "status {status}");
    }
}

#[test]
fn a_checked_in_room_whose_guests_are_out_arrives_with_its_guests() {
    for status in [8, 9, 10] {
        let room = hotel(status);
        assert_eq!(room["state"], "asleep", "status {status}");
        assert_eq!(i(&room["occupants"]), 1, "status {status}");
        assert_eq!(room["everOccupied"], true, "status {status}");
    }
}

#[test]
fn a_clean_vacant_room_arrives_empty_but_booked_and_is_counted() {
    let parsed = parse(&one(100, 104, 3, 24));
    let room = &of_kind(&parsed.save, "hotelSingle")[0];
    assert_eq!(room["state"], "empty");
    assert_eq!(room["everOccupied"], true);
    assert_eq!(parsed.counts.hotel_booked, 1);
}

#[test]
fn guests_keep_their_rooms_whatever_the_hour_the_file_was_saved() {
    // Tick 800 is 12:30 PM; the game counts checked-in guests all day.
    let room = room(&Spec {
        frame_time: 800,
        ..one(100, 104, 3, 16 | 1)
    });
    assert_eq!(room["state"], "asleep");
    assert_eq!(i(&room["occupants"]), 1);
}

fn rated(left: i64, right: i64, type_id: i64, rent_rate: u8) -> Value {
    room(&with_floors(vec![floor(
        20,
        vec![Tenant {
            rent_rate: Some(rent_rate),
            ..tenant(left, right, type_id)
        }],
    )]))
}

#[test]
fn rent_class_3_is_the_classic_high_rung_and_4_is_no_rate_on_priced_kinds_only() {
    assert_eq!(i(&rated(100, 109, 7, 3)["rent"]), 15_000);
    let office = rated(100, 109, 7, 4);
    assert_eq!(office["kind"], "office");
    assert_eq!(office["noRate"], true);
    assert!(office.get("rent").is_none(), "rent stays at the default");
    let shop = rated(100, 111, 10, 4);
    assert_eq!(shop["kind"], "shop");
    assert!(shop.get("noRate").is_none());
}

#[test]
fn a_room_keeps_its_recorded_width_and_a_mismatch_is_counted() {
    let parsed = parse(&one(100, 112, 7, 0));
    let office = &of_kind(&parsed.save, "office")[0];
    assert_eq!((i(&office["x"]), i(&office["width"])), (100, 12));
    assert_eq!(parsed.counts.width_mismatch, 1);
}

#[test]
fn of_two_overlapping_rooms_the_later_one_is_dropped_and_counted() {
    let parsed = parse(&with_floors(vec![floor(
        20,
        vec![
            tenant(100, 109, 7),
            tenant(105, 114, 7),
            tenant(114, 123, 7),
        ],
    )]));
    let xs: Vec<i64> = of_kind(&parsed.save, "office")
        .iter()
        .map(|u| i(&u["x"]))
        .collect();
    assert_eq!(xs, vec![100, 114]);
    assert_eq!(parsed.counts.overlapping, 1);
}

#[test]
fn a_room_off_the_lot_is_dropped_and_one_past_the_edge_is_trimmed() {
    let parsed = parse(&with_floors(vec![floor(
        20,
        vec![
            tenant(400, 410, 7),
            tenant(110, 100, 7),
            tenant(370, 380, 7),
        ],
    )]));
    let offices = of_kind(&parsed.save, "office");
    assert_eq!(offices.len(), 1);
    assert_eq!((i(&offices[0]["x"]), i(&offices[0]["width"])), (370, 5));
    assert_eq!(parsed.counts.off_lot, 2);
    assert_eq!(parsed.counts.clamped, 1);
}

#[test]
fn the_star_clamps_to_1_through_6_and_sets_the_review_flags() {
    let star = |level| {
        parse(&Spec {
            level,
            ..Spec::default()
        })
        .save
    };
    assert_eq!(star(0)["star"], 1);
    assert_eq!(star(9)["star"], 6);
    for (level, evaluated, vip) in [(6, true, true), (3, false, false), (4, false, true)] {
        let save = star(level);
        assert_eq!(save["evaluatedTower"], evaluated, "level {level}");
        assert_eq!(save["vipFavorable"], vip, "level {level}");
    }
}

#[test]
fn the_save_is_classic_named_for_the_file_seeded_by_its_bytes_and_numbered_past_its_ids() {
    let a = named(&sample(), "MY_TOWER.TDT").save;
    assert_eq!(a["mode"], "classic");
    assert_eq!(i(&a["version"]), PLAIN_SAVE_VERSION);
    assert_eq!(a["towerName"], "MY TOWER");
    let b = named(&sample(), "OTHER.TDT").save;
    assert_eq!(a["seed"], b["seed"]);
    let other = parse(&Spec {
        balance: 999,
        ..Spec::default()
    })
    .save;
    assert_ne!(other["seed"], a["seed"]);
    let max_id = units(&a)
        .iter()
        .chain(transports(&a))
        .map(|v| i(&v["id"]))
        .max()
        .unwrap();
    assert_eq!(i(&a["nextId"]), max_id + 1);
}

#[test]
fn the_sample_carries_every_fact_the_import_report_reads() {
    let parsed = named(&sample(), "GRAND.TDT");
    let save = &parsed.save;
    assert_eq!(save["towerName"], "GRAND");
    assert_eq!(save["star"], 2);
    assert_eq!(save["money"], 1_500_000);
    let floors: Vec<i64> = units(save).iter().map(|u| i(&u["floor"])).collect();
    assert_eq!(floors.iter().max(), Some(&5));
    assert_eq!(floors.iter().min(), Some(&0), "one basement level");
    assert_eq!(i(&save["minutes"]) / 1440, 3, "current day 3 is day 4");
    assert!(parsed.counts.rooms > 0);
    assert!(parsed.decoded, "the transports come from the save");
    assert!(!transports(save).is_empty());
    assert!(parsed.counts.parking_stalls > 0);
    assert!(parsed.counts.rents_applied > 0);
    assert!(parsed.counts.twin_rooms > 0);
    assert_eq!(parsed.tower.tail.people_count, Some(12));
    assert_eq!(parsed.tower.tail.parking_connected, Some(1));
    assert!(!notes(&parsed).contains("rebuilt"));
}
