use criterion::{black_box, criterion_group, criterion_main, Criterion};
use gabalah::cpu::Cpu;

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

fn bench_cpu_step_nop(c: &mut Criterion) {
    let mut cpu = make_nop_cpu();
    c.bench_function("cpu_step_nop", |b| {
        b.iter(|| black_box(cpu.step()))
    });
}

fn bench_cpu_step_alu(c: &mut Criterion) {
    let mut cpu = make_alu_cpu();
    c.bench_function("cpu_step_alu", |b| {
        b.iter(|| black_box(cpu.step()))
    });
}

criterion_group!(benches, bench_cpu_step_nop, bench_cpu_step_alu);
criterion_main!(benches);
