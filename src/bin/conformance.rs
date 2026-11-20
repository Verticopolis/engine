//! Replays every scenario in `conformance/scenarios/` on the Rust engine and
//! compares each checkpoint with `conformance/expected.json`. Exits 1 unless
//! every scenario matches end to end; the report names, per scenario, the
//! first divergent checkpoint or the first command the port cannot run yet.
//! It also checks each mode's catalog against `conformance/catalog-digests.json`.
//!
//! With `--update` it writes both locks instead, from this run: every
//! scenario file's checkpoints into `expected.json` and each mode's catalog
//! digest into `catalog-digests.json`. It writes nothing unless every
//! scenario runs to its end, and it refuses to run in CI (`CI` set), so a
//! lock only moves on purpose, in the pull request that moves it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::exit;

use verticopolis_engine::scenario::{run_scenario, Checkpoint, Run, RunError, Scenario};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let update = args.iter().any(|a| a == "--update");
    let root = args
        .iter()
        .find(|a| !a.starts_with("--"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("conformance"));
    if update {
        if std::env::var_os("CI").is_some() {
            eprintln!("--update regenerates the locks on purpose and never runs in CI");
            exit(2);
        }
        exit(write_locks(&root));
    }
    let lock_path = root.join("expected.json");
    let lock_text = match std::fs::read_to_string(&lock_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("cannot read {}: {e}", lock_path.display());
            exit(2);
        }
    };
    let lock: serde_json::Value = serde_json::from_str(&lock_text).expect("expected.json parses");
    let want: BTreeMap<String, Vec<Checkpoint>> =
        serde_json::from_value(lock["scenarios"].clone()).expect("lock shape");
    // The scenario directory and the lock must name the same set, so a new
    // scenario cannot slip past the referee and a stale lock entry cannot
    // pass for a deleted one.
    let on_disk = scenario_ids(&root);
    let in_lock: Vec<String> = want.keys().cloned().collect();
    if on_disk != in_lock {
        let unlocked: Vec<_> = on_disk
            .iter()
            .filter(|id| !want.contains_key(*id))
            .collect();
        let orphaned: Vec<_> = in_lock.iter().filter(|id| !on_disk.contains(id)).collect();
        eprintln!(
            "scenario set differs from the lock: not in the lock {unlocked:?}, no file {orphaned:?}"
        );
        exit(1);
    }
    // The report prints sorted by id, the order the lock is written in.
    let ids: Vec<String> = want.keys().cloned().collect();
    let runs = replay_all(&root, &ids);
    let mut all_ok = true;
    for ((id, want), run) in want.iter().zip(runs.iter()) {
        let run = match run {
            Ok(r) => r,
            Err(e) => {
                all_ok = false;
                println!("{id}: FAILED: {e}");
                continue;
            }
        };
        let got = &run.checkpoints;
        let matched = got
            .iter()
            .zip(want.iter())
            .take_while(|(g, w)| g == w)
            .count();
        let total = want.len();
        if matched < got.len() {
            all_ok = false;
            let g = &got[matched];
            let w = want.get(matched);
            if w.is_none() {
                println!(
                    "{id}: EXTRA checkpoint {matched} ({}) past the lock's {total}",
                    g.label
                );
                continue;
            }
            println!(
                "{id}: DIVERGED at checkpoint {matched} ({}): state {} vs {}, crowd {} vs {}, events {} vs {}",
                g.label,
                g.state,
                w.map(|w| w.state.as_str()).unwrap_or("none"),
                g.crowd,
                w.map(|w| w.crowd.as_str()).unwrap_or("none"),
                g.events,
                w.map(|w| w.events.as_str()).unwrap_or("none")
            );
            continue;
        }
        match &run.error {
            None if matched == total => println!("{id}: ok ({total} checkpoints)"),
            None => {
                all_ok = false;
                println!("{id}: SHORT, {matched} of {total} checkpoints");
            }
            Some(RunError::Unsupported { index, what }) => {
                all_ok = false;
                println!("{id}: {matched} of {total} checkpoints match, then command {index} ({what}) is not ported yet");
            }
            Some(RunError::Failed { index, what }) => {
                all_ok = false;
                println!("{id}: {matched} of {total} checkpoints match, then command {index} FAILED: {what}");
            }
        }
    }
    match check_catalogs(&root) {
        Ok(()) => println!("catalogs: ok"),
        Err(e) => {
            all_ok = false;
            println!("catalogs: {e}");
        }
    }
    if !all_ok {
        exit(1);
    }
}

/// The scenario files' stems under `root/scenarios`, sorted. Exits 2 when
/// the directory cannot be read.
fn scenario_ids(root: &Path) -> Vec<String> {
    let scenario_dir = root.join("scenarios");
    let mut ids: Vec<String> = match std::fs::read_dir(&scenario_dir)
        .and_then(|entries| entries.collect::<Result<Vec<_>, _>>())
    {
        Ok(entries) => entries
            .iter()
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                name.strip_suffix(".json").map(str::to_string)
            })
            .collect(),
        Err(e) => {
            eprintln!("cannot read {}: {e}", scenario_dir.display());
            exit(2);
        }
    };
    ids.sort();
    ids
}

/// Replays `ids` in order. Every scenario is its own engine, so they replay
/// on separate threads; a WebAssembly build (the wasm32-wasip1 referee) has
/// no threads, so it replays them one after another.
fn replay_all(root: &Path, ids: &[String]) -> Vec<Result<Run, String>> {
    if cfg!(target_family = "wasm") {
        return ids.iter().map(|id| replay(root, id)).collect();
    }
    std::thread::scope(|scope| {
        let handles: Vec<_> = ids
            .iter()
            .map(|id| scope.spawn(move || replay(root, id)))
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("scenario thread"))
            .collect()
    })
}

/// `--update`: runs every scenario file and, when all of them run to their
/// end, writes `expected.json` from their checkpoints and
/// `catalog-digests.json` from the catalogs. Returns the exit code.
fn write_locks(root: &Path) -> i32 {
    use verticopolis_engine::catalog::catalog_digest;
    use verticopolis_engine::clock::GameMode;
    let ids = scenario_ids(root);
    let runs = replay_all(root, &ids);
    let mut failed = false;
    let mut entries = Vec::new();
    for (id, run) in ids.iter().zip(runs) {
        match run {
            Ok(Run {
                error: None,
                checkpoints,
            }) => {
                let rows: Vec<String> = checkpoints
                    .iter()
                    .map(|c| {
                        format!(
                            "      {}",
                            serde_json::json!({
                                "label": c.label,
                                "state": c.state,
                                "crowd": c.crowd,
                                "events": c.events,
                            })
                        )
                    })
                    .collect();
                entries.push(format!(
                    "    {}: [\n{}\n    ]",
                    serde_json::Value::from(id.as_str()),
                    rows.join(",\n")
                ));
            }
            Ok(Run { error: Some(e), .. }) => {
                failed = true;
                println!("{id}: did not run to its end: {e:?}");
            }
            Err(e) => {
                failed = true;
                println!("{id}: FAILED: {e}");
            }
        }
    }
    if failed {
        println!("no lock written");
        return 1;
    }
    let lock = format!(
        "{{\n  \"scenarios\": {{\n{}\n  }}\n}}\n",
        entries.join(",\n")
    );
    let catalogs = serde_json::json!({
        "catalogs": {
            "classic": catalog_digest(GameMode::Classic),
            "modern": catalog_digest(GameMode::Modern),
        }
    });
    let catalogs = format!(
        "{}\n",
        serde_json::to_string_pretty(&catalogs).expect("the digests serialize")
    );
    for (name, text) in [("expected.json", lock), ("catalog-digests.json", catalogs)] {
        let path = root.join(name);
        if let Err(e) = std::fs::write(&path, text) {
            eprintln!("cannot write {}: {e}", path.display());
            return 2;
        }
        println!("wrote {}", path.display());
    }
    0
}

/// Replays one scenario. A panic inside it (a save the loader refuses) is
/// caught and reported as that scenario's failure instead of aborting the
/// referee, where the target unwinds (a WebAssembly build aborts instead).
fn replay(root: &Path, id: &str) -> Result<Run, String> {
    let path = root.join("scenarios").join(format!("{id}.json"));
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let scenario = Scenario::parse(&text)?;
    // The internal id equals the stem, so a copied scenario cannot run under
    // another file's lock entry.
    if scenario.id != id {
        return Err(format!(
            "{}: scenario id {:?} does not match the file stem {:?}",
            path.display(),
            scenario.id,
            id
        ));
    }
    std::panic::catch_unwind(|| run_scenario(&scenario, root)).map_err(|p| {
        let msg = p
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
            .unwrap_or_else(|| "panic".to_string());
        format!("engine panicked: {msg}")
    })
}

/// Each mode's catalog digest against `catalog-digests.json`, the lock
/// `--update` writes from this catalog.
fn check_catalogs(root: &std::path::Path) -> Result<(), String> {
    use verticopolis_engine::catalog::catalog_digest;
    use verticopolis_engine::clock::GameMode;
    let path = root.join("catalog-digests.json");
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let lock: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    let modes = [GameMode::Classic, GameMode::Modern];
    let mut moved = Vec::new();
    for mode in modes {
        match lock["catalogs"][mode.as_str()].as_str() {
            None => moved.push(format!("{} has no digest in the lock", mode.as_str())),
            Some(want) => {
                let got = catalog_digest(mode);
                if got != want {
                    moved.push(format!("{} {got} vs {want}", mode.as_str()));
                }
            }
        }
    }
    // The TypeScript check compares the whole map, so a stray entry fails
    // there; refuse it here too.
    if let Some(entries) = lock["catalogs"].as_object() {
        for key in entries.keys() {
            if !modes.iter().any(|m| m.as_str() == key) {
                moved.push(format!("the lock names an unknown mode {key}"));
            }
        }
    }
    if moved.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "DIVERGED: {} ({})",
            moved.join(", "),
            path.display()
        ))
    }
}
