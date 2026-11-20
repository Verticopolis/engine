//! What the `.TDT` importer makes of the elevator and stairs tables: the
//! shafts and flights it decodes, the payload layouts it tells apart (the
//! game's, ours from older builds, and the stamped one), the damaged files
//! it gives up on rather than read wrongly, and a decoded tower running in
//! the engine.

mod tdt_kit;

use serde_json::Value;
use tdt_kit::builder::{build, elevator, floor, sample, tenant, Elevator, Layout, Spec, Stair};
use tdt_kit::reading::{i, of_kind, parse, with_floors};
use tdt_kit::{transports, units};
use verticopolis_engine::facilities::{Kind, LOT_WIDTH};
use verticopolis_engine::load::deserialize;
use verticopolis_engine::tdt::format::TDT_HEADER_SIZE;
use verticopolis_engine::tdt::parse::parse_tdt;

/// What the walk read, the way the game's import record tells it.
struct Walk {
    /// The header's notes, then the walk's warnings.
    notes: Vec<String>,
    /// `(x, cars, stops)` of each shaft in table order, or `None` when the
    /// walk gave up and the importer rebuilt the shafts from the floors.
    shafts: Option<Vec<(i64, i64, i64)>>,
    flights: Option<usize>,
    parking: Option<i64>,
}

fn try_walk(bytes: &[u8]) -> Option<Walk> {
    let parsed = parse_tdt(bytes, "TOWER.TDT").ok()?;
    let save = &parsed.save;
    let kind = |t: &Value| t["kind"].as_str().unwrap_or("").to_string();
    let is_flight = |t: &Value| matches!(kind(t).as_str(), "stairs" | "escalator");
    let shafts = transports(save)
        .iter()
        .filter(|t| kind(t).starts_with("elevator"))
        .map(|t| {
            let skips = t["skipFloors"].as_array().map_or(0, Vec::len) as i64;
            (
                i(&t["x"]),
                i(&t["cars"]),
                i(&t["top"]) - i(&t["bottom"]) + 1 - skips,
            )
        })
        .collect();
    let flights = transports(save).iter().filter(|t| is_flight(t)).count();
    let tail = &parsed.tower.tail;
    Some(Walk {
        notes: [&parsed.header_notes[..], &tail.warnings[..]].concat(),
        shafts: parsed.decoded.then_some(shafts),
        flights: parsed.decoded.then_some(flights),
        parking: tail.parking_connected,
    })
}

fn walk(bytes: &[u8]) -> Walk {
    try_walk(bytes).expect("the file is read")
}

fn xs(walk: &Walk) -> Vec<i64> {
    let shafts = walk.shafts.as_ref().expect("the table was decoded");
    shafts.iter().map(|s| s.0).collect()
}

fn serving(e: Elevator, serviced: &[i64]) -> Elevator {
    Elevator {
        serviced: Some(serviced.to_vec()),
        ..e
    }
}

fn padded(e: Elevator, serviced: &[i64], payload_floors: i64) -> Elevator {
    Elevator {
        payload_floors: Some(payload_floors),
        ..serving(e, serviced)
    }
}

fn stair(type_id: u8, x: i64, floor: i64) -> Stair {
    Stair { type_id, x, floor }
}

fn shafts(list: Vec<Elevator>) -> Spec {
    Spec {
        elevators: list.into_iter().map(Some).collect(),
        ..Spec::default()
    }
}

/// Our trailer: the magic and a u16 generation.
fn stamped(mut bytes: Vec<u8>, generation: u16) -> Vec<u8> {
    bytes.extend(b"VCTDT");
    bytes.extend(generation.to_le_bytes());
    bytes
}

const SKY_STOPS: [i64; 8] = [10, 25, 40, 55, 70, 85, 99, 100];

/// The skipping standard shaft our 2.9.0 and earlier sized by its stops.
fn older_build_pair() -> Spec {
    Spec {
        routing_tail: 20_000,
        layout: Layout::LegacyServiced,
        ..shafts(vec![
            serving(elevator(1, 2, 100, 10, 21), &[10, 11, 12, 18, 19, 20, 21]),
            elevator(2, 3, 140, 10, 18),
        ])
    }
}

/// One standard shaft stopping only at its ends, a stair and a parking count.
fn end_stopper(layout: Layout) -> Spec {
    Spec {
        stairs: vec![stair(1, 120, 10)],
        parking_connected: 7,
        layout,
        ..shafts(vec![serving(elevator(1, 2, 100, 10, 40), &[10, 40])])
    }
}

#[test]
fn a_standard_shaft_decodes_its_position_span_cars_and_car_homes() {
    let parsed = parse(&Spec {
        floors: vec![floor_edges(10, 100, 200)],
        ..shafts(vec![Elevator {
            car_homes: Some(vec![10, 24, 39]),
            ..elevator(1, 3, 150, 9, 39)
        }])
    });
    let list = transports(&parsed.save);
    assert_eq!(list.len(), 1);
    let t = &list[0];
    assert_eq!(t["kind"], "elevatorStandard");
    assert_eq!(i(&t["x"]), 150);
    // The game's shaft is four tiles; a narrower one would leave a gap.
    assert_eq!(i(&t["width"]), Kind::ElevatorStandard.facility().width);
    assert_eq!(Kind::ElevatorStandard.facility().width, 4);
    assert_eq!((i(&t["bottom"]), i(&t["top"]), i(&t["cars"])), (0, 30, 3));
    assert_eq!(t["carPositions"], serde_json::json!([1, 15, 30]));
    assert_eq!(t["skipFloors"], serde_json::json!([]));
}

fn floor_edges(index: i64, left: i64, right: i64) -> tdt_kit::builder::Floor {
    tdt_kit::builder::Floor {
        left_edge: Some(left),
        right_edge: Some(right),
        ..floor(index, vec![])
    }
}

#[test]
fn the_serviced_floor_map_becomes_the_shafts_skipped_floors() {
    let parsed = parse(&Spec {
        floors: vec![floor_edges(10, 100, 200)],
        ..shafts(vec![serving(
            elevator(0, 8, 150, 10, 69),
            &[10, 24, 39, 54, 69],
        )])
    });
    let t = &transports(&parsed.save)[0];
    assert_eq!(t["kind"], "elevatorExpress");
    assert_eq!((i(&t["bottom"]), i(&t["top"])), (1, 60));
    let skips: Vec<i64> = t["skipFloors"].as_array().unwrap().iter().map(i).collect();
    assert!(skips.contains(&2));
    for lobby in [15, 30, 45] {
        assert!(!skips.contains(&lobby), "the sky lobby on {lobby} stops");
    }
}

#[test]
fn a_file_from_our_older_builds_still_imports_every_shaft() {
    // Those builds sized a payload by the floors a shaft stops at; the routing
    // tail every export since 1.14.0 ends with is what corroborates it.
    let spec = older_build_pair();
    let walked = walk(&build(&spec));
    assert_eq!(xs(&walked), vec![100, 140]);
    assert_eq!(walked.shafts.as_ref().unwrap()[1].1, 3);
    assert!(walked.notes.is_empty(), "{:?}", walked.notes);
    let save = parse(&spec).save;
    let skipper = transports(&save).iter().find(|t| t["x"] == 100).unwrap();
    assert_eq!(skipper["skipFloors"], serde_json::json!([4, 5, 6, 7, 8]));
}

#[test]
fn a_game_written_express_keeps_the_shafts_stairs_and_parking_after_it() {
    // The game sizes an express by its eligible floors; the skipping standard
    // shaft makes the file readable under exactly one rule.
    let walked = walk(&build(&Spec {
        stairs: vec![stair(1, 120, 10)],
        parking_connected: 302,
        routing_tail: 20_000,
        ..shafts(vec![
            serving(elevator(0, 8, 175, 10, 100), &SKY_STOPS),
            elevator(1, 6, 145, 10, 24),
            serving(elevator(1, 4, 251, 24, 39), &[24, 25, 30, 39]),
            elevator(2, 3, 209, 39, 54),
        ])
    }));
    assert!(walked.notes.is_empty(), "{:?}", walked.notes);
    assert_eq!(xs(&walked), vec![175, 145, 251, 209]);
    assert_eq!(walked.flights, Some(1));
    assert_eq!(walked.parking, Some(302));
    assert_eq!(walked.shafts.as_ref().unwrap()[0].2, SKY_STOPS.len() as i64);
}

fn odd_express_tower(elevators: Vec<Option<Elevator>>, stairs: Vec<Stair>) -> Spec {
    Spec {
        elevators,
        stairs,
        parking_connected: 302,
        routing_tail: 20_000,
        ff_payload: true,
        ..Spec::default()
    }
}

#[test]
fn shafts_after_an_express_of_no_known_size_are_found_by_shape_with_a_note() {
    // A real 1997 save held 16 entries for 11 and 12 stops; no layout predicts it.
    let walked = walk(&build(&odd_express_tower(
        vec![
            Some(padded(elevator(0, 8, 175, 10, 100), &SKY_STOPS, 16)),
            Some(elevator(1, 6, 145, 10, 24)),
            Some(padded(elevator(0, 8, 321, 10, 100), &[10, 40, 70, 100], 16)),
            Some(elevator(2, 3, 209, 39, 54)),
        ],
        vec![stair(1, 120, 10)],
    )));
    assert_eq!(walked.notes.len(), 1, "{:?}", walked.notes);
    assert!(walked.notes[0].contains("found by searching the file"));
    assert_eq!(xs(&walked), vec![175, 145, 321, 209]);
    assert_eq!(walked.flights, Some(1));
    assert_eq!(walked.parking, Some(302));
}

#[test]
fn empty_slots_the_search_passes_over_are_counted_so_parking_is_still_found() {
    let walked = walk(&build(&odd_express_tower(
        vec![
            Some(padded(elevator(0, 8, 175, 10, 100), &SKY_STOPS, 16)),
            None,
            Some(elevator(1, 6, 145, 10, 24)),
            None,
            Some(elevator(2, 3, 209, 39, 54)),
        ],
        vec![],
    )));
    assert_eq!(xs(&walked), vec![175, 145, 209]);
    assert_eq!(walked.parking, Some(302));
}

#[test]
fn a_header_planted_inside_an_odd_express_is_not_taken_for_the_next_shaft() {
    let bytes = build(&odd_express_tower(
        vec![
            Some(padded(elevator(0, 8, 175, 10, 100), &[10, 55, 100], 16)),
            Some(elevator(1, 6, 145, 10, 24)),
        ],
        vec![],
    ));
    assert_eq!(xs(&walk(&bytes)), vec![175, 145]);
    // Copy the standard shaft's header 7 bytes past the express's smallest
    // size, at x = 500: the distance to it fits no whole number of floors.
    let first = TDT_HEADER_SIZE + 120 * (6 + 188) + 4 + 512 * 18;
    let second = first + 194 + 3140 + 348 + 324 * 16;
    let planted = first + 194 + 3812 + 7;
    let mut forged = bytes.clone();
    forged[planted..planted + 194].copy_from_slice(&bytes[second..second + 194]);
    forged[planted + 62..planted + 64].copy_from_slice(&500u16.to_le_bytes());
    let walked = walk(&forged);
    assert_eq!(xs(&walked), vec![175, 145]);
    assert_eq!(walked.parking, Some(302));
}

#[test]
fn an_odd_express_with_no_header_behind_it_is_refused_not_guessed() {
    let walked = walk(&build(&odd_express_tower(
        vec![Some(padded(elevator(0, 8, 175, 10, 100), &SKY_STOPS, 16))],
        vec![stair(1, 120, 10)],
    )));
    assert!(walked.shafts.is_none());
    assert_eq!(walked.notes.len(), 1, "{:?}", walked.notes);
    assert!(walked.notes[0].contains("doesn't match the documented layout"));
}

/// Our own span-sized express, then a standard shaft.
fn spanned_pair(serviced: &[i64]) -> Spec {
    Spec {
        parking_connected: 7,
        layout: Layout::SpannedExpress,
        ..shafts(vec![
            serving(elevator(0, 8, 175, 10, 100), serviced),
            elevator(1, 6, 145, 10, 24),
        ])
    }
}

#[test]
fn our_span_sized_express_reads_stamped_or_not() {
    // With nothing to anchor on, the stamp states the layout and the fallback
    // lands on the longest walk that completes, which is the same answer.
    let spec = spanned_pair(&[10, 55, 100]);
    let generation_one = build(&Spec {
        generation_one_blocks: true,
        ..spec.clone()
    });
    for (label, bytes) in [
        ("stamped", stamped(generation_one, 1)),
        ("unstamped", build(&spec)),
    ] {
        let walked = walk(&bytes);
        assert!(walked.notes.is_empty(), "{label}: {:?}", walked.notes);
        assert_eq!(xs(&walked), vec![175, 145], "{label}");
        assert_eq!(walked.parking, Some(7), "{label}");
    }
}

#[test]
fn an_unstamped_span_sized_express_is_read_on_the_files_own_evidence() {
    for (stairs, tail) in [
        (vec![stair(1, 120, 10)], 20_000),
        (vec![], 0),
        (vec![], 32),
        (vec![], 63),
    ] {
        let walked = walk(&build(&Spec {
            stairs,
            routing_tail: tail,
            ..spanned_pair(&[10, 55, 100])
        }));
        assert_eq!(xs(&walked), vec![175, 145], "tail {tail}");
        assert_eq!(walked.parking, Some(7), "tail {tail}");
        assert!(walked.notes.is_empty(), "tail {tail}: {:?}", walked.notes);
    }
}

#[test]
fn an_express_with_no_stops_recorded_keeps_what_follows_it() {
    let walked = walk(&build(&Spec {
        stairs: vec![stair(1, 120, 10)],
        routing_tail: 20_000,
        ..spanned_pair(&[])
    }));
    assert_eq!(xs(&walked), vec![175, 145]);
    assert_eq!(walked.parking, Some(7));
    assert_eq!(walked.flights, Some(1));
    assert!(walked.notes.is_empty(), "{:?}", walked.notes);
}

#[test]
fn a_stamp_from_an_unknown_generation_does_not_override_the_structural_read() {
    let base = build(&older_build_pair());
    for generation in [4, 7, 0, 0xffff] {
        let walked = walk(&stamped(base.clone(), generation));
        assert_eq!(xs(&walked), vec![100, 140], "generation {generation}");
        assert!(walked.notes.is_empty(), "generation {generation}");
    }
}

fn gave_up(walked: &Walk) {
    assert!(walked.shafts.is_none(), "the table was read");
    let notes = walked.notes.join(" ").to_lowercase();
    assert!(notes.contains("elevator"), "{notes}");
}

#[test]
fn a_truncated_current_file_is_refused_not_read_as_legacy() {
    let full = build(&end_stopper(Layout::Game));
    gave_up(&walk(&full[..full.len() - 2_000]));
}

#[test]
fn no_truncation_of_a_current_file_yields_a_confident_wrong_parking_count() {
    // Every byte over the last 6,000, and a prime stride before that.
    let full = build(&end_stopper(Layout::Game));
    let tail_from = 3_000.max(full.len() - 6_000);
    let lengths = (3_000..tail_from).step_by(97).chain(tail_from..full.len());
    for len in lengths {
        let Some(walked) = try_walk(&full[..len]) else {
            continue;
        };
        if walked.shafts.is_none() || !walked.notes.is_empty() {
            continue;
        }
        assert_eq!(walked.parking, Some(7), "truncated to {len}");
    }
}

#[test]
fn a_stairless_legacy_tower_keeps_its_parking_count() {
    for skipped in [1, 5, 29] {
        let serviced: Vec<i64> = (10..=40).filter(|&f| f < 12 || f >= 12 + skipped).collect();
        let walked = walk(&build(&Spec {
            parking_connected: 7,
            routing_tail: 20_000,
            layout: Layout::LegacyServiced,
            ..shafts(vec![serving(elevator(1, 2, 100, 10, 40), &serviced)])
        }));
        assert_eq!(
            walked.shafts.as_ref().map(Vec::len),
            Some(1),
            "skipped {skipped}"
        );
        assert_eq!(walked.parking, Some(7), "skipped {skipped}");
        assert!(walked.notes.is_empty(), "skipped {skipped}");
    }
}

#[test]
fn a_stairless_export_from_before_the_routing_region_is_refused_loudly() {
    // An accepted limit: nothing in the file can tell it from a damaged one.
    gave_up(&walk(&build(&Spec {
        parking_connected: 7,
        layout: Layout::LegacyServiced,
        ..shafts(vec![
            serving(elevator(1, 2, 100, 10, 40), &[10, 40]),
            elevator(2, 3, 140, 10, 18),
        ])
    })));
}

#[test]
fn a_current_file_cut_at_the_legacy_length_is_refused_not_read_as_legacy() {
    let full = build(&end_stopper(Layout::Game));
    let legacy = build(&end_stopper(Layout::LegacyServiced)).len();
    gave_up(&walk(&full[..legacy]));
}

#[test]
fn a_stairless_current_tower_is_read_as_current() {
    let walked = walk(&build(&Spec {
        parking_connected: 7,
        routing_tail: 20_000,
        ..shafts(vec![serving(elevator(1, 2, 100, 10, 40), &[10, 20, 40])])
    }));
    assert_eq!(walked.shafts.as_ref().map(Vec::len), Some(1));
    assert_eq!(walked.parking, Some(7));
    assert!(walked.notes.is_empty(), "{:?}", walked.notes);
}

#[test]
fn a_lone_skipping_shaft_keeps_the_stairs_and_parking_in_either_layout() {
    // With no shaft after it the layout is decided from the tail.
    for layout in [Layout::LegacyServiced, Layout::Game] {
        let walked = walk(&build(&Spec {
            stairs: vec![stair(1, 120, 10), stair(0, 140, 11)],
            trailing_zeros: 20_000,
            ..end_stopper(layout)
        }));
        assert_eq!(xs(&walked), vec![100]);
        assert_eq!(walked.flights, Some(2));
        assert_eq!(walked.parking, Some(7));
        assert!(walked.notes.is_empty(), "{:?}", walked.notes);
    }
}

#[test]
fn a_legacy_last_shaft_skipping_one_floor_keeps_its_parking_count() {
    for skipped in [1, 2, 3] {
        let serviced: Vec<i64> = (10..=25).filter(|&f| f < 12 || f >= 12 + skipped).collect();
        let walked = walk(&build(&Spec {
            stairs: vec![stair(1, 120, 10)],
            parking_connected: 9,
            trailing_zeros: 20_000,
            layout: Layout::LegacyServiced,
            ..shafts(vec![serving(elevator(1, 2, 100, 10, 25), &serviced)])
        }));
        assert_eq!(
            walked.shafts.as_ref().map(Vec::len),
            Some(1),
            "skipped {skipped}"
        );
        assert_eq!(walked.parking, Some(9), "skipped {skipped}");
        assert_eq!(walked.flights, Some(1), "skipped {skipped}");
        assert!(walked.notes.is_empty(), "skipped {skipped}");
    }
}

#[test]
fn a_current_file_is_untouched_by_the_legacy_fallback() {
    let walked = walk(&build(&Spec {
        routing_tail: 0,
        layout: Layout::Game,
        ..older_build_pair()
    }));
    assert_eq!(xs(&walked), vec![100, 140]);
    assert!(walked.notes.is_empty(), "{:?}", walked.notes);
}

#[test]
fn a_car_count_outside_1_to_8_abandons_the_table() {
    // The span is sound, so only the car count can make the header malformed.
    for cars in [0, 9, 200] {
        let walked = walk(&build(&shafts(vec![
            elevator(1, cars, 100, 10, 20),
            elevator(1, 2, 140, 10, 20),
        ])));
        let notes = walked.notes.join(" ").to_lowercase();
        assert!(
            notes.contains("elevator table doesn't match"),
            "cars {cars}: {notes}"
        );
        assert!(walked.shafts.is_none(), "cars {cars}");
        assert!(walked.flights.is_none(), "cars {cars}");
    }
    for cars in [1, 8] {
        let walked = walk(&build(&shafts(vec![elevator(1, cars, 100, 10, 20)])));
        assert_eq!(xs(&walked), vec![100], "cars {cars}");
    }
}

#[test]
fn an_inverted_shaft_span_abandons_the_table_rather_than_mis_skipping() {
    // The builder refuses an inverted span, so damage a healthy file.
    let mut bytes = build(&shafts(vec![
        elevator(1, 2, 100, 20, 40),
        elevator(1, 2, 140, 10, 20),
    ]));
    let first = TDT_HEADER_SIZE + 120 * (6 + 188) + 4 + 512 * 18;
    assert_eq!(bytes[first], 1, "the first slot is built");
    bytes[first + 64] = 15;
    let walked = walk(&bytes);
    let notes = walked.notes.join(" ").to_lowercase();
    assert!(notes.contains("elevator table doesn't match"), "{notes}");
    assert!(walked.shafts.is_none());
    assert!(walked.flights.is_none());
    let save = parse_tdt(&bytes, "TOWER.TDT").unwrap().save;
    assert!(!transports(&save)
        .iter()
        .any(|t| t["x"] == 140 && t["kind"] == "elevatorStandard"));
}

#[test]
fn a_service_shaft_decodes_and_corrupt_geometry_is_trimmed_or_dropped() {
    let save = parse(&shafts(vec![
        elevator(2, 2, 100, 10, 20),
        elevator(1, 2, 500, 10, 20),
        elevator(1, 2, 100, 20, 20),
    ]))
    .save;
    let list = transports(&save);
    assert_eq!(list.len(), 2);
    assert!(list.iter().any(|t| t["kind"] == "elevatorService"));
    for t in list {
        assert!(i(&t["x"]) + i(&t["width"]) <= LOT_WIDTH);
        assert!(i(&t["top"]) > i(&t["bottom"]));
    }
}

#[test]
fn stairs_and_escalators_decode_and_taller_ones_stack_one_story_flights() {
    let save = parse(&Spec {
        stairs: vec![
            stair(1, 100, 10),
            stair(0, 120, 10),
            stair(3, 140, 10),
            stair(4, 160, 10),
        ],
        ..Spec::default()
    })
    .save;
    let list = transports(&save);
    let stairs: Vec<&Value> = list.iter().filter(|t| t["kind"] == "stairs").collect();
    let escalators = list.iter().filter(|t| t["kind"] == "escalator").count();
    assert_eq!((stairs.len(), escalators), (3, 4));
    let stacked: Vec<(i64, i64)> = stairs
        .iter()
        .filter(|t| t["x"] == 140)
        .map(|t| (i(&t["bottom"]), i(&t["top"])))
        .collect();
    assert_eq!(stacked, vec![(1, 2), (2, 3)]);
    assert!(list.iter().all(|t| i(&t["top"]) - i(&t["bottom"]) == 1));
}

#[test]
fn corrupt_shafts_are_dropped_or_trimmed_never_invented_and_counted() {
    let parsed = parse(&Spec {
        floors: vec![floor_edges(10, 100, 200)],
        ..shafts(vec![
            elevator(1, 2, 100, 115, 118),
            elevator(1, 2, 200, 10, 119),
        ])
    });
    let list = transports(&parsed.save);
    assert_eq!(list.len(), 1, "no stub at the crown");
    assert_eq!(i(&list[0]["x"]), 200);
    assert_eq!(
        i(&list[0]["top"]) - i(&list[0]["bottom"]),
        Kind::ElevatorStandard.max_span()
    );
    assert!(parsed.decode_stats.dropped_shafts >= 1);
    assert!(parsed.decode_stats.adjusted_shafts >= 1);
}

#[test]
fn of_two_overlapping_shafts_the_first_is_kept_and_the_other_counted() {
    let parsed = parse(&shafts(vec![
        elevator(1, 2, 100, 10, 20),
        elevator(1, 2, 101, 12, 18),
    ]));
    assert_eq!(transports(&parsed.save).len(), 1);
    assert!(parsed.decode_stats.dropped_shafts >= 1);
}

#[test]
fn the_64_link_pool_keeps_64_flights_and_counts_the_rest() {
    let parsed = parse(&Spec {
        stairs: (0..30).map(|n| stair(5, n * 12, 10)).collect(),
        ..Spec::default()
    });
    let flights = transports(&parsed.save)
        .iter()
        .filter(|t| t["kind"] == "stairs")
        .count();
    assert_eq!(flights, 64);
    assert!(parsed.decode_stats.dropped_flights > 0);
}

#[test]
fn the_same_bytes_always_decode_to_the_same_transports() {
    let a = parse(&sample()).save["transports"].clone();
    let b = parse(&sample()).save["transports"].clone();
    assert_eq!(a, b);
    let list = a.as_array().unwrap();
    assert!(list.iter().any(|t| t["kind"] == "elevatorStandard"));
    assert!(list.iter().any(|t| t["kind"] == "stairs"));
}

#[test]
fn the_sample_loads_into_the_engine_and_runs_six_hours() {
    let parsed = parse_tdt(&build(&sample()), "ROUNDTRIP.TDT").unwrap();
    let mut sim = deserialize(&parsed.save).expect("the import loads");
    assert_eq!((sim.money, sim.star), (1_500_000.0, 2));
    let saved = sim.serialize();
    assert_eq!(saved["towerName"], "ROUNDTRIP");
    assert_eq!(saved["mode"], "classic");
    let any =
        |kind: &str, test: &dyn Fn(&Value) -> bool| of_kind(&saved, kind).into_iter().any(test);
    assert!(any("condo", &|u| u["everOccupied"] == true));
    assert!(any("office", &|u| u["state"] == "occupied"));
    assert!(any("office", &|u| u["rent"].as_f64() == Some(15_000.0)));
    assert!(units(&saved).iter().any(|u| u["state"] == "asleep"));
    assert!(units(&saved).iter().any(|u| u["state"] == "dirty"));
    let kinds: Vec<&Value> = transports(&saved).iter().map(|t| &t["kind"]).collect();
    assert!(kinds.contains(&&Value::from("elevatorStandard")));
    assert!(kinds.contains(&&Value::from("stairs")));
    for _ in 0..6 * 60 {
        sim.tick(1.0);
    }
    assert!(sim.money.is_finite());
}

#[test]
fn a_room_under_construction_finishes_after_import() {
    let save = parse(&with_floors(vec![floor(20, vec![tenant(100, 109, -7)])])).save;
    let mut sim = deserialize(&save).expect("the import loads");
    let state = |sim: &verticopolis_engine::sim::Simulation| {
        of_kind(&sim.serialize(), "office")[0]["state"].clone()
    };
    assert_eq!(state(&sim), "construction");
    for _ in 0..24 {
        if state(&sim) != "construction" {
            break;
        }
        sim.tick(30.0);
    }
    assert_ne!(state(&sim), "construction");
}
