use gabalah::{app, config, cpu::Cpu, rom_loader};
use std::env;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

const MOONEYE_PASS: &[u8] = &[3, 5, 8, 13, 21, 34];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let cli = parse_cli_args(&args).unwrap_or_else(|err| {
        eprintln!("{err}");
        std::process::exit(1);
    });
    let rom_input_path = Path::new(&cli.rom_path);
    let rom = rom_loader::load_rom_from_path(rom_input_path, cli.entry.as_deref())?;
    let save_path = derive_save_path(rom_input_path, cli.entry.as_deref());

    if let Some(frames) = cli.test_frames {
        let mut cpu = Cpu::new();
        cpu.load_rom(rom);
        load_battery_ram_from_disk(&mut cpu, save_path.as_deref());
        let serial = app::run_headless(cpu, frames);
        if serial == MOONEYE_PASS {
            println!("PASS");
        } else {
            let hex: Vec<String> = serial.iter().map(|b| format!("{b:02x}")).collect();
            println!("FAIL [{}]", hex.join(" "));
        }
        return Ok(());
    }

    let mut cpu = Cpu::new();
    cpu.load_rom(rom);
    load_battery_ram_from_disk(&mut cpu, save_path.as_deref());
    let (backend_kind, backend_options) = config::load_graphics_settings()?;
    let window_scale = config::load_window_scale()?;
    let controls = config::load_controls()?;
    let debug_dump_settings = config::load_debug_dump_settings()?;
    app::run_loop(
        cpu,
        backend_kind,
        backend_options,
        window_scale,
        controls,
        debug_dump_settings,
        save_path,
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CliArgs {
    test_frames: Option<usize>,
    entry: Option<String>,
    rom_path: String,
}

fn parse_cli_args(args: &[String]) -> Result<CliArgs, rom_loader::RomLoadError> {
    let program = args
        .first()
        .cloned()
        .unwrap_or_else(|| "gabalah".to_string());
    let usage = format!("Usage: {program} [--test <frames>] [--entry <archive-path>] <rom file>");

    let mut test_frames = None;
    let mut entry = None;
    let mut rom_path = None;
    let mut i = 1;

    while i < args.len() {
        let current = &args[i];
        match current.as_str() {
            "--test" => {
                if test_frames.is_some() {
                    return Err(rom_loader::RomLoadError::InvalidCliArgument(
                        "`--test` may only be provided once".to_string(),
                    ));
                }
                i += 1;
                let Some(raw_frames) = args.get(i) else {
                    return Err(rom_loader::RomLoadError::InvalidCliArgument(format!(
                        "missing frame count after `--test`\n{usage}"
                    )));
                };
                let frames = raw_frames.parse::<usize>().map_err(|_| {
                    rom_loader::RomLoadError::InvalidCliArgument(format!(
                        "invalid frame count `{raw_frames}` for `--test`\n{usage}"
                    ))
                })?;
                test_frames = Some(frames);
            }
            "--entry" => {
                if entry.is_some() {
                    return Err(rom_loader::RomLoadError::InvalidCliArgument(
                        "`--entry` may only be provided once".to_string(),
                    ));
                }
                i += 1;
                let Some(raw_entry) = args.get(i) else {
                    return Err(rom_loader::RomLoadError::InvalidCliArgument(format!(
                        "missing archive entry path after `--entry`\n{usage}"
                    )));
                };
                entry = Some(raw_entry.clone());
            }
            value if value.starts_with("--") => {
                return Err(rom_loader::RomLoadError::InvalidCliArgument(format!(
                    "unknown argument `{value}`\n{usage}"
                )));
            }
            value => {
                if rom_path.is_some() {
                    return Err(rom_loader::RomLoadError::InvalidCliArgument(format!(
                        "unexpected extra positional argument `{value}`\n{usage}"
                    )));
                }
                rom_path = Some(value.to_string());
            }
        }
        i += 1;
    }

    let Some(rom_path) = rom_path else {
        return Err(rom_loader::RomLoadError::InvalidCliArgument(usage));
    };

    Ok(CliArgs {
        test_frames,
        entry,
        rom_path,
    })
}

fn derive_save_path(rom_input_path: &Path, entry: Option<&str>) -> Option<PathBuf> {
    if entry.is_some() {
        return None;
    }
    if matches!(
        rom_input_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase())
            .as_deref(),
        Some("zip" | "gz" | "7z")
    ) {
        return None;
    }
    Some(rom_input_path.with_extension("sav"))
}

fn load_battery_ram_from_disk(cpu: &mut Cpu, save_path: Option<&Path>) {
    if !cpu.has_battery_backed_ram() {
        return;
    }
    let Some(save_path) = save_path else {
        eprintln!(
            "Battery-backed cartridge detected, but save persistence is disabled for archive inputs."
        );
        return;
    };

    match fs::read(save_path) {
        Ok(bytes) => {
            if !cpu.load_battery_backed_ram(&bytes) {
                eprintln!(
                    "Ignoring save file '{}': cartridge does not expose battery-backed RAM.",
                    save_path.to_string_lossy()
                );
            }
        }
        Err(err) if err.kind() == ErrorKind::NotFound => {}
        Err(err) => {
            eprintln!(
                "Failed to read save file '{}': {err}",
                save_path.to_string_lossy()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{derive_save_path, parse_cli_args, CliArgs};
    use std::path::Path;

    fn args(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| item.to_string()).collect()
    }

    #[test]
    fn parses_standard_mode_with_entry() {
        let cli = parse_cli_args(&args(&[
            "gabalah",
            "--entry",
            "roms/game.gb",
            "archive.zip",
        ]))
        .expect("arguments should parse");
        assert_eq!(
            cli,
            CliArgs {
                test_frames: None,
                entry: Some("roms/game.gb".to_string()),
                rom_path: "archive.zip".to_string(),
            }
        );
    }

    #[test]
    fn parses_test_mode_and_entry() {
        let cli = parse_cli_args(&args(&[
            "gabalah",
            "--test",
            "1200",
            "--entry",
            "suite/pass.gb",
            "tests.7z",
        ]))
        .expect("arguments should parse");
        assert_eq!(
            cli,
            CliArgs {
                test_frames: Some(1200),
                entry: Some("suite/pass.gb".to_string()),
                rom_path: "tests.7z".to_string(),
            }
        );
    }

    #[test]
    fn rejects_unknown_flag() {
        let err = parse_cli_args(&args(&["gabalah", "--wat", "rom.gb"]))
            .expect_err("unknown flag should fail");
        assert!(err.to_string().contains("unknown argument"));
    }

    #[test]
    fn rejects_missing_test_value() {
        let err = parse_cli_args(&args(&["gabalah", "--test", "rom.gb"]))
            .expect_err("missing --test value should fail");
        assert!(err.to_string().contains("invalid frame count"));
    }

    #[test]
    fn rejects_missing_entry_value() {
        let err = parse_cli_args(&args(&["gabalah", "--entry"]))
            .expect_err("missing --entry value should fail");
        assert!(err.to_string().contains("missing archive entry path"));
    }

    #[test]
    fn rejects_duplicate_test_flag() {
        let err = parse_cli_args(&args(&[
            "gabalah", "--test", "60", "--test", "120", "rom.gb",
        ]))
        .expect_err("duplicate --test should fail");
        assert!(err.to_string().contains("may only be provided once"));
    }

    #[test]
    fn rejects_duplicate_entry_flag() {
        let err = parse_cli_args(&args(&[
            "gabalah", "--entry", "a.gb", "--entry", "b.gb", "rom.zip",
        ]))
        .expect_err("duplicate --entry should fail");
        assert!(err.to_string().contains("may only be provided once"));
    }

    #[test]
    fn rejects_extra_positional_argument() {
        let err = parse_cli_args(&args(&["gabalah", "rom1.gb", "rom2.gb"]))
            .expect_err("extra positional argument should fail");
        assert!(err
            .to_string()
            .contains("unexpected extra positional argument"));
    }

    #[test]
    fn derive_save_path_for_raw_rom() {
        let path = derive_save_path(Path::new("roms/zelda.gb"), None)
            .expect("raw ROM should map to a save path");
        assert_eq!(path, Path::new("roms/zelda.sav"));
    }

    #[test]
    fn derive_save_path_disables_archives() {
        assert!(derive_save_path(Path::new("bundle.zip"), None).is_none());
        assert!(derive_save_path(Path::new("bundle.gz"), None).is_none());
        assert!(derive_save_path(Path::new("bundle.7z"), None).is_none());
    }

    #[test]
    fn derive_save_path_disables_explicit_archive_entries() {
        assert!(derive_save_path(Path::new("bundle.zip"), Some("games/zelda.gb")).is_none());
    }
}
