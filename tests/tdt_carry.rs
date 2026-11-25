//! What a `.TDT` file carries for the game beyond the tower's shape: the
//! opening view in the header's view-scroll words (0x26 and 0x28), and every
//! shop, fast food and restaurant style, each at its own spot.

mod tdt_kit;

use serde_json::{json, Value};
use tdt_kit::*;
use verticopolis_engine::facilities::Kind;
use verticopolis_engine::tdt::format::{
    view_words_from_view, TDT_DEFAULT_VIEW_X, TDT_DEFAULT_VIEW_Y,
};

/// A lobby-only Classic tower, enough for the writer.
fn bare() -> Value {
    tower(vec![unit(1, "lobby", 1, 0, 1)], vec![])
}

fn words(bytes: &[u8]) -> (i64, i64) {
    (u16_at(bytes, 0x26) as i64, u16_at(bytes, 0x28) as i64)
}

#[test]
fn the_writer_stamps_the_saved_view_and_drops_its_zoom() {
    let mut save = bare();
    save["view"] = json!({ "tile": 200, "floor": 20, "zoom": 2 });
    assert_eq!(
        words(&build(&save).bytes),
        view_words_from_view(200.0, 20.0)
    );
    assert_ne!(
        view_words_from_view(200.0, 20.0),
        (TDT_DEFAULT_VIEW_X, TDT_DEFAULT_VIEW_Y)
    );
}

#[test]
fn a_save_without_a_whole_view_opens_on_the_new_tower_default() {
    let default = (TDT_DEFAULT_VIEW_X, TDT_DEFAULT_VIEW_Y);
    assert_eq!(words(&build(&bare()).bytes), default);
    let mut half = bare();
    half["view"] = json!({ "tile": 200 });
    assert_eq!(words(&build(&half).bytes), default);
}

#[test]
fn the_reader_brings_the_words_back_as_a_zoomless_view() {
    let mut save = bare();
    save["view"] = json!({ "tile": 200, "floor": 20 });
    let back = import(&build(&save).bytes).save;
    let view = back["view"].as_object().expect("a view");
    assert!((view["tile"].as_f64().unwrap() - 200.0).abs() < 0.1);
    assert!((view["floor"].as_f64().unwrap() - 20.0).abs() < 0.1);
    assert!(!view.contains_key("zoom"));
}

#[test]
fn a_file_with_no_view_words_reads_back_with_no_view() {
    let mut spec = builder::sample();
    spec.view = (0, 0);
    let save = reading::parse(&spec).save;
    assert!(save.get("view").is_none(), "{}", save["view"]);
}

#[test]
fn a_view_survives_export_and_import_within_a_tile_and_a_floor() {
    let mut save = sample_save();
    save["view"] = json!({ "tile": 178, "floor": 7 });
    let back = import(&build(&save).bytes).save;
    assert!((back["view"]["tile"].as_f64().unwrap() - 178.0).abs() <= 1.0);
    assert!((back["view"]["floor"].as_f64().unwrap() - 7.0).abs() <= 1.0);
}

#[test]
fn every_retail_style_survives_export_and_import_at_its_own_spot() {
    let kinds = [
        (Kind::FastFood, "fastFood", 16),
        (Kind::Restaurant, "restaurant", 24),
        (Kind::Shop, "shop", 12),
    ];
    let mut rooms = vec![unit(1, "lobby", 1, 0, 1)];
    let mut spots = vec![];
    for (floor, (kind, name, width)) in (2..).zip(kinds) {
        for (i, style) in kind.subtype_list().expect("styles").iter().enumerate() {
            let x = i as i64 * (width + 2);
            let mut room = unit_in(rooms.len() as i64 + 1, name, floor, x, width, "occupied");
            room["subtype"] = json!(style);
            rooms.push(room);
            spots.push((name, floor, x, *style));
        }
    }
    let back = import(&build(&tower(rooms, vec![])).bytes).save;
    for (name, floor, x, style) in spots {
        let room = units(&back)
            .iter()
            .find(|u| u["kind"] == name && u["floor"] == floor && u["x"] == x)
            .unwrap_or_else(|| panic!("{name} \"{style}\" lost its spot"));
        assert_eq!(room["subtype"], style, "{name} at {floor},{x}");
    }
}
