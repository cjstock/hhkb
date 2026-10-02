use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use hhkb::config::{CurrentKeymaps, diff_keymaps, format_toml};
use hhkb::{Hhkb, validate_identity};

#[derive(Parser)]
#[command(
    version,
    about = "Read, edit, and verify HHKB current-mode base/Fn maps"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Export aligned keyboard rows; the destination must not exist.
    Export {
        #[arg(default_value = "hhkb.toml")]
        path: PathBuf,
        /// Export version 1 byte arrays, including modifier maps, instead of rows.
        #[arg(long)]
        raw: bool,
    },
    /// Validate, back up, write, and verify base and Fn assignments.
    Import { path: PathBuf },
    /// Preview every change against the connected keyboard without writing maps.
    Diff {
        #[arg(default_value = "hhkb.toml")]
        path: PathBuf,
    },
    /// Align rows in place, retain comments, or upgrade a raw export to rows.
    Format {
        #[arg(default_value = "hhkb.toml")]
        path: PathBuf,
    },
}

fn save_new(path: &Path, config: &CurrentKeymaps) -> Result<()> {
    let text = config.to_toml()?;
    save_document_new(path, &text)
}

fn save_document_new(path: &Path, text: &str) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .with_context(|| {
            format!(
                "Cannot create {} (existing files are never overwritten)",
                path.display()
            )
        })?;
    file.write_all(text.as_bytes())?;
    file.sync_all()?;
    Ok(())
}

fn format_file(path: &Path) -> Result<()> {
    // Resolve symlinks before replacing the file so formatting retains the link.
    let destination = fs::canonicalize(path)?;
    let original = fs::read_to_string(&destination)?;
    let formatted = format_toml(&original)?;
    if formatted == original {
        return Ok(());
    }
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let mut name = destination
        .file_name()
        .context("Input path has no filename")?
        .to_os_string();
    name.push(format!(".format-{timestamp}.tmp"));
    let temporary = destination.with_file_name(name);
    let mut created = false;
    let result = (|| -> Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        created = true;
        file.write_all(formatted.as_bytes())?;
        fs::set_permissions(&temporary, fs::metadata(&destination)?.permissions())?;
        file.sync_all()?;
        fs::rename(&temporary, &destination)?;
        Ok(())
    })();
    if result.is_err() && created {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn backup(path: &Path, config: &CurrentKeymaps) -> Result<PathBuf> {
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let mut filename = path
        .file_name()
        .context("Input path has no filename")?
        .to_os_string();
    filename.push(format!(".backup-{timestamp}.toml"));
    let destination = path.with_file_name(filename);
    save_new(&destination, config).context("Backup failed; write aborted")?;
    Ok(destination)
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    if let Some(Command::Format { path }) = &cli.command {
        format_file(path).with_context(|| format!("Cannot format {}", path.display()))?;
        println!("Formatted {}", path.display());
        return Ok(());
    }
    // Parse and validate imported bytes before even opening the device.
    let imported = match &cli.command {
        Some(Command::Import { path } | Command::Diff { path }) => Some(CurrentKeymaps::from_toml(
            &fs::read_to_string(path).with_context(|| format!("Cannot read {}", path.display()))?,
        )?),
        _ => None,
    };
    let hhkb = Hhkb::new()?;
    match cli.command {
        Some(Command::Export { path, raw }) => {
            let config = hhkb.read_current_keymaps()?;
            let text = if raw {
                config.to_toml()?
            } else {
                config.to_visual_toml()?
            };
            save_document_new(&path, &text)?;
            println!("Exported {}", path.display());
        }
        Some(Command::Import { path }) => {
            let config = imported.unwrap();
            let snapshot = hhkb.read_snapshot()?;
            validate_identity(&config, &snapshot)?;
            let backup = backup(&path, &snapshot.current_keymaps())?;
            println!("Backup saved to {}", backup.display());
            hhkb.write_current_keymaps(&config)?;
            println!("Verified base/Fn keys and modifiers, sleep time, and mode.");
        }
        Some(Command::Diff { .. }) => {
            let config = imported.unwrap();
            let snapshot = hhkb.read_snapshot()?;
            validate_identity(&config, &snapshot)?;
            let changes = diff_keymaps(&snapshot.current_keymaps(), &config)?;
            if changes.is_empty() {
                println!("No changes.");
            } else {
                for change in &changes {
                    println!("{change}");
                }
                println!("{} change(s).", changes.len());
            }
        }
        Some(Command::Format { .. }) => unreachable!(),
        None => {
            println!("{:?}", hhkb.get_info()?);
            println!("{:?}", hhkb.get_dip_state()?);
            let mode = hhkb.get_mode()?;
            println!("{mode:?}");
            println!("{:?}", hhkb.base_layer(mode)?);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use hhkb::config::{Keymap, ModifierKeymaps};

    #[test]
    fn formatting_upgrades_without_changing_bytes_and_keeps_invalid_input() {
        let directory = std::env::temp_dir().join(format!(
            "hhkb-format-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&directory).unwrap();
        let path = directory.join("maps.toml");
        let config = CurrentKeymaps {
            schema_version: 1,
            model: "PD-KB800WNS".into(),
            serial: "TEST-SERIAL".into(),
            mode: 0,
            base: Keymap(std::array::from_fn(|i| i as u8)),
            fn_layer: Keymap([0; 128]),
            modifiers: None,
        };
        save_new(&path, &config).unwrap();
        format_file(&path).unwrap();
        let formatted = fs::read_to_string(&path).unwrap();
        assert!(formatted.contains("schema_version = 2"));
        assert_eq!(CurrentKeymaps::from_toml(&formatted).unwrap(), config);
        format_file(&path).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), formatted);
        fs::write(&path, "not valid TOML").unwrap();
        assert!(format_file(&path).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "not valid TOML");
        assert_eq!(fs::read_dir(&directory).unwrap().count(), 1);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn export_does_not_overwrite_and_backup_is_adjacent() {
        let directory = std::env::temp_dir().join(format!(
            "hhkb-files-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&directory).unwrap();
        let path = directory.join("maps.toml");
        let config = CurrentKeymaps {
            schema_version: 1,
            model: "test".into(),
            serial: "test".into(),
            mode: 0,
            base: Keymap([0; 128]),
            fn_layer: Keymap(std::array::from_fn(|i| {
                if [44, 31, 7].contains(&i) { 0 } else { 1 }
            })),
            modifiers: Some(ModifierKeymaps {
                base: Keymap([0xa3; 128]),
                fn_layer: Keymap(std::array::from_fn(|i| {
                    if [44, 31, 7].contains(&i) { 0 } else { 0x50 }
                })),
            }),
        };
        save_new(&path, &config).unwrap();
        assert!(save_new(&path, &config).is_err());
        let first = backup(&path, &config).unwrap();
        let second = backup(&path, &config).unwrap();
        assert_ne!(first, second);
        assert_eq!(first.parent(), path.parent());
        assert_eq!(
            CurrentKeymaps::from_toml(&fs::read_to_string(first).unwrap()).unwrap(),
            config
        );
        assert!(backup(&directory.join("missing/maps.toml"), &config).is_err());
        fs::remove_dir_all(directory).unwrap();
    }
}
