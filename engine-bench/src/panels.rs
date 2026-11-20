//! What each panel getter costs on the bench tower: the reads a frontend
//! makes every UI tick or while a panel is open. The medians are report-only
//! like the rest of the bench; the one check is a loose tripwire for a
//! getter that turns pathological, far above any healthy median.

use std::hint::black_box;
use std::time::Instant;

use verticopolis_engine::facilities::Kind;
use verticopolis_engine::panels::heatmap::HeatmapMode;
use verticopolis_engine::sim::Simulation;

/// Runs per getter; the median is reported.
pub const RUNS: usize = 30;
/// A median above this fails the run: a getter gone pathological, not a
/// slow runner.
pub const TRIPWIRE_MS: f64 = 50.0;

const HEATMAPS: [(&str, HeatmapMode); 4] = [
    ("congestion", HeatmapMode::Congestion),
    ("occupancy", HeatmapMode::Occupancy),
    ("satisfaction", HeatmapMode::Satisfaction),
    ("cleanliness", HeatmapMode::Cleanliness),
];

fn median_ms(sim: &mut Simulation, mut read: impl FnMut(&mut Simulation)) -> f64 {
    let mut samples: Vec<f64> = (0..RUNS)
        .map(|_| {
            let t0 = Instant::now();
            read(sim);
            t0.elapsed().as_secs_f64() * 1000.0
        })
        .collect();
    samples.sort_by(f64::total_cmp);
    samples[RUNS / 2]
}

/// Each getter's median in milliseconds, in a fixed order, or the first
/// getter whose median is over [`TRIPWIRE_MS`].
pub fn panel_getters(sim: &mut Simulation) -> Result<Vec<(String, f64)>, String> {
    let office = sim
        .tower
        .units
        .iter()
        .find(|u| u.kind == Kind::Office)
        .map_or(1, |u| u.id);
    let mut rows: Vec<(String, f64)> = vec![
        (
            "towerStats".into(),
            median_ms(sim, |s| {
                black_box(s.tower_stats());
            }),
        ),
        (
            "statsReport".into(),
            median_ms(sim, |s| {
                black_box(s.stats_report());
            }),
        ),
        (
            "housekeepingCoverage".into(),
            median_ms(sim, |s| {
                black_box(s.housekeeping_coverage());
            }),
        ),
        (
            "unitDetail".into(),
            median_ms(sim, |s| {
                black_box(s.unit_detail(office));
            }),
        ),
        (
            "peakCongestionHotspot".into(),
            median_ms(sim, |s| {
                black_box(s.peak_congestion_hotspot());
            }),
        ),
        (
            "onboardingFacts".into(),
            median_ms(sim, |s| {
                black_box(s.onboarding_facts());
            }),
        ),
    ];
    for (name, mode) in HEATMAPS {
        let ms = median_ms(sim, |s| {
            black_box(s.floor_heatmap(mode));
        });
        rows.push((format!("floorHeatmap({name})"), ms));
    }
    match rows.iter().find(|(_, ms)| *ms > TRIPWIRE_MS) {
        Some((name, ms)) => Err(format!(
            "panel getter {name} took a median {ms:.1}ms, over the {TRIPWIRE_MS}ms tripwire"
        )),
        None => Ok(rows),
    }
}

/// The report line for the getters' medians.
pub fn panel_line(rows: &[(String, f64)]) -> String {
    let cells: Vec<String> = rows
        .iter()
        .map(|(name, ms)| format!("{name}={ms:.3}"))
        .collect();
    format!("panel getters (median ms): {}", cells.join(" "))
}
