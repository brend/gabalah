#![deny(clippy::all)]
#![forbid(unsafe_code)]

pub const WIDTH: u32 = 160;
pub const HEIGHT: u32 = 144;

// Game Boy default palette: lightest to darkest
const GB_COLORS: [[u8; 4]; 4] = [
    [0x9B, 0xBC, 0x0F, 0xFF],
    [0x8B, 0xAC, 0x0F, 0xFF],
    [0x30, 0x62, 0x30, 0xFF],
    [0x0F, 0x38, 0x0F, 0xFF],
];

/// Renders the background layer into `screen` (RGBA, 160×144).
/// `ram` must be a 65536-byte slice (the full Game Boy address space).
/// Reads SCX/SCY scroll registers and respects LCDC tile map / data area bits.
pub fn render_frame(ram: &[u8], screen: &mut [u8]) {
    let lcdc = ram[0xFF40];
    let scy = ram[0xFF42] as usize;
    let scx = ram[0xFF43] as usize;

    // LCDC bit 3: BG tile map area (0=0x9800, 1=0x9C00)
    let tile_map_base: usize = if (lcdc & 0x08) != 0 { 0x9C00 } else { 0x9800 };
    // LCDC bit 4: BG & Window tile data area (0=0x8800 signed, 1=0x8000 unsigned)
    let signed_addressing = (lcdc & 0x10) == 0;

    for screen_y in 0..HEIGHT as usize {
        for screen_x in 0..WIDTH as usize {
            let bg_x = (scx + screen_x) & 0xFF;
            let bg_y = (scy + screen_y) & 0xFF;

            let tile_col = bg_x / 8;
            let tile_row = bg_y / 8;
            let tile_index = ram[tile_map_base + tile_row * 32 + tile_col];

            let tile_address = if signed_addressing {
                (0x9000i32 + (tile_index as i8 as i32 * 16)) as usize
            } else {
                0x8000 + tile_index as usize * 16
            };

            let pixel_x = bg_x % 8;
            let pixel_y = bg_y % 8;
            let lo = ram[tile_address + pixel_y * 2];
            let hi = ram[tile_address + pixel_y * 2 + 1];
            let bit = 7 - pixel_x;
            let palette_index = (((hi >> bit) & 1) << 1 | ((lo >> bit) & 1)) as usize;

            let offset = (screen_y * WIDTH as usize + screen_x) * 4;
            screen[offset..offset + 4].copy_from_slice(&GB_COLORS[palette_index]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blank_ram() -> Vec<u8> {
        vec![0u8; 65536]
    }

    fn blank_screen() -> Vec<u8> {
        vec![0u8; WIDTH as usize * HEIGHT as usize * 4]
    }

    fn pixel(screen: &[u8], x: usize, y: usize) -> [u8; 4] {
        let offset = (y * WIDTH as usize + x) * 4;
        screen[offset..offset + 4].try_into().unwrap()
    }

    // Write a single 8×8 tile's 2bpp data at the given address.
    // `rows` is 8 pairs of (lo_byte, hi_byte).
    fn write_tile(ram: &mut [u8], tile_address: usize, rows: [(u8, u8); 8]) {
        for (i, (lo, hi)) in rows.iter().enumerate() {
            ram[tile_address + i * 2] = *lo;
            ram[tile_address + i * 2 + 1] = *hi;
        }
    }

    #[test]
    fn zeroed_vram_produces_lightest_colour() {
        // Tile index 0 in tile map, all tile data zero → palette index 0.
        let ram = blank_ram();
        let mut screen = blank_screen();
        render_frame(&ram, &mut screen);
        for y in 0..HEIGHT as usize {
            for x in 0..WIDTH as usize {
                assert_eq!(pixel(&screen, x, y), GB_COLORS[0], "({x},{y})");
            }
        }
    }

    #[test]
    fn tile_pixel_decode_palette_indices() {
        // Tile at index 0, address 0x9000 (signed, bit 4 of LCDC = 0).
        // Row 0: lo=0b10101010, hi=0b11001100
        // Pixel bit 7: lo=1, hi=1 → palette 3
        // Pixel bit 6: lo=0, hi=1 → palette 2
        // Pixel bit 5: lo=1, hi=0 → palette 1
        // Pixel bit 4: lo=0, hi=0 → palette 0
        let mut ram = blank_ram();
        write_tile(&mut ram, 0x9000, [
            (0b10101010, 0b11001100),
            (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0),
        ]);
        let mut screen = blank_screen();
        render_frame(&ram, &mut screen);
        assert_eq!(pixel(&screen, 0, 0), GB_COLORS[3]); // bit 7
        assert_eq!(pixel(&screen, 1, 0), GB_COLORS[2]); // bit 6
        assert_eq!(pixel(&screen, 2, 0), GB_COLORS[1]); // bit 5
        assert_eq!(pixel(&screen, 3, 0), GB_COLORS[0]); // bit 4
    }

    #[test]
    fn scx_shifts_viewport_horizontally() {
        // Put a solid palette-3 tile at BG map col 1 (tile index 1 in map).
        // Set SCX=8 so BG col 1 appears at screen x=0.
        let mut ram = blank_ram();
        ram[0xFF43] = 8; // SCX
        ram[0x9801] = 1; // tile map col 1 → tile index 1
        // Tile 1 at 0x9000 + 1*16 = 0x9010: all pixels palette 3
        write_tile(&mut ram, 0x9010, [
            (0xFF, 0xFF), (0xFF, 0xFF), (0xFF, 0xFF), (0xFF, 0xFF),
            (0xFF, 0xFF), (0xFF, 0xFF), (0xFF, 0xFF), (0xFF, 0xFF),
        ]);
        let mut screen = blank_screen();
        render_frame(&ram, &mut screen);
        for row in 0..8 {
            assert_eq!(pixel(&screen, 0, row), GB_COLORS[3], "row {row}");
        }
    }

    #[test]
    fn scy_shifts_viewport_vertically() {
        // Solid palette-3 tile at BG map row 1 (tile index 1 in map).
        // Set SCY=8 so BG row 1 appears at screen y=0.
        let mut ram = blank_ram();
        ram[0xFF42] = 8; // SCY
        ram[0x9820] = 1; // tile map row 1 col 0 → tile index 1
        write_tile(&mut ram, 0x9010, [
            (0xFF, 0xFF), (0xFF, 0xFF), (0xFF, 0xFF), (0xFF, 0xFF),
            (0xFF, 0xFF), (0xFF, 0xFF), (0xFF, 0xFF), (0xFF, 0xFF),
        ]);
        let mut screen = blank_screen();
        render_frame(&ram, &mut screen);
        for col in 0..8 {
            assert_eq!(pixel(&screen, col, 0), GB_COLORS[3], "col {col}");
        }
    }

    #[test]
    fn lcdc_bit4_selects_unsigned_tile_addressing() {
        // LCDC bit 4 = 1 → tile data at 0x8000 + index*16 (unsigned).
        // Tile index 1 → 0x8010.
        let mut ram = blank_ram();
        ram[0xFF40] = 0x10; // LCDC bit 4
        ram[0x9800] = 1;    // tile map slot 0 → tile index 1
        write_tile(&mut ram, 0x8010, [
            (0xFF, 0xFF), (0, 0), (0, 0), (0, 0),
            (0, 0), (0, 0), (0, 0), (0, 0),
        ]);
        let mut screen = blank_screen();
        render_frame(&ram, &mut screen);
        // First row of tile 1: all palette 3
        for col in 0..8 {
            assert_eq!(pixel(&screen, col, 0), GB_COLORS[3], "col {col}");
        }
        // Second row: all palette 0
        for col in 0..8 {
            assert_eq!(pixel(&screen, col, 1), GB_COLORS[0], "col {col}");
        }
    }
}
