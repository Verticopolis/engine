//! `cargo run --release` builds the bench tower, walks one in-game day and
//! prints the report lines, then times each panel getter on the walked
//! tower. Exits 1 when the tower does not build, the walk does not cross
//! every hour boundary, or a getter's median passes the loose tripwire; a
//! slow tick never fails.

use std::process::exit;

use verticopolis_engine_bench::{big_tower, panel_getters, panel_line, walk};

fn main() {
    let report = big_tower().and_then(|mut sim| {
        let r = walk(&mut sim)?;
        let panels = panel_getters(&mut sim)?;
        Ok((r, panels))
    });
    match report {
        Ok((r, panels)) => {
            for line in r.lines() {
                println!("{line}");
            }
            println!("{}", panel_line(&panels));
        }
        Err(e) => {
            eprintln!("bench FAILED: {e}");
            exit(1);
        }
    }
}
