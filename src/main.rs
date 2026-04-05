mod app;
mod cpu;
mod err;
mod memory;
mod renderer;

use cpu::Cpu;
use std::env;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // read the rom from file
    let rom = read_rom()?;
    // create a new CPU and load the rom
    let mut cpu = Cpu::new();
    cpu.load_rom(rom);
    Ok(app::run_loop(cpu)?)
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
