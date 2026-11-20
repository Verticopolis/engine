//! The monthly chance that a sold condo's household relocates: Classic
//! condos never relocate, and a Modern household's chance scales with its
//! size around the three-person mean (a missing household reads as three).
//! The departures run below: a Modern household leaves with a toasted
//! notice and a buy-back, Classic never draws the roll, and a relocation
//! notice never rescinds.
use super::{lay_row, put, unit, MID};
use crate::clock::GameMode;
use crate::facilities::Kind;
use crate::rules::CONDO_RELOCATION_CHANCE_MONTHLY;
use crate::sim::{LogKind, Simulation};
use crate::tower::{Unit, UnitState};

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-12
}

#[test]
fn classic_never_relocates_a_condo() {
    for residents in [None, Some(2), Some(3), Some(4), Some(5)] {
        assert_eq!(GameMode::Classic.condo_relocation_chance(residents), 0.0);
    }
}

#[test]
fn modern_scales_the_chance_with_the_household_size() {
    let chance = |r| GameMode::Modern.condo_relocation_chance(r);
    let base = CONDO_RELOCATION_CHANCE_MONTHLY;
    assert!(
        close(chance(Some(3)), base),
        "the mean household is neutral"
    );
    assert!(close(chance(None), base), "no household reads as three");
    assert!(chance(Some(2)) < chance(Some(3)));
    assert!(chance(Some(5)) > chance(Some(3)));
    assert!(close(chance(Some(5)), base * 5.0 / 3.0));
}

const MONTH: f64 = 60.0 * 24.0 * 30.0;

/// A served floor-2 tower holding one sold condo, happy and full; `edit`
/// adjusts the condo before the clock runs.
fn sold_condo(mode: GameMode, residents: i64, edit: impl FnOnce(&mut Unit)) -> (Simulation, i64) {
    let mut sim = Simulation::new_game(4, mode);
    sim.money = 10_000_000.0;
    sim.star = 1;
    lay_row(&mut sim, Kind::Lobby, 1);
    lay_row(&mut sim, Kind::Floor, 2);
    assert!(sim.build_transport(Kind::ElevatorStandard, MID, 1, 2).ok);
    let id = put(&mut sim, Kind::Condo, 2, MID + 2);
    let u = sim.tower.get_unit_mut(id).unwrap();
    u.state = UnitState::Occupied;
    u.ever_occupied = true;
    u.residents = (mode == GameMode::Modern).then_some(residents);
    u.satisfaction = 1.0;
    edit(u);
    (sim, id)
}

/// The month a five-person Modern household gives its relocation notice.
fn notice_month(sim: &mut Simulation, id: i64) -> Option<usize> {
    (0..360).find(|_| {
        sim.tick(MONTH);
        unit(sim, id).extras().vacate_reason == Some("relocation")
    })
}

#[test]
fn a_modern_household_relocates_with_a_toasted_notice_and_a_buy_back() {
    let (mut sim, id) = sold_condo(GameMode::Modern, 5, |_| {});
    let before = sim.money;
    assert!(
        notice_month(&mut sim, id).is_some(),
        "no relocation in 30 years"
    );
    let notice = sim
        .log
        .iter()
        .find(|e| e.text.contains("is relocating"))
        .unwrap();
    assert_eq!(notice.kind, LogKind::Bad, "the warning toasts");
    sim.tick(MONTH);
    assert_ne!(unit(&sim, id).extras().vacate_reason, Some("relocation"));
    assert!(sim.money < before, "the buy-back was charged");
}

#[test]
fn the_same_seed_relocates_in_the_same_month() {
    let month = || {
        let (mut sim, id) = sold_condo(GameMode::Modern, 5, |_| {});
        notice_month(&mut sim, id)
    };
    let first = month();
    assert!(first.is_some());
    assert_eq!(month(), first);
}

#[test]
fn a_classic_owner_stays_and_never_draws_the_roll() {
    let (mut classic, id) = sold_condo(GameMode::Classic, 3, |_| {});
    let (mut modern, _) = sold_condo(GameMode::Modern, 3, |_| {});
    assert_eq!(classic.rng.seed(), modern.rng.seed());
    classic.tick(MONTH);
    modern.tick(MONTH);
    assert_ne!(
        classic.rng.seed(),
        modern.rng.seed(),
        "Modern draws the roll and Classic skips it"
    );
    for _ in 0..60 {
        classic.tick(MONTH);
        assert_ne!(
            unit(&classic, id).extras().vacate_reason,
            Some("relocation")
        );
    }
    assert_eq!(unit(&classic, id).state, UnitState::Occupied);
}

/// An overdue notice on a fully happy condo: a relocation still leaves and is
/// bought back, while a fixable cause (access) rescinds.
#[test]
fn a_relocation_notice_never_rescinds_and_an_access_notice_does() {
    let overdue = |reason: &'static str| {
        sold_condo(GameMode::Modern, 4, |u| {
            u.state = UnitState::Vacating;
            u.extras_mut().vacate_reason = Some(reason);
            u.extras_mut().vacate_at = Some(0.0);
        })
    };
    let (mut moving, id) = overdue("relocation");
    let before = moving.money;
    moving.tick(60.0);
    assert_ne!(unit(&moving, id).state, UnitState::Occupied);
    assert!(moving.money < before, "bought back on departure");
    let (mut fixed, id) = overdue("access");
    fixed.tick(60.0);
    assert_eq!(unit(&fixed, id).state, UnitState::Occupied);
}
