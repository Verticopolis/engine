//! The referee's `--update` run on a one-scenario suite: it writes the same
//! checkpoints and catalog digests the repository's locks hold, the check
//! run then passes on them, and it writes nothing when a scenario fails or
//! when `CI` is set.
#![cfg(feature = "conformance")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const REAL: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/conformance");
const SCENARIO: &str = "starter-classic";

/// A fresh suite directory holding the one scenario.
fn suite(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("vc-referee-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("scenarios")).unwrap();
    fs::copy(
        Path::new(REAL).join(format!("scenarios/{SCENARIO}.json")),
        root.join(format!("scenarios/{SCENARIO}.json")),
    )
    .unwrap();
    root
}

fn referee(root: &Path, args: &[&str], ci: bool) -> i32 {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_conformance"));
    cmd.args(args).arg(root).env_remove("CI");
    if ci {
        cmd.env("CI", "true");
    }
    cmd.output().unwrap().status.code().unwrap()
}

fn json(path: PathBuf) -> serde_json::Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn an_update_writes_the_locks_the_repository_holds_and_the_check_passes() {
    let root = suite("ok");
    assert_eq!(referee(&root, &["--update"], false), 0);
    let real = json(Path::new(REAL).join("expected.json"));
    let wrote = json(root.join("expected.json"));
    assert_eq!(wrote["scenarios"].as_object().unwrap().len(), 1);
    assert_eq!(wrote["scenarios"][SCENARIO], real["scenarios"][SCENARIO]);
    assert_eq!(
        fs::read_to_string(root.join("catalog-digests.json")).unwrap(),
        fs::read_to_string(Path::new(REAL).join("catalog-digests.json")).unwrap(),
    );
    assert_eq!(referee(&root, &[], false), 0, "the check passes on it");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn an_update_writes_nothing_when_a_scenario_fails_or_in_ci() {
    let root = suite("refused");
    assert_eq!(referee(&root, &["--update"], true), 2, "never in CI");
    assert!(!root.join("expected.json").exists());
    // A copy under another file name fails: its id is not its stem.
    fs::copy(
        root.join(format!("scenarios/{SCENARIO}.json")),
        root.join("scenarios/renamed.json"),
    )
    .unwrap();
    assert_eq!(referee(&root, &["--update"], false), 1);
    assert!(!root.join("expected.json").exists());
    assert!(!root.join("catalog-digests.json").exists());
    fs::remove_dir_all(root).unwrap();
}
