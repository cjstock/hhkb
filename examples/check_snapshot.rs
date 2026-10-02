//! Save or compare a full preservation snapshot during hardware acceptance.
use std::fs::{self, OpenOptions};
use std::io::Write;

use anyhow::{Context, Result, ensure};
use hhkb::Hhkb;
use hhkb::config::{CurrentKeymaps, Keymap, ModifierKeymaps};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    current: CurrentKeymaps,
    companion_base: Keymap,
    companion_fn: Keymap,
    sleep_minutes: u8,
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    ensure!(
        args.len() == 3 && matches!(args[1].as_str(), "save" | "check"),
        "Usage: check_snapshot <save|check> <path>"
    );
    let snapshot = Hhkb::new()?.read_snapshot()?;
    let record = Record {
        current: snapshot.current_keymaps(),
        companion_base: snapshot.maps[1].clone(),
        companion_fn: snapshot.maps[3].clone(),
        sleep_minutes: snapshot.sleep_minutes,
    };
    if args[1] == "save" {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&args[2])?;
        file.write_all(toml::to_string_pretty(&record)?.as_bytes())?;
        file.sync_all()?;
        println!("Saved complete baseline to {}", args[2]);
    } else {
        let mut baseline: Record =
            toml::from_str(&fs::read_to_string(&args[2])?).context("Invalid full snapshot")?;
        if baseline.current.modifiers.is_none() {
            baseline.current.modifiers = Some(ModifierKeymaps {
                base: baseline.companion_base.clone(),
                fn_layer: baseline.companion_fn.clone(),
            });
        }
        ensure!(record == baseline, "Full snapshot differs from baseline");
        println!("All four maps, sleep time, identity, and mode match baseline.");
    }
    Ok(())
}
