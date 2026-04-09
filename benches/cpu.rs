use criterion::{black_box, criterion_group, criterion_main, Criterion};
use gabalah::cpu::Cpu;
use gabalah::memory::Addr;

const CYCLES_PER_FRAME: usize = 70_224;
const INTERRUPT_SERVICE_CYCLES: usize = 20;

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

/// Keeps LCD timing and STAT mode/coincidence bits in sync with the app loop.
fn tick_lcd(cpu: &mut Cpu, cycles: usize, ppu_line_cycles: &mut usize) {
    let lcdc = cpu.memory.read_byte(Addr(0xFF40));
    if (lcdc & 0x80) == 0 {
        *ppu_line_cycles = 0;
        cpu.memory.set_ly_raw(0);
        update_stat(cpu, 0, false, false);
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

    let ly = cpu.memory.read_byte(Addr(0xFF44));
    let mode = if ly >= 144 {
        1
    } else if *ppu_line_cycles < 80 {
        2
    } else if *ppu_line_cycles < 252 {
        3
    } else {
        0
    };
    let lyc = cpu.memory.read_byte(Addr(0xFF45));
    update_stat(cpu, mode, ly == lyc, false);
}

fn update_stat(cpu: &mut Cpu, mode: u8, coincidence: bool, allow_interrupt: bool) {
    let old_stat = cpu.memory.read_byte(Addr(0xFF41));
    let old_mode = old_stat & 0x03;
    let old_coincidence = (old_stat & 0x04) != 0;
    let mut new_stat = (old_stat & 0x78) | (mode & 0x03);
    if coincidence {
        new_stat |= 0x04;
    }
    cpu.memory.set_stat_raw(new_stat);

    if !allow_interrupt {
        return;
    }

    let mode_changed = mode != old_mode;
    let mode_irq = match mode {
        0 => (new_stat & 0x08) != 0,
        1 => (new_stat & 0x10) != 0,
        2 => (new_stat & 0x20) != 0,
        _ => false,
    };
    let lyc_irq = coincidence && !old_coincidence && (new_stat & 0x40) != 0;
    if (mode_changed && mode_irq) || lyc_irq {
        cpu.raise_if(0x02);
    }
}

fn is_interrupt_pending(cpu: &Cpu) -> bool {
    cpu.registers.ime && (cpu.get_ie() & cpu.get_if()) != 0
}

fn interrupt(cpu: &mut Cpu) -> usize {
    cpu.halted = false;
    cpu.registers.ime = false;
    let pending = cpu.get_if() & cpu.get_ie();
    for bit in 0..5u8 {
        if pending & (1 << bit) != 0 {
            cpu.clear_if(1 << bit);
            let vector = 0x0040u16 + (bit as u16) * 8;
            cpu.memory
                .write_word(Addr(cpu.registers.sp.wrapping_sub(2)), cpu.registers.pc);
            cpu.registers.sp = cpu.registers.sp.wrapping_sub(2);
            cpu.registers.pc = vector;
            cpu.total_cycles += INTERRUPT_SERVICE_CYCLES as u64;
            return INTERRUPT_SERVICE_CYCLES;
        }
    }
    0
}

fn step_cycles(cpu: &mut Cpu, cycle_budget: usize, ppu_line_cycles: &mut usize) {
    let mut cycles_this_step = 0;
    while cycles_this_step < cycle_budget {
        let cycles = cpu.step();
        cycles_this_step += cycles;
        tick_lcd(cpu, cycles, ppu_line_cycles);
        if cpu.memory.tick(cycles as u32) {
            cpu.raise_if(0x04);
        }
        if is_interrupt_pending(cpu) {
            let interrupt_cycles = interrupt(cpu);
            cycles_this_step += interrupt_cycles;
            tick_lcd(cpu, interrupt_cycles, ppu_line_cycles);
            if cpu.memory.tick(interrupt_cycles as u32) {
                cpu.raise_if(0x04);
            }
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
            step_cycles(&mut cpu, CYCLES_PER_FRAME, &mut ppu_line_cycles);
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
