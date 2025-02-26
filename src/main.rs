mod app;
mod err;
mod cpu;
mod memory;

use std::env;
use std::fs;
use cpu::Cpu;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // read the rom from file
    let rom = read_rom()?;
    // create a new CPU and load the rom
    let mut cpu = Cpu::new();
    cpu.load_rom(rom);
    let mut i = 0;

    loop {
        i += 1;
        println!("Step: {}", i);
        cpu.step();
    }
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