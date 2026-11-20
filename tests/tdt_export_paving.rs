//! The paving records the writer lays (lobby 24 and empty floor 0, one
//! span per run, never under a room), and what an exported file needs for
//! the 1994 game to load and play it: the people pool and census, the
//! routing tail, our stamp, the opening view and the shafts' schedules.

mod tdt_kit;

use serde_json::{json, Value};
use tdt_kit::*;
use verticopolis_engine::facilities::LOT_WIDTH;
use verticopolis_engine::tdt::format::{
    TDT_DEFAULT_VIEW_X, TDT_DEFAULT_VIEW_Y, TDT_ELEVATOR_HEADER_SIZE, TDT_ELEVATOR_SLOTS,
    TDT_FINANCE_TOTAL_AT, TDT_FLOOR_COUNT, TDT_FLOOR_OFFSET, TDT_PERSON_RECORD_SIZE,
    TDT_RETAIL_RECORD_SIZE, TDT_RETAIL_SLOTS, TDT_ROUTING_TAIL_SIZE, TDT_STAMP_GENERATION,
    TDT_STAMP_MAGIC, TDT_STAMP_SIZE, TDT_STATS_SIZE,
};

fn paving_tower(units: Vec<Value>) -> Value {
    let mut save = tower(units, vec![]);
    save["star"] = json!(2);
    save
}

fn record(left: i64, right: i64, type_id: i64) -> Record {
    // Lobby paving is written with status 0, empty floor with status 2.
    let status = if type_id == 24 { 0 } else { 2 };
    Record {
        left,
        right,
        type_id,
        status,
        rent_class: 4,
        subtype: 0,
    }
}

fn of_type(bytes: &[u8], floor: i64, type_id: i64) -> Vec<Record> {
    row(bytes, floor + TDT_FLOOR_OFFSET)
        .records
        .into_iter()
        .filter(|r| r.type_id == type_id)
        .collect()
}

/// Width-1 tiles of `kind` on `floor` over `from..to`.
fn tiles(id: &mut i64, kind: &str, floor: i64, from: i64, to: i64) -> Vec<Value> {
    (from..to)
        .map(|x| {
            *id += 1;
            unit(*id, kind, floor, x, 1)
        })
        .collect()
}

/// Paving the way an imported save lays it, under rooms too: the ground
/// lobby and a sky lobby on floor 15 over `0..10`, and floor 2 paved over
/// `0..20` with offices on `0..9` and `11..20` and an empty gap between.
fn paved_tower() -> Value {
    let mut id = 0;
    let mut list = tiles(&mut id, "lobby", 1, 0, 10);
    list.extend(tiles(&mut id, "lobby", 15, 0, 10));
    list.extend(tiles(&mut id, "floor", 2, 0, 20));
    list.push(unit_in(100, "office", 2, 0, 9, "occupied"));
    list.push(unit_in(101, "office", 2, 11, 9, "occupied"));
    paving_tower(list)
}

#[test]
fn the_ground_and_sky_lobbies_are_one_lobby_span_each() {
    let bytes = build(&paved_tower()).bytes;
    assert_eq!(
        row(&bytes, 1 + TDT_FLOOR_OFFSET).records,
        vec![record(0, 10, 24)]
    );
    assert_eq!(of_type(&bytes, 15, 24), vec![record(0, 10, 24)]);
}

#[test]
fn an_empty_gap_is_one_floor_span_and_nothing_is_paved_under_a_room() {
    let bytes = build(&paved_tower()).bytes;
    assert_eq!(of_type(&bytes, 2, 0), vec![record(9, 11, 0)]);
    let offices: Vec<(i64, i64)> = of_type(&bytes, 2, 7)
        .iter()
        .map(|r| (r.left, r.right))
        .collect();
    assert_eq!(offices, vec![(0, 9), (11, 20)]);
}

#[test]
fn a_paved_tower_reads_back_quietly_and_writes_the_same_bytes_again() {
    let first = build(&paved_tower()).bytes;
    assert_eq!(notes(&first), Vec::<String>::new());
    assert_eq!(build(&import(&first).save).bytes, first);
}

#[test]
fn a_room_in_the_lobby_splits_it_into_a_span_on_each_side() {
    let mut id = 0;
    let mut list = tiles(&mut id, "lobby", 1, 0, 20);
    list.push(unit(100, "security", 1, 6, 4));
    let bytes = build(&paving_tower(list)).bytes;
    assert_eq!(
        of_type(&bytes, 1, 24),
        vec![record(0, 6, 24), record(10, 20, 24)]
    );
}

#[test]
fn a_burned_lobby_tile_is_paved_as_lobby_and_the_tower_round_trips() {
    // Fire never reaches a lobby in play; only a hand-edited save has one.
    let list: Vec<Value> = (0..10)
        .map(|x| {
            unit_in(
                x + 1,
                "lobby",
                1,
                x,
                1,
                if x == 4 || x == 5 { "gutted" } else { "empty" },
            )
        })
        .collect();
    let first = build(&paving_tower(list)).bytes;
    assert_eq!(of_type(&first, 1, 24), vec![record(0, 10, 24)]);
    assert_eq!(notes(&first), Vec::<String>::new());
    assert_eq!(build(&import(&first).save).bytes, first);
}

#[test]
fn paving_with_holes_is_written_as_the_importer_will_read_it() {
    // The importer paves a row's whole extent as one block, so the writer
    // writes the extent minus the rooms, whatever tiles the save held.
    let mut id = 0;
    let mut list = tiles(&mut id, "floor", 3, 0, 5);
    list.push(unit_in(100, "office", 3, 20, 9, "occupied"));
    list.extend(tiles(&mut id, "floor", 4, 0, 5));
    list.extend(tiles(&mut id, "floor", 4, 10, 15));
    list.extend(tiles(&mut id, "lobby", 15, 0, 8));
    let first = build(&paving_tower(list)).bytes;
    assert_eq!(notes(&first), Vec::<String>::new());
    assert_eq!(build(&import(&first).save).bytes, first);
    assert_eq!(of_type(&first, 3, 0), vec![record(0, 20, 0)]);
    let office: Vec<(i64, i64)> = of_type(&first, 3, 7)
        .iter()
        .map(|r| (r.left, r.right))
        .collect();
    assert_eq!(office, vec![(20, 29)]);
    assert_eq!(of_type(&first, 4, 0), vec![record(0, 15, 0)]);
    assert_eq!(of_type(&first, 15, 24), vec![record(0, 8, 24)]);
}

#[test]
fn a_floor_with_no_width_stays_inside_the_lot() {
    // An infinite width reaches the engine as null, the way JSON writes it.
    let mut wide = unit(2, "floor", 2, 0, 1);
    wide["width"] = Value::Null;
    let bytes = build(&paving_tower(vec![unit(1, "lobby", 1, 0, 1), wide])).bytes;
    for index in 0..TDT_FLOOR_COUNT {
        for r in row(&bytes, index).records {
            assert!(r.left >= 0 && r.right <= LOT_WIDTH, "row {index}: {r:?}");
        }
    }
}

#[test]
fn no_paving_span_overlaps_a_part_of_the_cathedral() {
    let mut id = 0;
    let mut list = vec![];
    for floor in 96..=100 {
        list.extend(tiles(&mut id, "floor", floor, 0, 40));
    }
    list.push(unit(1_000, "weddingHall", 100, 5, 28));
    let bytes = build(&paving_tower(list)).bytes;
    for floor in 100..=104 {
        let records = row(&bytes, floor + TDT_FLOOR_OFFSET).records;
        let parts: Vec<&Record> = records
            .iter()
            .filter(|r| (36..=40).contains(&r.type_id.abs()))
            .collect();
        assert_eq!(parts.len(), 1, "floor {floor}");
        assert_eq!(parts[0].type_id.abs(), 140 - floor, "floor {floor}");
        for span in records.iter().filter(|r| r.type_id == 0 || r.type_id == 24) {
            let p = parts[0];
            assert!(
                !(span.left < p.right && p.left < span.right),
                "floor {floor}: {span:?} over {p:?}"
            );
        }
    }
    assert_eq!(notes(&bytes), Vec::<String>::new());
    assert_eq!(build(&import(&bytes).save).bytes, bytes);
}

/// The people pool keeps a spare block of 256 records past the census,
/// rounded up to whole blocks the way the game's own saves are.
const PEOPLE_BLOCK: u32 = 256;

fn pool(census: u32) -> u32 {
    (census + PEOPLE_BLOCK).div_ceil(PEOPLE_BLOCK) * PEOPLE_BLOCK
}

/// The finance block's population total, for a tower with no built shafts:
/// past the people, the retail table, the empty elevator slots and the
/// statistic words.
fn finance_total(bytes: &[u8]) -> u32 {
    let at = people_at(bytes);
    let finance = at
        + 4
        + u32_at(bytes, at) as usize * TDT_PERSON_RECORD_SIZE
        + TDT_RETAIL_SLOTS * TDT_RETAIL_RECORD_SIZE
        + TDT_ELEVATOR_SLOTS * TDT_ELEVATOR_HEADER_SIZE
        + TDT_STATS_SIZE;
    u32_at(bytes, finance + TDT_FINANCE_TOTAL_AT)
}

/// The finance block counts `census` people and the pool is sized from it.
fn assert_census(bytes: &[u8], census: u32) {
    assert_eq!(finance_total(bytes), census, "the finance total");
    assert_eq!(
        u32_at(bytes, people_at(bytes)),
        pool(census),
        "the people pool"
    );
}

fn lobby_and(rooms: Vec<Value>) -> Vec<u8> {
    let mut list = vec![unit(1, "lobby", 1, 0, 1)];
    list.extend(rooms);
    build(&paving_tower(list)).bytes
}

#[test]
fn an_empty_tower_still_writes_a_spare_block_of_people() {
    let bytes = lobby_and(vec![]);
    assert_census(&bytes, 0);
    assert_eq!(u32_at(&bytes, people_at(&bytes)), PEOPLE_BLOCK);
}

#[test]
fn offices_and_condos_count_their_people_and_the_records_are_zeroed() {
    let bytes = lobby_and(vec![
        unit_in(2, "office", 2, 0, 12, "occupied"),
        unit_in(3, "office", 3, 0, 12, "occupied"),
        unit_in(4, "condo", 4, 0, 16, "occupied"),
        unit_in(5, "office", 5, 0, 12, "empty"),
    ]);
    assert_census(&bytes, 6 + 6 + 3);
    let at = people_at(&bytes);
    let records = &bytes[at + 4..at + 4 + u32_at(&bytes, at) as usize * TDT_PERSON_RECORD_SIZE];
    assert!(records.iter().all(|&b| b == 0));
}

#[test]
fn shops_and_restaurants_count_their_people_and_vacant_ones_count_none() {
    let shops = lobby_and(vec![
        unit_in(2, "shop", 2, 0, 12, "occupied"),
        unit_in(3, "fastFood", 3, 0, 16, "occupied"),
    ]);
    assert_census(&shops, 20 + 25);
    let vacant = lobby_and(vec![
        unit_in(2, "shop", 2, 0, 12, "empty"),
        unit_in(3, "restaurant", 3, 0, 24, "empty"),
    ]);
    assert_census(&vacant, 0);
    let mixed = lobby_and(vec![
        unit_in(2, "office", 2, 0, 12, "occupied"),
        unit_in(3, "fastFood", 3, 0, 16, "occupied"),
        unit_in(4, "restaurant", 4, 0, 24, "occupied"),
        unit_in(5, "shop", 5, 0, 12, "empty"),
    ]);
    assert_census(&mixed, 6 + 25 + 35);
}

#[test]
fn a_census_past_the_spare_block_grows_the_pool_a_whole_block() {
    let offices = |n: i64| {
        lobby_and(
            (0..n)
                .map(|i| unit_in(2 + i, "office", 2 + i, 0, 12, "occupied"))
                .collect(),
        )
    };
    assert_census(&offices(42), 252);
    assert_eq!(pool(252), 2 * PEOPLE_BLOCK);
    assert_census(&offices(43), 258);
    assert_eq!(pool(258), 3 * PEOPLE_BLOCK);
}

#[test]
fn a_condo_with_no_resident_count_does_not_zero_the_census() {
    let mut condo = unit_in(3, "condo", 3, 0, 16, "occupied");
    condo["residents"] = Value::Null;
    let bytes = lobby_and(vec![unit_in(2, "office", 2, 0, 12, "occupied"), condo]);
    assert!(finance_total(&bytes) >= 6);
}

#[test]
fn the_routing_tail_is_ff_except_its_two_empty_tables() {
    let bytes = lobby_and(vec![]);
    let end = bytes.len() - TDT_STAMP_SIZE;
    let tail = &bytes[end - TDT_ROUTING_TAIL_SIZE..end];
    // The stall table's 512 empty records read FF then zeros, and the hall
    // and theatre table's 16 read FE FE then zeros.
    let stalls = 4_012;
    for r in 0..512 {
        assert_eq!(
            &tail[stalls + r * 6..stalls + r * 6 + 6],
            &[0xff, 0, 0, 0, 0, 0],
            "stall {r}"
        );
    }
    let venues = 7_220;
    for r in 0..16 {
        let at = venues + r * 12;
        assert_eq!(
            &tail[at..at + 12],
            &[0xfe, 0xfe, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            "venue {r}"
        );
    }
    let rest = tail[..stalls]
        .iter()
        .chain(&tail[stalls + 512 * 6..venues])
        .chain(&tail[venues + 16 * 12..]);
    assert!(rest.copied().all(|b| b == 0xff));
}

#[test]
fn the_file_ends_with_our_stamp_and_opens_on_the_ground_lobby() {
    let bytes = lobby_and(vec![]);
    let at = bytes.len() - TDT_STAMP_SIZE;
    assert_eq!(
        &bytes[at..at + TDT_STAMP_MAGIC.len()],
        TDT_STAMP_MAGIC.as_bytes()
    );
    assert_eq!(
        u16_at(&bytes, at + TDT_STAMP_MAGIC.len()) as i64,
        TDT_STAMP_GENERATION
    );
    assert_eq!(u16_at(&bytes, 0x26) as i64, TDT_DEFAULT_VIEW_X);
    assert_eq!(u16_at(&bytes, 0x28) as i64, TDT_DEFAULT_VIEW_Y);
}

#[test]
fn a_built_shaft_carries_the_default_schedule_so_its_cars_run() {
    let mut list = vec![unit(1, "lobby", 1, 0, 1)];
    let mut lift = shaft(10, "elevatorStandard", 20, 1, 9, 1);
    lift["width"] = json!(8);
    let bytes = build(&{
        let mut save = paving_tower(list.clone());
        save["transports"] = json!([lift]);
        save
    })
    .bytes;
    let at = people_at(&bytes);
    let slot = at
        + 4
        + u32_at(&bytes, at) as usize * TDT_PERSON_RECORD_SIZE
        + TDT_RETAIL_SLOTS * TDT_RETAIL_RECORD_SIZE;
    assert_eq!(bytes[slot], 1, "the first slot is in use");
    let mut schedule = vec![0x01; 14];
    schedule.extend([0x05; 14]);
    schedule.extend([0x00; 28]);
    assert_eq!(&bytes[slot + 4..slot + 4 + 56], &schedule[..]);
    // With an office as well, the file still reads back with no notes.
    list.push(unit_in(2, "office", 2, 0, 12, "occupied"));
    let mut save = paving_tower(list);
    save["transports"] = json!([shaft(10, "elevatorStandard", 20, 1, 9, 1)]);
    assert_eq!(notes(&build(&save).bytes), Vec::<String>::new());
}
