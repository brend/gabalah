mod app;
mod config;
mod cpu;
mod memory;
mod renderer;
mod ui;

use cpu::Cpu;
use std::env;
use std::fs;

const MOONEYE_PASS: &[u8] = &[3, 5, 8, 13, 21, 34];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() >= 4 && args[1] == "--test" {
        let frames: usize = args[2].parse()?;
        let rom = fs::read(&args[3])?;
        let mut cpu = Cpu::new();
        cpu.load_rom(rom);
        let serial = app::run_headless(cpu, frames);
        if serial == MOONEYE_PASS {
            println!("PASS");
        } else {
            let hex: Vec<String> = serial.iter().map(|b| format!("{b:02x}")).collect();
            println!("FAIL [{}]", hex.join(" "));
        }
        return Ok(());
    }

    let rom = read_rom()?;
    let mut cpu = Cpu::new();
    cpu.load_rom(rom);
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
    )
}

fn read_rom() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <rom file>", args[0]);
        std::process::exit(1);
    }
    let filename = &args[1];
    let rom = fs::read(filename)?;
    Ok(rom)
}
