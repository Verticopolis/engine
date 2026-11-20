//! What the `.TDT` importer makes of a file's header and floor map: the
//! refusals, the money, clock and star, where each tenant type lands, how
//! multi-story parts merge, and the rooms a 1994 file can hold that the
//! engine cannot. Files are built from a spec (`tdt_kit::builder`).

mod tdt_kit;

use serde_json::Value;
use tdt_kit::builder::{build, floor, sample, tenant, Floor, Spec, Tenant};
use tdt_kit::{case_bytes, units};
use verticopolis_engine::tdt::format::{parse_tdt_binary, TDT_HEADER_SIZE, TDT_MAX_FILE_BYTES};
use verticopolis_engine::tdt::parse::{parse_tdt, ParsedLegacyTower};

fn parse(spec: &Spec) -> ParsedLegacyTower {
    parse_tdt(&build(spec), "TOWER.TDT").unwrap_or_else(|e| panic!("refused: {}", e.0))
}

fn refusal(spec: &Spec) -> String {
    match parse_tdt(&build(spec), "TOWER.TDT") {
        Ok(_) => panic!("the file was read"),
        Err(e) => e.0,
    }
}

fn with_floors(floors: Vec<Floor>) -> Spec {
    Spec {
        floors,
        ..Spec::default()
    }
}

fn of_kind<'a>(save: &'a Value, kind: &str) -> Vec<&'a Value> {
    units(save).iter().filter(|u| u["kind"] == kind).collect()
}

fn rooms(save: &Value) -> Vec<&Value> {
    units(save)
        .iter()
        .filter(|u| u["kind"] != "floor" && u["kind"] != "lobby")
        .collect()
}

fn notes(parsed: &ParsedLegacyTower) -> String {
    [&parsed.header_notes[..], &parsed.warnings[..]]
        .concat()
        .join(" ")
}

fn i(v: &Value) -> i64 {
    v.as_i64().unwrap_or_else(|| panic!("not an integer: {v}"))
}

/// Floors of a tenant's type on one row, `.TDT` index `index`.
fn one(index: i64, left: i64, right: i64, type_id: i64) -> Spec {
    with_floors(vec![floor(index, vec![tenant(left, right, type_id)])])
}

#[test]
fn the_builder_writes_the_locks_sample_byte_for_byte() {
    // The lock's sample was written before the statistics, the two undecoded
    // blocks and the clinic list joined the layout, so it is the same tower
    // with the generation-one blocks. A builder that drifted would test a
    // file no case holds.
    let spec = Spec {
        generation_one_blocks: true,
        ..sample()
    };
    let built = build(&spec);
    let lock = case_bytes("sample");
    let first = built.iter().zip(&lock).position(|(a, b)| a != b);
    assert_eq!(
        (built.len(), first),
        (lock.len(), None),
        "built and lock differ (length, first differing byte)"
    );
}

#[test]
fn a_wrong_magic_a_tiny_file_and_a_huge_one_are_refused_with_a_reason() {
    assert!(refusal(&Spec {
        magic: 0x1234,
        ..Spec::default()
    })
    .contains("doesn't look like a SimTower save"));
    let tiny = parse_tdt(&[0; 8], "TOWER.TDT").err().expect("refused").0;
    assert!(tiny.contains("too small"), "{tiny}");
    let huge = parse_tdt_binary(&vec![0; TDT_MAX_FILE_BYTES + 1])
        .err()
        .expect("refused")
        .0;
    assert!(huge.contains("too large"), "{huge}");
}

#[test]
fn a_file_cut_inside_the_floor_map_says_it_was_cut_short() {
    let cut = Spec {
        truncate_at: Some(TDT_HEADER_SIZE + 194 * 3 + 10),
        ..sample()
    };
    assert!(refusal(&cut).contains("cut short"));
}

#[test]
fn an_absurd_tenant_count_is_refused_before_any_loop() {
    let forged = |index, count| Floor {
        forge_count: Some(count),
        ..floor(index, vec![])
    };
    assert!(refusal(&with_floors(vec![forged(10, 257)])).contains("corrupt"));
    // A count under the cap that runs past the file's end is a cut file.
    let past_end = Spec {
        floors: vec![forged(119, 250)],
        include_retail: false,
        ..Spec::default()
    };
    assert!(refusal(&past_end).contains("cut short"));
}

#[test]
fn an_absurd_people_count_is_a_note_and_a_missing_retail_table_leaves_transports_unread() {
    let mut bytes = build(&Spec {
        include_retail: false,
        ..Spec::default()
    });
    let end = bytes.len();
    bytes[end - 4..].copy_from_slice(&[0xff; 4]);
    let tail = parse_tdt_binary(&bytes).expect("reads").tail;
    assert_eq!(tail.people_count, None);
    assert!(
        tail.warnings.join(" ").contains("head count"),
        "{:?}",
        tail.warnings
    );

    let tail = parse_tdt_binary(&build(&Spec {
        include_retail: false,
        ..Spec::default()
    }))
    .expect("reads")
    .tail;
    assert!(tail.elevators.is_none() && tail.stairs.is_none());
    assert!(!tail.warnings.is_empty());

    let tail = parse_tdt_binary(&build(&Spec {
        retail_rows: 7,
        ..Spec::default()
    }))
    .expect("reads")
    .tail;
    assert_eq!(tail.warnings, Vec::<String>::new());
}

#[test]
fn money_is_stored_in_hundreds_and_negative_funds_survive() {
    assert_eq!(
        parse(&Spec {
            balance: 20_000,
            ..Spec::default()
        })
        .save["money"],
        2_000_000
    );
    assert_eq!(
        parse(&Spec {
            balance: -500,
            ..Spec::default()
        })
        .save["money"],
        -50_000
    );
}

fn minutes(frame_time: u16, current_day: i32) -> i64 {
    i(&parse(&Spec {
        frame_time,
        current_day,
        ..Spec::default()
    })
    .save["minutes"])
}

#[test]
fn the_clock_starts_at_7_am_and_the_small_hours_belong_to_the_same_day() {
    assert_eq!(minutes(0, 0), 7 * 60);
    assert_eq!(minutes(0, 2), 2 * 1440 + 7 * 60);
    // The game turns the date at tick 2300, so that is midnight of the same day.
    assert_eq!(minutes(2300, 5), 5 * 1440);
    let last = minutes(2599, 5);
    assert!(last > 5 * 1440 && last < 5 * 1440 + 7 * 60, "{last}");
}

#[test]
fn out_of_range_header_fields_are_clamped_with_a_note() {
    let negative = parse(&Spec {
        current_day: -12,
        ..Spec::default()
    });
    assert_eq!(negative.save["minutes"], 7 * 60);
    assert!(
        notes(&negative).contains("day counter was negative"),
        "{}",
        notes(&negative)
    );
    let clock = parse(&Spec {
        frame_time: 60_000,
        ..Spec::default()
    });
    assert!(clock.save["minutes"].is_i64());
    assert!(
        notes(&clock).contains("clock was out of range"),
        "{}",
        notes(&clock)
    );
    let star = parse(&Spec {
        level: 0,
        ..Spec::default()
    });
    assert!(notes(&star).contains("star rating (0)"), "{}", notes(&star));
    let fine = parse(&Spec {
        frame_time: 2599,
        level: 3,
        current_day: 4,
        ..Spec::default()
    });
    assert!(!notes(&fine).contains("out of range") && !notes(&fine).contains("was negative"));
}

#[test]
fn rows_map_to_floors_and_the_reserved_rows_are_dropped() {
    let floor_of =
        |index, type_id| i(&rooms(&parse(&one(index, 100, 109, type_id)).save)[0]["floor"]);
    assert_eq!(floor_of(0, 10), -9);
    assert_eq!(floor_of(9, 10), 0);
    assert_eq!(floor_of(109, 7), 100);
    let ground = parse(&one(10, 100, 109, 24));
    assert!(units(&ground.save)
        .iter()
        .any(|u| u["kind"] == "lobby" && u["floor"] == 1));
    let dropped = parse(&one(110, 100, 109, 7));
    assert!(units(&dropped.save).is_empty());
    assert_eq!(dropped.counts.dropped_floors, 1);
}

/// One tenant of `type_id` where its kind may stand: basement kinds on B4,
/// the cathedral's parts on floor 100, anything else on floor 11.
fn placed(type_id: i64) -> Spec {
    let index = match type_id.abs() {
        11 | 44 | 20 | 21 | 31 | 32 | 33 => 6,
        36..=40 => 109,
        _ => 20,
    };
    one(index, 100, 109, type_id)
}

#[test]
fn each_tenant_type_imports_as_its_kind() {
    for (type_id, kind) in [
        (3, "hotelSingle"),
        (4, "hotelDouble"),
        (5, "hotelSuite"),
        (6, "restaurant"),
        (7, "office"),
        (9, "condo"),
        (10, "shop"),
        (11, "parking"),
        (12, "fastFood"),
        (13, "medical"),
        (14, "security"),
        (15, "housekeeping"),
        (17, "security"),
        (44, "parkingRamp"),
        // The parts of a multi-story building merge into one.
        (18, "cinema"),
        (19, "cinema"),
        (34, "cinema"),
        (35, "cinema"),
        (20, "recycling"),
        (21, "recycling"),
        (29, "partyHall"),
        (30, "partyHall"),
        (31, "metro"),
        (32, "metro"),
        (33, "metro"),
        (36, "weddingHall"),
        (40, "weddingHall"),
    ] {
        let save = parse(&placed(type_id)).save;
        let got = rooms(&save);
        assert_eq!(got.len(), 1, "type {type_id}");
        assert_eq!(got[0]["kind"], kind, "type {type_id}");
    }
}

#[test]
fn a_metro_tunnel_alone_is_paving() {
    let save = parse(&placed(45)).save;
    assert!(units(&save).iter().all(|u| u["kind"] == "floor"));
}

fn style(type_id: i64, variant: u8, byte17: u8) -> Value {
    let spec = with_floors(vec![floor(
        20,
        vec![Tenant {
            variant,
            byte17,
            ..tenant(100, 109, type_id)
        }],
    )]);
    rooms(&parse(&spec).save)[0]["subtype"].clone()
}

#[test]
fn a_shops_style_is_read_from_byte_6_and_an_unknown_one_takes_the_first() {
    use verticopolis_engine::facilities::Kind;
    let list = |kind: Kind| kind.subtype_list().expect("styles");
    assert_eq!(style(10, 3, 0), list(Kind::Shop)[3]);
    assert_eq!(style(12, 1, 0), list(Kind::FastFood)[1]);
    assert_eq!(style(6, 4, 0), list(Kind::Restaurant)[4]);
    assert_eq!(style(10, 200, 0), list(Kind::Shop)[0]);
    // Byte 17 is a decoy the game leaves 0; reading it would give style 1.
    assert_eq!(style(10, 3, 1), list(Kind::Shop)[3]);
}

#[test]
fn a_theatres_four_pieces_merge_into_one_cinema_on_its_bottom_story() {
    let save = parse(&with_floors(vec![
        floor(20, vec![tenant(100, 107, 35), tenant(107, 131, 19)]),
        floor(21, vec![tenant(100, 107, 34), tenant(107, 131, 18)]),
    ]))
    .save;
    let cinemas = of_kind(&save, "cinema");
    assert_eq!(cinemas.len(), 1);
    assert_eq!(
        (
            i(&cinemas[0]["floor"]),
            i(&cinemas[0]["x"]),
            i(&cinemas[0]["width"])
        ),
        (11, 100, 31)
    );
}

fn cathedral(bottom_index: i64) -> Vec<Floor> {
    (0..5)
        .map(|k| floor(bottom_index + k, vec![tenant(180, 208, 40 - k)]))
        .collect()
}

#[test]
fn the_cathedrals_five_parts_merge_into_one_wedding_hall_on_floor_100() {
    let parsed = parse(&with_floors(cathedral(109)));
    let halls = of_kind(&parsed.save, "weddingHall");
    assert_eq!(halls.len(), 1);
    assert_eq!((i(&halls[0]["floor"]), i(&halls[0]["width"])), (100, 28));
    assert_eq!(parsed.save["builtWeddingHall"], true);
    assert_eq!(parsed.counts.cathedral, 1);
}

fn recycling_floors(save: &Value) -> Vec<(i64, i64)> {
    let mut out: Vec<(i64, i64)> = of_kind(save, "recycling")
        .iter()
        .map(|u| (i(&u["x"]), i(&u["floor"])))
        .collect();
    out.sort();
    out
}

#[test]
fn buildings_of_one_kind_merge_only_with_their_own_parts() {
    // Far apart, stacked on adjacent pairs, flush on one floor, and flush
    // pairs chained through a building below all stay separate.
    let pair = |bottom: i64, x: i64| {
        vec![
            floor(bottom, vec![tenant(x, x + 20, 21)]),
            floor(bottom + 1, vec![tenant(x, x + 20, 20)]),
        ]
    };
    let far = parse(&with_floors([pair(3, 100), pair(6, 100)].concat())).save;
    assert_eq!(recycling_floors(&far), vec![(100, -6), (100, -3)]);
    let stacked = parse(&with_floors([pair(3, 100), pair(5, 100)].concat())).save;
    assert_eq!(recycling_floors(&stacked), vec![(100, -6), (100, -4)]);
    let flush = parse(&with_floors(vec![floor(
        5,
        vec![tenant(100, 120, 21), tenant(120, 140, 21)],
    )]))
    .save;
    assert_eq!(of_kind(&flush, "recycling").len(), 2);
    let mut xs: Vec<i64> = of_kind(&flush, "recycling")
        .iter()
        .map(|u| i(&u["x"]))
        .collect();
    xs.sort();
    assert_eq!(xs, vec![100, 120]);
    let chained = parse(&with_floors(vec![
        floor(6, vec![tenant(107, 132, 21)]),
        floor(7, vec![tenant(107, 132, 20)]),
        floor(8, vec![tenant(92, 117, 21), tenant(117, 142, 21)]),
        floor(9, vec![tenant(92, 117, 20), tenant(117, 142, 20)]),
    ]))
    .save;
    assert_eq!(
        recycling_floors(&chained),
        vec![(92, -1), (107, -3), (117, -1)]
    );
    let width = verticopolis_engine::facilities::Kind::Recycling
        .facility()
        .width;
    assert!(of_kind(&chained, "recycling")
        .iter()
        .all(|u| u["width"] == width));
}

#[test]
fn a_merged_building_claims_every_story_it_stands_on() {
    let parsed = parse(&with_floors(vec![
        floor(20, vec![tenant(100, 131, 19)]),
        floor(21, vec![tenant(100, 109, 7)]),
    ]));
    assert_eq!(of_kind(&parsed.save, "office").len(), 1);
    assert_eq!(of_kind(&parsed.save, "cinema").len(), 0);
    assert_eq!(parsed.counts.overlapping, 1);
    // A building whose upper story would pass floor 100 is dropped, and the
    // room below it stays.
    let top = parse(&with_floors(vec![
        floor(109, vec![tenant(100, 131, 19)]),
        floor(108, vec![tenant(100, 109, 7)]),
    ]));
    assert_eq!(
        (
            of_kind(&top.save, "cinema").len(),
            of_kind(&top.save, "office").len()
        ),
        (0, 1)
    );
}

#[test]
fn a_room_on_a_floor_its_kind_cannot_use_is_dropped_and_counted() {
    let parking = parse(&with_floors(vec![floor(
        20,
        vec![tenant(100, 116, 44), tenant(116, 120, 11)],
    )]));
    assert!(
        of_kind(&parking.save, "parking").is_empty()
            && of_kind(&parking.save, "parkingRamp").is_empty()
    );
    assert!(parking.counts.misplaced >= 1);
    let recycling = parse(&with_floors(vec![
        floor(20, vec![tenant(100, 120, 21)]),
        floor(21, vec![tenant(100, 120, 20)]),
    ]));
    assert!(of_kind(&recycling.save, "recycling").is_empty());
    let office = parse(&one(6, 100, 109, 7));
    assert!(of_kind(&office.save, "office").is_empty());
    let concourse = parse(&one(10, 100, 116, 12));
    assert!(of_kind(&concourse.save, "fastFood").is_empty());
    assert!(concourse.counts.misplaced >= 1);
    // The cathedral stands on floor 100 only.
    let low = parse(&with_floors(cathedral(100)));
    assert!(of_kind(&low.save, "weddingHall").is_empty());
    assert_eq!(
        (&low.save["builtWeddingHall"], &low.save["vipVisitDay"]),
        (&Value::Bool(false), &Value::from(-1))
    );
    assert!(low.counts.misplaced >= 1);
}

#[test]
fn basement_tiles_are_marked_excavated_and_nothing_above_ground_is() {
    let save = parse(&with_floors(vec![
        floor(9, vec![tenant(100, 104, 11)]),
        floor(20, vec![tenant(100, 109, 7)]),
    ]))
    .save;
    let dug: Vec<&str> = save["excavated"]
        .as_array()
        .expect("excavated")
        .iter()
        .map(|k| k.as_str().unwrap())
        .collect();
    assert!(dug.contains(&"0:100") && dug.contains(&"0:103"), "{dug:?}");
    assert!(dug
        .iter()
        .all(|k| k.split(':').next().unwrap().parse::<i64>().unwrap() <= 0));
}

#[test]
fn a_cathedral_below_tower_rating_books_the_vip_three_days_on() {
    let spec = Spec {
        level: 5,
        current_day: 10,
        floors: cathedral(109),
        ..Spec::default()
    };
    let below = parse(&spec).save;
    assert_eq!(
        (&below["builtWeddingHall"], &below["vipVisitDay"]),
        (&Value::Bool(true), &Value::from(13))
    );
    let tower = parse(&Spec { level: 6, ..spec }).save;
    assert_eq!(
        (&tower["vipVisitDay"], &tower["evaluatedTower"]),
        (&Value::from(-1), &Value::Bool(true))
    );
    assert_eq!(
        parse(&Spec {
            level: 5,
            ..Spec::default()
        })
        .save["vipVisitDay"],
        -1
    );
}

#[test]
fn a_metro_stations_three_stories_merge_at_the_bottom() {
    let save = parse(&with_floors(vec![
        floor(0, vec![tenant(0, 375, 33)]),
        floor(1, vec![tenant(0, 375, 32)]),
        floor(2, vec![tenant(0, 375, 31)]),
    ]))
    .save;
    let metros = of_kind(&save, "metro");
    assert_eq!(metros.len(), 1);
    assert_eq!((i(&metros[0]["floor"]), i(&metros[0]["width"])), (-9, 375));
}

#[test]
fn paving_records_and_built_extents_become_tiles_of_floor_or_lobby() {
    let pave = parse(&one(15, 100, 120, 0)).save;
    assert_eq!(units(&pave).len(), 20);
    assert!(units(&pave).iter().all(|u| u["kind"] == "floor"));
    let ground = parse(&one(10, 100, 120, 24)).save;
    assert!(units(&ground).iter().all(|u| u["kind"] == "lobby"));
    let extent = |index| Floor {
        left_edge: Some(100),
        right_edge: Some(110),
        ..floor(index, vec![])
    };
    assert!(units(&parse(&with_floors(vec![extent(24)])).save)
        .iter()
        .all(|u| u["kind"] == "lobby"));
    assert!(units(&parse(&with_floors(vec![extent(20)])).save)
        .iter()
        .all(|u| u["kind"] == "floor"));
}

#[test]
fn a_burned_area_reads_as_bare_floor_and_is_counted() {
    let parsed = parse(&one(20, 100, 110, 48));
    assert!(units(&parsed.save).iter().all(|u| u["kind"] == "floor"));
    assert_eq!(parsed.counts.burned, 1);
}
