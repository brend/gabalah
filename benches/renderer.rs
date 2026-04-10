use criterion::{black_box, criterion_group, criterion_main, Criterion};
use gabalah::renderer;

fn make_ram() -> Vec<u8> {
    let mut ram = vec![0u8; 65536];
    // LCD on, BG + OBJ enabled, unsigned tile data (0x8000)
    ram[0xFF40] = 0x93;
    // Identity palette
    ram[0xFF47] = 0xE4;
    ram[0xFF48] = 0xE4;
    // Mild scroll so tile fetches aren't all tile 0
    ram[0xFF42] = 17; // SCY
    ram[0xFF43] = 23; // SCX
                      // Populate a few tiles with non-zero data so rendering isn't trivially empty
    for i in 0..16usize {
        let addr = 0x8000 + i * 16;
        for row in 0..8usize {
            ram[addr + row * 2] = (i as u8).wrapping_mul(17);
            ram[addr + row * 2 + 1] = (i as u8).wrapping_mul(31);
        }
    }
    // Scatter tile indices across the BG map
    for i in 0..32usize {
        for j in 0..32usize {
            ram[0x9800 + i * 32 + j] = ((i + j) % 16) as u8;
        }
    }
    // One sprite on-screen
    ram[0xFE00] = 24; // Y: screen row 8
    ram[0xFE01] = 16; // X: screen col 8
    ram[0xFE02] = 1; // tile index 1
    ram
}

fn make_priority_ram() -> Vec<u8> {
    let mut ram = make_ram();
    ram[0xFE03] = 0x80; // sprite priority behind non-zero BG
    ram
}

fn make_uniform_latches(ram: &[u8]) -> [renderer::ScanlineRegs; renderer::HEIGHT as usize] {
    let mut latches = [renderer::ScanlineRegs::default(); renderer::HEIGHT as usize];
    let regs = renderer::ScanlineRegs {
        lcdc: ram[0xFF40],
        scy: ram[0xFF42],
        scx: ram[0xFF43],
        bgp: ram[0xFF47],
        wy: ram[0xFF4A],
        wx: ram[0xFF4B],
    };
    latches.fill(regs);
    latches
}

fn make_split_latches(ram: &[u8]) -> [renderer::ScanlineRegs; renderer::HEIGHT as usize] {
    let mut latches = make_uniform_latches(ram);
    for (line, regs) in latches.iter_mut().enumerate() {
        if line < 16 {
            // Simulate a fixed HUD region
            regs.scx = 0;
            regs.wy = 0;
            regs.wx = 7;
            regs.lcdc |= 0x20;
        } else {
            // Simulate gameplay scroll changing by scanline
            regs.scx = ((line * 3) & 0xFF) as u8;
            regs.scy = ((17 + line * 2) & 0xFF) as u8;
        }
    }
    latches
}

fn bench_render_frame_alloc(c: &mut Criterion) {
    let ram = make_ram();
    let mut screen = vec![0u8; renderer::WIDTH as usize * renderer::HEIGHT as usize * 4];
    c.bench_function("render_frame_alloc", |b| {
        b.iter(|| renderer::render_frame(black_box(&ram), black_box(&mut screen)))
    });
}

fn bench_render_frame_reuse(c: &mut Criterion) {
    let ram = make_ram();
    let mut screen = vec![0u8; renderer::WIDTH as usize * renderer::HEIGHT as usize * 4];
    let mut bg_opaque = vec![false; renderer::WIDTH as usize * renderer::HEIGHT as usize];
    c.bench_function("render_frame_reuse", |b| {
        b.iter(|| {
            renderer::render_frame_with_bg_opaque(
                black_box(&ram),
                black_box(&mut screen),
                &mut bg_opaque,
            )
        })
    });
}

fn bench_render_frame_reuse_priority(c: &mut Criterion) {
    let ram = make_priority_ram();
    let mut screen = vec![0u8; renderer::WIDTH as usize * renderer::HEIGHT as usize * 4];
    let mut bg_opaque = vec![false; renderer::WIDTH as usize * renderer::HEIGHT as usize];
    c.bench_function("render_frame_reuse_priority", |b| {
        b.iter(|| {
            renderer::render_frame_with_bg_opaque(
                black_box(&ram),
                black_box(&mut screen),
                &mut bg_opaque,
            )
        })
    });
}

fn bench_render_frame_latched_uniform(c: &mut Criterion) {
    let ram = make_ram();
    let latches = make_uniform_latches(&ram);
    let mut screen = vec![0u8; renderer::WIDTH as usize * renderer::HEIGHT as usize * 4];
    let mut bg_opaque = vec![false; renderer::WIDTH as usize * renderer::HEIGHT as usize];
    c.bench_function("render_frame_latched_uniform", |b| {
        b.iter(|| {
            renderer::render_frame_with_scanline_latches(
                black_box(&ram),
                black_box(&mut screen),
                &mut bg_opaque,
                black_box(&latches),
            )
        })
    });
}

fn bench_render_frame_latched_split(c: &mut Criterion) {
    let ram = make_ram();
    let latches = make_split_latches(&ram);
    let mut screen = vec![0u8; renderer::WIDTH as usize * renderer::HEIGHT as usize * 4];
    let mut bg_opaque = vec![false; renderer::WIDTH as usize * renderer::HEIGHT as usize];
    c.bench_function("render_frame_latched_split", |b| {
        b.iter(|| {
            renderer::render_frame_with_scanline_latches(
                black_box(&ram),
                black_box(&mut screen),
                &mut bg_opaque,
                black_box(&latches),
            )
        })
    });
}

criterion_group!(
    benches,
    bench_render_frame_alloc,
    bench_render_frame_reuse,
    bench_render_frame_reuse_priority,
    bench_render_frame_latched_uniform,
    bench_render_frame_latched_split
);
criterion_main!(benches);
