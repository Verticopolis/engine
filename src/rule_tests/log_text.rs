//! The log's prose on every conformance fixture. The conformance view drops
//! log text (it pins each entry's minute and kind), so a line worded or
//! formatted wrongly (a dollar amount without its thousands separators, a
//! flattened plural) would reach a player unnoticed. Each fixture opens the
//! way the game opens a file, runs a day, then the clock jumps to an hour
//! before the next quarter and the next maintenance period and runs across
//! them, so the rent, maintenance and checkout lines with their large
//! amounts are written.
use crate::sim::Simulation;
use crate::vctower_import::import_vctower;

const DAY: f64 = 1440.0;
const HOUR: f64 = 60.0;

const FIXTURES: [(&str, &str); 9] = [
    (
        "late-game-classic",
        include_str!("../../conformance/fixtures/late-game-classic.vctower"),
    ),
    (
        "sixseven-december",
        include_str!("../../conformance/fixtures/sixseven-december.vctower"),
    ),
    (
        "sixseven_2",
        include_str!("../../conformance/fixtures/sixseven_2.vctower"),
    ),
    (
        "split-tower-4",
        include_str!("../../conformance/fixtures/split-tower-4.vctower"),
    ),
    (
        "split-tower",
        include_str!("../../conformance/fixtures/split-tower.vctower"),
    ),
    (
        "towerone-star1",
        include_str!("../../conformance/fixtures/towerone-star1.vctower"),
    ),
    (
        "towerone-star4",
        include_str!("../../conformance/fixtures/towerone-star4.vctower"),
    ),
    (
        "towerone-vip-pending",
        include_str!("../../conformance/fixtures/towerone-vip-pending.vctower"),
    ),
    (
        "towerone_6",
        include_str!("../../conformance/fixtures/towerone_6.vctower"),
    ),
];

fn run(sim: &mut Simulation, hours: usize) {
    for _ in 0..hours {
        sim.tick(HOUR);
    }
}

/// Moves the clock, raw, to an hour before the next multiple of `period`
/// days that lies at least an hour ahead.
fn jump_before(sim: &mut Simulation, period: i64) {
    let minutes = sim.clock.minutes;
    let period_f = period as f64;
    let mut day = ((minutes / DAY / period_f).floor() + 1.0) * period_f;
    while day * DAY - HOUR <= minutes {
        day += period_f;
    }
    sim.clock.advance(day * DAY - HOUR - minutes);
}

fn lines_of(name: &str, text: &str) -> Vec<String> {
    let mut sim = import_vctower(text).unwrap_or_else(|e| panic!("{name}: {e}"));
    run(&mut sim, 24);
    let calendar = sim.clock.calendar;
    jump_before(&mut sim, calendar.quarter_days);
    run(&mut sim, 48);
    jump_before(&mut sim, calendar.maint_period_days);
    run(&mut sim, 48);
    sim.log.iter().map(|e| e.text.clone()).collect()
}

/// Whether `s` is a whole-dollar amount grouped in threes: `$1`, `$12,345`.
fn grouped_dollars(s: &str) -> bool {
    let Some(digits) = s.strip_prefix('$') else {
        return false;
    };
    let groups: Vec<&str> = digits.split(',').collect();
    let first = groups[0];
    !first.is_empty()
        && first.len() <= 3
        && first.bytes().all(|b| b.is_ascii_digit())
        && groups[1..]
            .iter()
            .all(|g| g.len() == 3 && g.bytes().all(|b| b.is_ascii_digit()))
}

/// Every `$` amount in `line`: the `$` and the digits and commas after it,
/// up to the last digit.
fn amounts(line: &str) -> Vec<&str> {
    let mut out = Vec::new();
    for (i, _) in line.match_indices('$') {
        let rest = &line[i + 1..];
        let run = rest
            .find(|c: char| !(c.is_ascii_digit() || c == ','))
            .unwrap_or(rest.len());
        let amount = rest[..run].trim_end_matches(',');
        if amount.bytes().any(|b| b.is_ascii_digit()) {
            out.push(&line[i..i + 1 + amount.len()]);
        }
    }
    out
}

const NOUNS: [&str; 6] = ["office", "condo", "clinic", "club", "rental", "hotel room"];

/// Each counted noun in `line` as (count, plural): a number, a space, one
/// of [`NOUNS`] with an optional `s`, then the end of a word that is not
/// followed by `(`.
fn counted(line: &str) -> Vec<(String, bool)> {
    let mut out = Vec::new();
    let bytes = line.as_bytes();
    for noun in NOUNS {
        for (at, _) in line.match_indices(&format!(" {noun}")) {
            let mut start = at;
            while start > 0 && (bytes[start - 1].is_ascii_digit() || bytes[start - 1] == b',') {
                start -= 1;
            }
            let count = &line[start..at];
            if !count.bytes().next().is_some_and(|b| b.is_ascii_digit()) {
                continue;
            }
            let mut end = at + 1 + noun.len();
            let plural = bytes.get(end) == Some(&b's');
            if plural {
                end += 1;
            }
            if bytes
                .get(end)
                .is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_')
            {
                continue;
            }
            if bytes.get(end) == Some(&b'(') {
                continue;
            }
            out.push((count.to_string(), plural));
        }
    }
    out
}

/// `$` then digits and commas, ending in a digit.
fn money(s: &str) -> bool {
    s.strip_prefix('$').is_some_and(|d| {
        !d.is_empty()
            && d.bytes().all(|b| b.is_ascii_digit() || b == b',')
            && d.ends_with(|c: char| c.is_ascii_digit())
    })
}

/// `<money> (<count> office[s])`.
fn rent_tail(s: &str) -> bool {
    let Some((amount, rest)) = s.split_once(" (") else {
        return false;
    };
    let Some(rest) = rest.strip_suffix(").") else {
        return false;
    };
    let Some((count, noun)) = rest.split_once(' ') else {
        return false;
    };
    money(amount)
        && count.bytes().next().is_some_and(|b| b.is_ascii_digit())
        && count.bytes().all(|b| b.is_ascii_digit() || b == b',')
        && (noun == "office" || noun == "offices")
}

/// The money lines whose wording and amount format are at stake: each line
/// that opens with the prefix must read as the check says.
const MONEY_LINES: [(&str, fn(&str) -> bool); 5] = [
    ("Quarterly office rent collected: ", rent_tail),
    ("Monthly office rent collected: ", rent_tail),
    ("Monthly maintenance paid: ", |s| {
        s.strip_suffix('.').is_some_and(money)
    }),
    ("Maintenance paid: ", |s| {
        s.strip_suffix('.').is_some_and(money)
    }),
    ("Hotel guests checked out: ", |s| {
        s.strip_suffix(" earned overnight.").is_some_and(money)
    }),
];

#[test]
fn the_matchers_read_what_they_name() {
    assert!(grouped_dollars("$1") && grouped_dollars("$12,345") && !grouped_dollars("$12345"));
    assert_eq!(
        amounts("paid $1,234,567. and $9,"),
        vec!["$1,234,567", "$9"]
    );
    assert_eq!(
        counted("rent: $10,000 (1 offices). 2 condo units, 3 clubs("),
        vec![("1".into(), true), ("2".into(), false)]
    );
    assert!(rent_tail("$10,000 (12 offices)."));
    assert!(!rent_tail("$10000 (12 desks)."));
}

#[test]
fn every_fixture_writes_its_money_and_counts_as_a_player_reads_them() {
    let lines: Vec<String> = FIXTURES
        .iter()
        .flat_map(|(name, text)| lines_of(name, text))
        .collect();
    for line in &lines {
        for amount in amounts(line) {
            assert!(grouped_dollars(amount), "{amount} in {line}");
        }
        for (count, plural) in counted(line) {
            assert_eq!(plural, count != "1", "{line}: the noun agrees with {count}");
        }
    }
    assert!(
        lines
            .iter()
            .any(|t| amounts(t).iter().any(|a| a.contains(','))),
        "a grouped dollar amount"
    );
    for (prefix, reads) in MONEY_LINES {
        let written: Vec<&String> = lines.iter().filter(|t| t.starts_with(prefix)).collect();
        assert!(!written.is_empty(), "a \"{prefix}\" line");
        for line in written {
            assert!(reads(&line[prefix.len()..]), "{line}");
        }
    }
}
