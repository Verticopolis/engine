//! What the `.TDT` writer puts in a file, read back off the bytes and through
//! the importer: the round trip of the sample tower, the elevator table's
//! sizes and order, the header counts, the record order on a row, rent
//! classes, and saves the game cannot make but a hand-edited file can.

mod tdt_kit;

use serde_json::{json, Value};
use tdt_kit::*;
use verticopolis_engine::facilities::{LOBBY_INTERVAL, LOT_WIDTH};
use verticopolis_engine::tdt::format::{
    parse_tdt_binary, TDT_ELEVATOR_PER_FLOOR_SIZE, TDT_FLOOR_OFFSET,
};

fn non_paving(save: &Value) -> Vec<Value> {
    let mut rooms: Vec<Value> = units(save)
        .iter()
        .filter(|u| u["kind"] != "floor" && u["kind"] != "lobby")
        .map(|u| {
            json!([
                u["kind"],
                u["floor"],
                u["x"],
                u["width"],
                u["state"],
                u["everOccupied"],
                u["occupants"],
                u["rent"]
            ])
        })
        .collect();
    rooms.sort_by_key(|r| (r[1].as_i64(), r[2].as_i64()));
    rooms
}

fn shaft_facts(save: &Value) -> Vec<Value> {
    let mut out: Vec<Value> = transports(save)
        .iter()
        .map(|t| {
            let mut skips: Vec<i64> = t["skipFloors"]
                .as_array()
                .map(|a| a.iter().filter_map(Value::as_i64).collect())
                .unwrap_or_default();
            skips.sort();
            json!([t["kind"], t["x"], t["bottom"], t["top"], t["cars"], skips])
        })
        .collect();
    out.sort_by_key(|t| t.to_string());
    out
}

fn shaft_at<'a>(save: &'a Value, kind: &str, x: i64) -> &'a Value {
    transports(save)
        .iter()
        .find(|t| t["kind"] == kind && t["x"] == x)
        .unwrap_or_else(|| panic!("no {kind} at {x}"))
}

fn skips(t: &Value) -> Vec<i64> {
    t["skipFloors"]
        .as_array()
        .map(|a| a.iter().filter_map(Value::as_i64).collect())
        .unwrap_or_default()
}

/// The x of each elevator slot the file holds, in table order.
fn slots(bytes: &[u8]) -> Vec<i64> {
    let tower = parse_tdt_binary(bytes).expect("reads");
    tower
        .tail
        .elevators
        .expect("the elevator table")
        .iter()
        .map(|e| e.x)
        .collect()
}

fn with_shaft(shaft: Value) -> Value {
    let mut save = sample_save();
    transports_mut(&mut save).push(shaft);
    save
}

#[test]
fn the_sample_keeps_its_rooms_shafts_money_star_and_clock_through_export_and_import() {
    let save = sample_save();
    let back = import(&build(&save).bytes).save;
    assert_eq!(non_paving(&back), non_paving(&save));
    assert_eq!(shaft_facts(&back), shaft_facts(&save));
    for key in ["money", "star", "minutes"] {
        assert_eq!(back[key], save[key], "{key}");
    }
    assert_eq!(back["mode"], "classic");
}

#[test]
fn our_own_export_raises_no_importer_notes_and_writes_the_same_bytes_again() {
    let first = build(&sample_save()).bytes;
    assert_eq!(notes(&first), Vec::<String>::new());
    assert_eq!(build(&import(&first).save).bytes, first);
}

#[test]
fn a_shafts_payload_does_not_grow_with_its_cars() {
    // One fixed car block per shaft: sizing it per car desyncs every slot after it.
    let one = with_shaft(shaft(100, "elevatorStandard", 100, 1, 10, 1));
    let eight = with_shaft(shaft(100, "elevatorStandard", 100, 1, 10, 8));
    assert_eq!(build(&eight).bytes.len(), build(&one).bytes.len());
}

#[test]
fn a_shafts_payload_counts_the_floors_it_spans_and_keeps_its_stops() {
    let skipping = |top: i64, skip: Vec<i64>| {
        let mut s = shaft(100, "elevatorStandard", 100, 1, top, 2);
        s["skipFloors"] = json!(skip);
        with_shaft(s)
    };
    let everywhere = skipping(12, vec![]);
    let skips_five = skipping(12, vec![4, 5, 6, 7, 8]);
    assert_eq!(
        build(&skips_five).bytes.len(),
        build(&everywhere).bytes.len()
    );
    // Ten more floors of span, all of them skipped, add ten per-floor entries.
    let taller = skipping(22, (12..22).collect());
    let grown = build(&taller).bytes.len() - build(&everywhere).bytes.len();
    assert_eq!(grown as i64, 10 * TDT_ELEVATOR_PER_FLOOR_SIZE);
    let back = import(&build(&skips_five).bytes).save;
    assert_eq!(
        skips(shaft_at(&back, "elevatorStandard", 100)),
        vec![4, 5, 6, 7, 8]
    );
}

#[test]
fn a_tower_with_no_tenants_exports_to_a_fixed_length() {
    // A literal, because the writer and reader share every block size and
    // would move together. The tower has no tenants, so no economy tuning
    // moves the people block. If this fails, find which block size changed
    // (the three elevator block sizes, the routing tail, or a fixed block)
    // before changing the number.
    let mut units_list = vec![unit_in(1, "lobby", 1, 100, 1, "occupied")];
    for i in 0..4 {
        units_list.push(unit(10 + i, "floor", 2 + i, 100, 9));
    }
    let mut lift = shaft(50, "elevatorStandard", 90, 1, 8, 2);
    lift["skipFloors"] = json!([3, 4]);
    let mut save = tower(units_list, vec![lift]);
    save["money"] = json!(1_000_000);
    save["minutes"] = json!(600);
    let built = build(&save);
    assert_eq!(built.counts.rooms, 0);
    assert_eq!(built.bytes.len(), 75_551);
}

#[test]
fn a_shaft_after_one_that_skips_floors_still_reads_back() {
    let mut save = sample_save();
    let mut skipper = shaft(200, "elevatorStandard", 100, 1, 12, 2);
    skipper["skipFloors"] = json!([4, 5, 6, 7, 8]);
    transports_mut(&mut save).push(skipper);
    transports_mut(&mut save).push(shaft(201, "elevatorService", 120, 2, 9, 3));
    let bytes = build(&save).bytes;
    assert_eq!(notes(&bytes), Vec::<String>::new());
    // Only a slot after the skipping one would read wrong, so pin the order.
    let order = slots(&bytes);
    let at = |x: i64| {
        order
            .iter()
            .position(|&s| s == x)
            .unwrap_or_else(|| panic!("no slot at {x}"))
    };
    assert!(at(120) > at(100), "{order:?}");
    let back = import(&bytes).save;
    let trailing = shaft_at(&back, "elevatorService", 120);
    assert_eq!(
        (
            trailing["bottom"].as_i64(),
            trailing["top"].as_i64(),
            trailing["cars"].as_i64()
        ),
        (Some(2), Some(9), Some(3))
    );
    assert_eq!(
        skips(shaft_at(&back, "elevatorStandard", 100)),
        vec![4, 5, 6, 7, 8]
    );
}

#[test]
fn a_modern_tower_is_refused() {
    let mut save = sample_save();
    save["mode"] = json!("modern");
    let refusal = verticopolis_engine::tdt::export::build_tdt(&save)
        .err()
        .expect("refused");
    assert!(refusal.0.contains("Classic towers"), "{}", refusal.0);
}

#[test]
fn an_empty_tower_exports_and_reads_back_empty() {
    let built = build(&tower(vec![], vec![]));
    assert_eq!(built.counts.rooms, 0);
    let back = import(&built.bytes).save;
    assert!(units(&back).is_empty());
    assert_eq!(back["money"], 2_000_000);
}

#[test]
fn the_security_count_stops_at_ten() {
    let mut list = vec![unit(0, "lobby", 1, 0, 1)];
    list.extend((0..12).map(|i| unit(i + 1, "security", 2 + i, 0, 8)));
    assert_eq!(u16_at(&build(&tower(list, vec![])).bytes, 0x30), 10);
}

#[test]
fn every_kind_the_header_counts_writes_a_room() {
    // The header counts and the record tables are kept apart, so a counted
    // kind with no record would make the header outrun the map.
    for (kind, width) in [
        ("recycling", 20),
        ("shop", 12),
        ("restaurant", 24),
        ("fastFood", 16),
        ("security", 8),
        ("partyHall", 24),
        ("cinema", 31),
    ] {
        let built = build(&tower(
            vec![unit(1, "lobby", 1, 0, 1), unit(2, kind, 3, 0, width)],
            vec![],
        ));
        assert!(built.counts.rooms >= 1, "{kind}");
    }
}

#[test]
fn a_wide_row_writes_its_records_left_to_right_with_the_gap_between_rooms() {
    let floor = 3;
    let save = tower(
        vec![
            unit(1, "lobby", 1, 0, 1),
            unit(2, "office", floor, 0, 9),
            unit(3, "office", floor, 9, 9),
            unit(4, "office", floor, 18, 9),
            unit(5, "floor", floor, 27, 1),
            unit(6, "floor", floor, 28, 1),
            unit(7, "office", floor, 29, 9),
        ],
        vec![],
    );
    let records = row(&build(&save).bytes, floor + TDT_FLOOR_OFFSET).records;
    let lefts: Vec<i64> = records.iter().map(|r| r.left).collect();
    let mut sorted = lefts.clone();
    sorted.sort();
    assert_eq!(lefts, sorted);
    let gap = records
        .iter()
        .position(|r| r.type_id == 0 && r.left == 27)
        .expect("the gap");
    let past = records
        .iter()
        .position(|r| r.type_id == 7 && r.left == 29)
        .expect("the office past it");
    assert!(gap < past);
}

#[test]
fn a_forged_x_still_writes_the_row_in_the_order_of_its_written_edges() {
    let floor = 3;
    let mut forged = unit(3, "office", floor, 0, 9);
    forged["x"] = Value::Null;
    let save = tower(
        vec![
            unit(1, "lobby", 1, 0, 1),
            unit(2, "office", floor, 0, 9),
            forged,
            unit(4, "office", floor, 18, 9),
        ],
        vec![],
    );
    let a = build(&save).bytes;
    assert_eq!(build(&save).bytes, a);
    let lefts: Vec<i64> = row(&a, floor + TDT_FLOOR_OFFSET)
        .records
        .iter()
        .map(|r| r.left)
        .collect();
    let mut sorted = lefts.clone();
    sorted.sort();
    assert_eq!(lefts, sorted);
}

#[test]
fn a_plain_floor_on_a_sky_lobby_story_is_written_as_the_lobby() {
    let sky = LOBBY_INTERVAL * 2;
    let with = |kind: &str, floor: i64| {
        tower(
            vec![
                unit(1, "lobby", 1, 0, 1),
                unit(2, kind, floor, 40, 1),
                unit(3, kind, floor, 41, 1),
                unit(4, kind, floor, 42, 1),
            ],
            vec![],
        )
    };
    let as_floor = build(&with("floor", sky)).bytes;
    assert_eq!(as_floor, build(&with("lobby", sky)).bytes);
    let records = row(&as_floor, sky + TDT_FLOOR_OFFSET).records;
    assert!(!records.is_empty() && records.iter().all(|r| r.type_id == 24));
    let back = import(&as_floor).save;
    let kinds: Vec<&Value> = units(&back)
        .iter()
        .filter(|u| u["floor"] == sky)
        .map(|u| &u["kind"])
        .collect();
    assert!(kinds.contains(&&json!("lobby")) && !kinds.contains(&&json!("floor")));
    // One story up, the same tiles are plain floor.
    let plain = row(
        &build(&with("floor", sky + 1)).bytes,
        sky + 1 + TDT_FLOOR_OFFSET,
    )
    .records;
    assert!(!plain.is_empty() && plain.iter().all(|r| r.type_id == 0));
}

#[test]
fn stacked_stair_flights_come_back_one_per_floor() {
    let mut save = sample_save();
    for i in 0..3 {
        transports_mut(&mut save).push(shaft(8_000 + i, "stairs", 200, 1 + i, 2 + i, 0));
    }
    let back = import(&build(&save).bytes).save;
    let mut flights: Vec<(i64, i64)> = transports(&back)
        .iter()
        .filter(|t| t["kind"] == "stairs" && t["x"] == 200)
        .map(|t| (t["bottom"].as_i64().unwrap(), t["top"].as_i64().unwrap()))
        .collect();
    flights.sort();
    assert_eq!(flights, vec![(1, 2), (2, 3), (3, 4)]);
}

#[test]
fn money_rounds_to_the_hundred_and_a_room_name_is_counted_as_left_behind() {
    let mut save = sample_save();
    save["money"] = json!(1_234_567);
    let office = units_mut(&mut save)
        .iter_mut()
        .find(|u| u["kind"] == "office")
        .expect("an office");
    office["label"] = json!("Corner Office");
    let built = build(&save);
    assert_eq!(built.stats.balance * 100, 1_234_600);
    assert_eq!(built.stats.money, 1_234_567.0);
    assert!(built.counts.names_dropped >= 1);
    assert!(built.stats.elevators_len >= 1);
}

#[test]
fn no_rate_and_unpriced_kinds_write_rent_class_4_and_no_rate_survives_a_round_trip() {
    let class_of = |bytes: &[u8], type_id: i64| {
        row(bytes, 5 + TDT_FLOOR_OFFSET)
            .records
            .iter()
            .find(|r| r.type_id.abs() == type_id)
            .map(|r| r.rent_class)
    };
    let mut office = unit(1, "office", 5, 100, 9);
    office["noRate"] = json!(true);
    let first = build(&tower(vec![office], vec![])).bytes;
    assert_eq!(class_of(&first, 7), Some(4));
    // Fast food charges no rent; the game stores it as class 4 as well.
    let fast = build(&tower(vec![unit(1, "fastFood", 5, 100, 16)], vec![])).bytes;
    assert_eq!(class_of(&fast, 12), Some(4));
    let back = import(&first).save;
    let office = units(&back)
        .iter()
        .find(|u| u["kind"] == "office")
        .expect("the office");
    assert_eq!(office["noRate"], true);
    assert_eq!(class_of(&build(&back).bytes, 7), Some(4));
}

#[test]
fn every_classic_rent_rung_writes_its_class_and_reads_back_exactly() {
    let ladders: [(&str, i64, i64, [i64; 4]); 5] = [
        ("office", 7, 9, [2_000, 5_000, 10_000, 15_000]),
        ("condo", 9, 16, [50_000, 100_000, 150_000, 200_000]),
        ("hotelSingle", 3, 4, [500, 1_500, 2_000, 3_000]),
        ("hotelDouble", 4, 6, [800, 2_000, 3_000, 4_500]),
        ("hotelSuite", 5, 10, [1_500, 4_000, 6_000, 9_000]),
    ];
    let mut list = vec![];
    let mut id = 1;
    for (row_at, (kind, _, width, rents)) in ladders.iter().enumerate() {
        for (i, rent) in rents.iter().enumerate() {
            let mut u = unit(id, kind, 5 + row_at as i64, 20 + i as i64 * 40, *width);
            u["rent"] = json!(rent);
            list.push(u);
            id += 1;
        }
    }
    let mut no_rate = unit(id, "office", 5, 200, 9);
    no_rate["noRate"] = json!(true);
    list.push(no_rate);
    let first = build(&tower(list, vec![]));
    assert_eq!(first.counts.rents_snapped, 0);
    for (row_at, (kind, type_id, _, _)) in ladders.iter().enumerate() {
        let mut records: Vec<Record> = row(&first.bytes, 5 + row_at as i64 + TDT_FLOOR_OFFSET)
            .records
            .into_iter()
            .filter(|r| r.type_id.abs() == *type_id)
            .collect();
        records.sort_by_key(|r| r.left);
        let classes: Vec<u8> = records.iter().map(|r| r.rent_class).collect();
        let expected: Vec<u8> = if *kind == "office" {
            vec![0, 1, 2, 3, 4]
        } else {
            vec![0, 1, 2, 3]
        };
        assert_eq!(classes, expected, "{kind}");
    }
    let back = import(&first.bytes).save;
    for (kind, _, _, rents) in ladders {
        let mut found: Vec<(i64, i64)> = units(&back)
            .iter()
            .filter(|u| u["kind"] == kind && u["noRate"] != true)
            .map(|u| {
                (
                    u["x"].as_i64().unwrap(),
                    u["rent"].as_i64().expect("a rent on each rung"),
                )
            })
            .collect();
        found.sort();
        assert_eq!(
            found.iter().map(|r| r.1).collect::<Vec<_>>(),
            rents.to_vec(),
            "{kind}"
        );
    }
    assert_eq!(notes(&first.bytes), Vec::<String>::new());
    assert_eq!(build(&back).bytes, first.bytes);
}

#[test]
fn last_quarters_money_is_written_in_hundreds_at_0x10() {
    let mut save = tower(vec![], vec![]);
    save["lastQuarterMoney"] = json!(1_500_000);
    assert_eq!(i32_at(&build(&save).bytes, 0x10), 15_000);
    assert_eq!(i32_at(&build(&tower(vec![], vec![])).bytes, 0x10), 0);
}

#[test]
fn a_tenant_moving_in_or_out_is_written_as_a_sitting_tenant() {
    let mut save = sample_save();
    let office = units_mut(&mut save)
        .iter_mut()
        .find(|u| u["kind"] == "office" && u["state"] == "occupied")
        .expect("an occupied office");
    office["state"] = json!("vacating");
    let x = office["x"].clone();
    let back = import(&build(&save).bytes).save;
    let office = units(&back)
        .iter()
        .find(|u| u["kind"] == "office" && u["x"] == x)
        .expect("the office");
    assert_eq!(
        (&office["state"], &office["everOccupied"]),
        (&json!("occupied"), &json!(true))
    );
}

#[test]
fn a_burned_room_is_written_as_paving_over_its_whole_footprint() {
    // The game draws a burned-area record as garbage, so the debris is cleared.
    let mut save = sample_save();
    let office = units_mut(&mut save)
        .iter_mut()
        .find(|u| u["kind"] == "office" && u["state"] == "occupied")
        .expect("an occupied office");
    office["state"] = json!("gutted");
    let (floor, x, width) = (
        office["floor"].as_i64().unwrap(),
        office["x"].as_i64().unwrap(),
        office["width"].as_i64().unwrap(),
    );
    let built = build(&save);
    assert_eq!(built.counts.burned_out, 1);
    let line = row(&built.bytes, floor + TDT_FLOOR_OFFSET);
    assert!(line.records.iter().all(|r| r.type_id != 48));
    let paving = paved(&line);
    assert!((x..x + width).all(|t| paving.contains(&t)), "{paving:?}");
    let back = import(&built.bytes).save;
    assert!(!units(&back)
        .iter()
        .any(|u| u["kind"] == "office" && u["x"] == x && u["floor"] == floor));
}

/// The sample with `floor` cleared and `extra` placed on it.
fn sample_with(floor: i64, extra: Vec<Value>) -> Value {
    let mut save = sample_save();
    units_mut(&mut save).retain(|u| u["floor"] != floor);
    units_mut(&mut save).extend(extra);
    save
}

#[test]
fn a_row_whose_only_room_burned_is_still_paved_end_to_end() {
    let floor = 7;
    let bytes = build(&sample_with(
        floor,
        vec![unit_in(9_400, "office", floor, 120, 9, "gutted")],
    ))
    .bytes;
    let line = row(&bytes, floor + TDT_FLOOR_OFFSET);
    assert!(line.records.iter().all(|r| r.type_id != 48));
    let paving = paved(&line);
    assert!((120..129).all(|t| paving.contains(&t)));
    assert_eq!(notes(&bytes), Vec::<String>::new());
}

#[test]
fn a_burned_room_past_the_lot_edge_paves_and_claims_only_up_to_the_edge() {
    let floor = 8;
    let bytes = build(&sample_with(
        floor,
        vec![unit_in(9_401, "office", floor, LOT_WIDTH - 4, 9, "gutted")],
    ))
    .bytes;
    let line = row(&bytes, floor + TDT_FLOOR_OFFSET);
    assert!(!line.records.is_empty());
    assert_eq!(line.records.iter().map(|r| r.right).max(), Some(LOT_WIDTH));
    assert_eq!(line.right, LOT_WIDTH);
    assert!(line.right >= line.left);
    assert_eq!(notes(&bytes), Vec::<String>::new());
}

#[test]
fn a_row_never_writes_a_left_edge_smaller_than_the_one_before() {
    let floor = 6;
    let bytes = build(&sample_with(
        floor,
        vec![
            unit_in(9_500, "office", floor, -1, 9, "occupied"),
            unit_in(9_501, "office", floor, 40, 9, "occupied"),
        ],
    ))
    .bytes;
    let lefts: Vec<i64> = row(&bytes, floor + TDT_FLOOR_OFFSET)
        .records
        .iter()
        .map(|r| r.left)
        .collect();
    assert!(lefts.len() > 1);
    assert!(lefts.windows(2).all(|w| w[1] >= w[0]), "{lefts:?}");
    assert_eq!(notes(&bytes), Vec::<String>::new());
}

#[test]
fn a_forged_width_never_puts_a_rows_extent_and_records_out_of_step() {
    // An infinite width reaches the engine as null, the way JSON writes it.
    for (floor, width) in [(9, Value::Null), (8, json!(400))] {
        let mut paving = unit(9_402 + floor, "floor", floor, 0, 1);
        paving["width"] = width;
        let bytes = build(&sample_with(floor, vec![paving])).bytes;
        let line = row(&bytes, floor + TDT_FLOOR_OFFSET);
        assert!(
            line.right <= LOT_WIDTH && line.right >= line.left,
            "floor {floor}"
        );
        for r in &line.records {
            assert!(
                r.left >= line.left && r.right <= line.right,
                "floor {floor}: {r:?}"
            );
        }
        assert_eq!(
            !line.records.is_empty(),
            line.right > line.left,
            "floor {floor}"
        );
        assert_eq!(notes(&bytes), Vec::<String>::new());
    }
}

#[test]
fn a_burned_room_on_the_ground_row_is_paved_as_lobby() {
    let bytes = build(&sample_with(
        1,
        vec![
            unit_in(9_404, "lobby", 1, 100, 1, "occupied"),
            unit_in(9_405, "office", 1, 101, 9, "gutted"),
        ],
    ))
    .bytes;
    let lobby: std::collections::BTreeSet<i64> = row(&bytes, 1 + TDT_FLOOR_OFFSET)
        .records
        .iter()
        .filter(|r| r.type_id == 24)
        .flat_map(|r| r.left..r.right)
        .collect();
    assert!((101..110).all(|t| lobby.contains(&t)));
    assert_eq!(notes(&bytes), Vec::<String>::new());
}

#[test]
fn express_shafts_are_all_written_and_come_after_the_rest() {
    let base = build(&sample_save()).stats.elevators_len;
    let mut save = sample_save();
    for x in [60, 80, 100] {
        transports_mut(&mut save).push(shaft(800 + x, "elevatorExpress", x, 1, 30, 2));
    }
    let built = build(&save);
    assert_eq!(built.stats.elevators_len, base + 3);
    assert_eq!(
        (built.stats.shafts_dropped, built.stats.shafts_colliding),
        (0, 0)
    );

    // The retail game loses every shaft written after an express slot.
    let mut save = sample_save();
    transports_mut(&mut save).push(shaft(360, "elevatorExpress", 60, 1, 30, 2));
    transports_mut(&mut save).push(shaft(380, "elevatorStandard", 80, 1, 12, 2));
    transports_mut(&mut save).push(shaft(390, "elevatorService", 90, 1, 12, 2));
    let order = slots(&build(&save).bytes);
    let at = |x: i64| {
        order
            .iter()
            .position(|&s| s == x)
            .unwrap_or_else(|| panic!("no slot at {x}"))
    };
    assert!(at(80) < at(60) && at(90) < at(60), "{order:?}");
}

#[test]
fn the_24_slot_cap_drops_the_last_built_shafts_and_keeps_the_express() {
    let mut save = sample_save();
    let list = transports_mut(&mut save);
    list.clear();
    list.push(shaft(400, "elevatorExpress", 40, 1, 30, 2));
    for i in 0..30 {
        list.push(shaft(500 + i, "elevatorStandard", 60 + i * 5, 1, 12, 1));
    }
    let order = slots(&build(&save).bytes);
    assert_eq!(order.len(), 24);
    assert_eq!(order.last(), Some(&40));
}

#[test]
fn shafts_past_the_24_slots_are_counted_as_dropped() {
    let mut save = sample_save();
    for i in 0..30 {
        transports_mut(&mut save).push(shaft(20_000 + i, "elevatorStandard", 5 + i * 6, 0, 5, 2));
    }
    let built = build(&save);
    assert!(built.stats.elevators_len >= 24);
    assert!(built.stats.shafts_dropped > 0);
}

#[test]
fn a_burned_two_story_room_leaves_both_rows_paved() {
    let mut save = sample_save();
    units_mut(&mut save).retain(|u| u["floor"] != 11 && u["floor"] != 12);
    units_mut(&mut save).push(unit_in(9_403, "cinema", 11, 100, 31, "gutted"));
    let bytes = build(&save).bytes;
    for floor in [11, 12] {
        let paving = paved(&row(&bytes, floor + TDT_FLOOR_OFFSET));
        assert!((100..131).all(|t| paving.contains(&t)), "floor {floor}");
    }
    assert_eq!(notes(&bytes), Vec::<String>::new());
}

#[test]
fn the_cathedral_over_a_two_story_room_overlaps_nothing() {
    let mut save = sample_save();
    units_mut(&mut save).push(unit(9_200, "cinema", 98, 180, 31));
    units_mut(&mut save).push(unit(9_201, "weddingHall", 100, 180, 16));
    let bytes = build(&save).bytes;
    assert_eq!(notes(&bytes), Vec::<String>::new());
    let back = import(&bytes);
    let count = |kind: &str| {
        units(&back.save)
            .iter()
            .filter(|u| u["kind"] == kind)
            .count()
    };
    assert_eq!((count("cinema"), count("weddingHall")), (1, 1));
    assert!(units(&back.save)
        .iter()
        .any(|u| u["kind"] == "weddingHall" && u["floor"] == 100));
    assert_eq!(back.counts.overlapping, 0);
}

#[test]
fn a_row_of_256_rooms_exports_and_257_is_refused() {
    let row_of = |n: i64| {
        let mut save = sample_save();
        units_mut(&mut save).extend((0..n).map(|i| unit(11_000 + i, "office", 40, i, 1)));
        save
    };
    assert_eq!(notes(&build(&row_of(256)).bytes), Vec::<String>::new());
    assert!(verticopolis_engine::tdt::export::build_tdt(&row_of(257)).is_err());
}

fn first_elevator(save: &mut Value) -> &mut Value {
    transports_mut(save)
        .iter_mut()
        .find(|t| t["kind"] != "stairs" && t["kind"] != "escalator")
        .expect("the sample has an elevator")
}

#[test]
fn a_nine_car_shaft_is_written_as_eight_without_breaking_the_table() {
    let mut save = sample_save();
    first_elevator(&mut save)["cars"] = json!(9);
    assert_eq!(notes(&build(&save).bytes), Vec::<String>::new());
}

#[test]
fn a_shaft_always_stops_at_its_ends() {
    let mut save = sample_save();
    let shaft = first_elevator(&mut save);
    shaft["skipFloors"] = json!([shaft["bottom"]]);
    let (kind, x) = (
        shaft["kind"].as_str().unwrap().to_string(),
        shaft["x"].as_i64().unwrap(),
    );
    let back = import(&build(&save).bytes).save;
    assert!(skips(shaft_at(&back, &kind, x)).is_empty());
}

#[test]
fn a_stall_with_no_ramp_is_not_counted_as_connected() {
    // The sample has a ramp and one stall beside it on B1.
    let mut save = sample_save();
    units_mut(&mut save).push(unit(9_400, "parking", 0, 300, 4));
    let tower = parse_tdt_binary(&build(&save).bytes).expect("reads");
    assert_eq!(tower.tail.parking_connected, Some(1));
}

#[test]
fn a_forged_star_is_clamped_once_for_the_file_and_the_count() {
    for (star, expected) in [(Value::Null, 1), (json!(9.7), 6)] {
        let mut save = sample_save();
        save["star"] = star;
        let built = build(&save);
        assert_eq!(built.stats.star, expected);
        assert_eq!(import(&built.bytes).save["star"], expected);
    }
}
