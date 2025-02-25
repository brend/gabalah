mod app;
mod err;
mod cpu;
mod memory;

use cpu::Cpu;

fn main()  {
    let mut cpu = Cpu::new();
    let rom = include_bytes!("../roms/test_rom.gb");
    cpu.load_rom(rom.to_vec());
    let mut i = 0;

    loop {
        i += 1;
        println!("Step: {}", i);
        cpu.step();
    }
}