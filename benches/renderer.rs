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
    ram[0xFE02] = 1;  // tile index 1
    ram
}

fn bench_render_frame(c: &mut Criterion) {
    let ram = make_ram();
    let mut screen = vec![0u8; renderer::WIDTH as usize * renderer::HEIGHT as usize * 4];
    c.bench_function("render_frame", |b| {
        b.iter(|| renderer::render_frame(black_box(&ram), black_box(&mut screen)))
    });
}

criterion_group!(benches, bench_render_frame);
criterion_main!(benches);
