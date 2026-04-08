use criterion::{black_box, criterion_group, criterion_main, Criterion};
use gabalah::cpu::Cpu;
use gabalah::memory::Addr;

const CYCLES_PER_FRAME: usize = 70_224;

/// ROM filled with NOPs (0x00) — measures raw instruction dispatch overhead
/// with no memory side-effects.
fn make_nop_cpu() -> Cpu {
    let mut cpu = Cpu::new();
    cpu.load_rom(vec![0x00u8; 0x8000]);
    cpu
}

/// ROM filled with ADD A,A (0x87) — exercises the ALU and flag-write path.
fn make_alu_cpu() -> Cpu {
    let mut cpu = Cpu::new();
    cpu.load_rom(vec![0x87u8; 0x8000]);
    cpu
}

/// Tight `JR -2` loop at 0x0100 — realistic steady-state for step_frame measurement.
/// JR takes 12 cycles/iteration → ~5,852 dispatches per frame.
fn make_loop_cpu() -> Cpu {
    let mut cpu = Cpu::new();
    let mut rom = vec![0x00u8; 0x8000];
    rom[0x0100] = 0x18; // JR
    rom[0x0101] = 0xFE; // offset -2 → loops back to 0x0100
    cpu.load_rom(rom);
    cpu
}

/// Advances PPU line state by `cycles`. Mirrors the LY/VBlank logic in app.rs.
fn tick_lcd(cpu: &mut Cpu, cycles: usize, ppu_line_cycles: &mut usize) {
    let lcdc = cpu.memory.read_byte(Addr(0xFF40));
    if (lcdc & 0x80) == 0 {
        *ppu_line_cycles = 0;
        cpu.memory.set_ly_raw(0);
        return;
    }
    *ppu_line_cycles += cycles;
    while *ppu_line_cycles >= 456 {
        *ppu_line_cycles -= 456;
        let ly = cpu.memory.read_byte(Addr(0xFF44));
        let new_ly = if ly >= 153 { 0 } else { ly + 1 };
        cpu.memory.set_ly_raw(new_ly);
        if new_ly == 144 {
            cpu.raise_if(0x01);
        }
    }
}

fn bench_cpu_step_nop(c: &mut Criterion) {
    let mut cpu = make_nop_cpu();
    c.bench_function("cpu_step_nop", |b| b.iter(|| black_box(cpu.step())));
}

fn bench_cpu_step_alu(c: &mut Criterion) {
    let mut cpu = make_alu_cpu();
    c.bench_function("cpu_step_alu", |b| b.iter(|| black_box(cpu.step())));
}

/// Full frame: ~5,852 JR dispatches + timer ticks + LCD line progression per iteration.
/// This is the number that determines whether the emulator meets its 16.7 ms budget.
fn bench_step_frame(c: &mut Criterion) {
    let mut cpu = make_loop_cpu();
    let mut ppu_line_cycles = 0usize;
    c.bench_function("step_frame", |b| {
        b.iter(|| {
            let mut cycles_run = 0;
            while cycles_run < CYCLES_PER_FRAME {
                let cycles = cpu.step();
                cycles_run += cycles;
                tick_lcd(&mut cpu, cycles, &mut ppu_line_cycles);
                if cpu.memory.tick(cycles as u32) {
                    cpu.raise_if(0x04);
                }
            }
            black_box(cpu.total_cycles)
        })
    });
}

criterion_group!(
    benches,
    bench_cpu_step_nop,
    bench_cpu_step_alu,
    bench_step_frame
);
criterion_main!(benches);
