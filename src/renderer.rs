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
    if (lcdc & 0x80) == 0 {
        for pixel in screen.chunks_exact_mut(4) {
            pixel.copy_from_slice(&GB_COLORS[0]);
        }
        return;
    }

    // On DMG, LCDC bit 0 gates both BG and Window.
    if (lcdc & 0x01) != 0 {
        render_bg(ram, screen);
        render_window(ram, screen);
    }
    render_obj(ram, screen);
}

fn render_obj(ram: &[u8], screen: &mut [u8]) {
    let lcdc = ram[0xFF40];

    // LCDC bit 1: OBJ (sprite) enable
    if (lcdc & 0x02) == 0 {
        return;
    }

    // TODO: Handle sprite priority (attribute bit 7)
    let obj_tile_base: usize = 0x8000;
    let obj_height: usize = if (lcdc & 0x04) != 0 { 16 } else { 8 };
    let mut obj_addr = 0xFE00;

    while obj_addr <= 0xFE9F {
        let tile_y = ram[obj_addr] as i16 - 16;
        let tile_x = ram[obj_addr + 1] as i16 - 8;
        let tile_index = ram[obj_addr + 2];
        let attributes = ram[obj_addr + 3];
        let x_flip = (attributes & 0x20) != 0;
        let y_flip = (attributes & 0x40) != 0;
        let obp = if (attributes & 0x10) != 0 {
            ram[0xFF49]
        } else {
            ram[0xFF48]
        };

        for row in 0..obj_height {
            let screen_y = tile_y + row as i16;
            if !(0..HEIGHT as i16).contains(&screen_y) {
                continue;
            }

            let obj_row = if y_flip { obj_height - 1 - row } else { row };
            let tile_row = obj_row % 8;
            let row_tile_index = if obj_height == 16 {
                ((tile_index & 0xFE) as usize) + (obj_row / 8)
            } else {
                tile_index as usize
            };
            let tile_addr = obj_tile_base + (row_tile_index * 16);
            let lo = ram[tile_addr + tile_row * 2];
            let hi = ram[tile_addr + tile_row * 2 + 1];

            for col in 0..8 {
                let screen_x = tile_x + col as i16;
                if !(0..WIDTH as i16).contains(&screen_x) {
                    continue;
                }

                let obj_col = if x_flip { 7 - col } else { col };
                let bit = 7 - obj_col;
                let palette_index = ((hi >> bit) & 1) << 1 | ((lo >> bit) & 1);

                if palette_index == 0 {
                    continue;
                }

                let color = (obp >> (palette_index * 2)) & 0x3;
                let offset = (screen_y as usize * WIDTH as usize + screen_x as usize) * 4;
                screen[offset..offset + 4].copy_from_slice(&GB_COLORS[color as usize]);
            }
        }

        obj_addr += 4;
    }
}

fn tile_address(tile_index: u8, signed_addressing: bool) -> usize {
    if signed_addressing {
        (0x9000i32 + (tile_index as i8 as i32 * 16)) as usize
    } else {
        0x8000 + tile_index as usize * 16
    }
}

fn tile_palette_index(ram: &[u8], tile_address: usize, pixel_x: usize, pixel_y: usize) -> u8 {
    let lo = ram[tile_address + pixel_y * 2];
    let hi = ram[tile_address + pixel_y * 2 + 1];
    let bit = 7 - pixel_x;
    ((hi >> bit) & 1) << 1 | ((lo >> bit) & 1)
}

fn render_bg(ram: &[u8], screen: &mut [u8]) {
    let lcdc = ram[0xFF40];
    let bgp = ram[0xFF47];
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

            let tile_address = tile_address(tile_index, signed_addressing);

            let pixel_x = bg_x % 8;
            let pixel_y = bg_y % 8;
            let palette_index = tile_palette_index(ram, tile_address, pixel_x, pixel_y);
            let shade = ((bgp >> (palette_index * 2)) & 0x03) as usize;

            let offset = (screen_y * WIDTH as usize + screen_x) * 4;
            screen[offset..offset + 4].copy_from_slice(&GB_COLORS[shade]);
        }
    }
}

fn render_window(ram: &[u8], screen: &mut [u8]) {
    let lcdc = ram[0xFF40];
    if (lcdc & 0x20) == 0 {
        return;
    }

    let bgp = ram[0xFF47];
    let wy = ram[0xFF4A] as usize;
    let wx = ram[0xFF4B] as usize;
    if wy >= HEIGHT as usize {
        return;
    }

    // LCDC bit 6: Window tile map area (0=0x9800, 1=0x9C00)
    let tile_map_base: usize = if (lcdc & 0x40) != 0 { 0x9C00 } else { 0x9800 };
    // LCDC bit 4: BG & Window tile data area (0=0x8800 signed, 1=0x8000 unsigned)
    let signed_addressing = (lcdc & 0x10) == 0;

    for screen_y in wy..HEIGHT as usize {
        let win_y = screen_y - wy;
        for screen_x in 0..WIDTH as usize {
            if screen_x + 7 < wx {
                continue;
            }
            let win_x = screen_x + 7 - wx;
            let tile_col = win_x / 8;
            let tile_row = win_y / 8;
            let tile_index = ram[tile_map_base + tile_row * 32 + tile_col];
            let tile_address = tile_address(tile_index, signed_addressing);
            let pixel_x = win_x % 8;
            let pixel_y = win_y % 8;
            let palette_index = tile_palette_index(ram, tile_address, pixel_x, pixel_y);
            let shade = ((bgp >> (palette_index * 2)) & 0x03) as usize;

            let offset = (screen_y * WIDTH as usize + screen_x) * 4;
            screen[offset..offset + 4].copy_from_slice(&GB_COLORS[shade]);
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
        ram[0xFF40] = 0x81; // LCDC: display on, BG enabled, signed tile data
        ram[0xFF47] = 0xE4; // BGP: identity mapping (3→3, 2→2, 1→1, 0→0)
        write_tile(
            &mut ram,
            0x9000,
            [
                (0b10101010, 0b11001100),
                (0, 0),
                (0, 0),
                (0, 0),
                (0, 0),
                (0, 0),
                (0, 0),
                (0, 0),
            ],
        );
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
        ram[0xFF40] = 0x81; // LCDC: display on, BG enabled, signed tile data
        ram[0xFF47] = 0xE4; // BGP: identity mapping
        ram[0xFF43] = 8; // SCX
        ram[0x9801] = 1; // tile map col 1 → tile index 1
                         // Tile 1 at 0x9000 + 1*16 = 0x9010: all pixels palette 3
        write_tile(
            &mut ram,
            0x9010,
            [
                (0xFF, 0xFF),
                (0xFF, 0xFF),
                (0xFF, 0xFF),
                (0xFF, 0xFF),
                (0xFF, 0xFF),
                (0xFF, 0xFF),
                (0xFF, 0xFF),
                (0xFF, 0xFF),
            ],
        );
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
        ram[0xFF40] = 0x81; // LCDC: display on, BG enabled, signed tile data
        ram[0xFF47] = 0xE4; // BGP: identity mapping
        ram[0xFF42] = 8; // SCY
        ram[0x9820] = 1; // tile map row 1 col 0 → tile index 1
        write_tile(
            &mut ram,
            0x9010,
            [
                (0xFF, 0xFF),
                (0xFF, 0xFF),
                (0xFF, 0xFF),
                (0xFF, 0xFF),
                (0xFF, 0xFF),
                (0xFF, 0xFF),
                (0xFF, 0xFF),
                (0xFF, 0xFF),
            ],
        );
        let mut screen = blank_screen();
        render_frame(&ram, &mut screen);
        for col in 0..8 {
            assert_eq!(pixel(&screen, col, 0), GB_COLORS[3], "col {col}");
        }
    }

    // --- Window rendering ---

    #[test]
    fn window_disabled_leaves_background_unchanged() {
        let mut ram = blank_ram();
        ram[0xFF47] = 0xE4; // BGP: identity mapping
        ram[0xFF40] = 0x99; // LCDC: display on, BG on, BG map=0x9C00, unsigned tile data, window off
        ram[0xFF4A] = 0; // WY
        ram[0xFF4B] = 7; // WX: window origin x=0 if enabled
        ram[0x9800] = 1; // window map tile index (would be visible if window were enabled)
        write_tile(&mut ram, 0x8010, [(0xFF, 0xFF); 8]); // tile 1: shade 3
        let mut screen = blank_screen();
        render_frame(&ram, &mut screen);
        assert_eq!(
            pixel(&screen, 0, 0),
            GB_COLORS[0],
            "window-off must not overwrite BG"
        );
    }

    #[test]
    fn window_uses_wx_minus_7_and_wy() {
        let mut ram = blank_ram();
        ram[0xFF47] = 0xE4; // BGP: identity mapping
        ram[0xFF40] = 0xB9; // LCDC: display on, BG on, window on, BG map=0x9C00, unsigned tile data
        ram[0xFF4A] = 5; // WY: window starts at y=5
        ram[0xFF4B] = 15; // WX: window starts at x=8 (WX-7)
        ram[0x9800] = 1; // window map tile 0 -> tile index 1
        write_tile(&mut ram, 0x8010, [(0xFF, 0xFF); 8]); // tile 1: shade 3
        let mut screen = blank_screen();
        render_frame(&ram, &mut screen);
        assert_eq!(
            pixel(&screen, 7, 5),
            GB_COLORS[0],
            "left of window edge must stay BG"
        );
        assert_eq!(pixel(&screen, 8, 4), GB_COLORS[0], "above WY must stay BG");
        assert_eq!(
            pixel(&screen, 8, 5),
            GB_COLORS[3],
            "window should appear at WX-7,WY"
        );
    }

    // --- Sprite (OBJ) rendering ---

    #[test]
    fn obj_disabled_leaves_background_unchanged() {
        // LCDC bit 1 clear: sprites must not appear even if OAM has valid data.
        let mut ram = blank_ram();
        ram[0xFF47] = 0xE4; // BGP: identity
        ram[0xFF40] = 0xA1; // LCDC: display on, BG on, window on, OBJ off (bit 1 = 0)
        ram[0xFF48] = 0xE4; // OBP0: identity
                            // Place a solid sprite tile at VRAM index 1
        write_tile(&mut ram, 0x8010, [(0xFF, 0xFF); 8]);
        // OAM entry 0: Y=24 (screen 8), X=16 (screen 8), tile 1
        ram[0xFE00] = 24;
        ram[0xFE01] = 16;
        ram[0xFE02] = 1;
        let mut screen = blank_screen();
        render_frame(&ram, &mut screen);
        // Background with all-zero tile data → shade 0 everywhere
        assert_eq!(
            pixel(&screen, 8, 8),
            GB_COLORS[0],
            "sprite must not appear when OBJ disabled"
        );
    }

    #[test]
    fn sprite_appears_at_oam_position() {
        // OAM Y=24 → screen y=8, OAM X=16 → screen x=8.
        let mut ram = blank_ram();
        ram[0xFF40] = 0x93; // LCDC: display on, BG on, OBJ on (bit 1), unsigned tile data (bit 4)
        ram[0xFF47] = 0xE4; // BGP: identity (background stays shade 0)
        ram[0xFF48] = 0xE4; // OBP0: identity
                            // Sprite tile 1 at 0x8010: all pixels palette index 3
        write_tile(&mut ram, 0x8010, [(0xFF, 0xFF); 8]);
        ram[0xFE00] = 24; // Y: screen row 8
        ram[0xFE01] = 16; // X: screen col 8
        ram[0xFE02] = 1; // tile index
        let mut screen = blank_screen();
        render_frame(&ram, &mut screen);
        assert_eq!(
            pixel(&screen, 8, 8),
            GB_COLORS[3],
            "sprite pixel should be shade 3"
        );
        assert_eq!(
            pixel(&screen, 0, 0),
            GB_COLORS[0],
            "background outside sprite untouched"
        );
    }

    #[test]
    fn sprite_palette_index_zero_is_transparent() {
        // A tile with all-zero data → every pixel is palette index 0 → transparent.
        // The background (shade 0) must show through.
        let mut ram = blank_ram();
        ram[0xFF40] = 0x93; // LCDC: display on, BG on, OBJ on, unsigned addressing
        ram[0xFF47] = 0xE4;
        ram[0xFF48] = 0xE4;
        // Tile 1 all zeroes (already the case in blank_ram)
        ram[0xFE00] = 24;
        ram[0xFE01] = 16;
        ram[0xFE02] = 1;
        let mut screen = blank_screen();
        render_frame(&ram, &mut screen);
        assert_eq!(
            pixel(&screen, 8, 8),
            GB_COLORS[0],
            "transparent sprite must not overwrite BG"
        );
    }

    #[test]
    fn sprite_attribute_x_flip_mirrors_horizontally() {
        let mut ram = blank_ram();
        ram[0xFF40] = 0x93; // LCDC: display on, BG on, OBJ on, 8x8 sprites
        ram[0xFF47] = 0xE4; // BGP: identity
        ram[0xFF48] = 0xE4; // OBP0: identity

        // Sprite tile 1 row 0 has a single palette-3 pixel at the left edge.
        write_tile(
            &mut ram,
            0x8010,
            [(0b1000_0000, 0b1000_0000), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)],
        );

        ram[0xFE00] = 24; // Y: screen row 8
        ram[0xFE01] = 16; // X: screen col 8
        ram[0xFE02] = 1; // tile index
        ram[0xFE03] = 0x20; // X flip

        let mut screen = blank_screen();
        render_frame(&ram, &mut screen);

        assert_eq!(
            pixel(&screen, 8, 8),
            GB_COLORS[0],
            "left edge should be empty after X flip"
        );
        assert_eq!(
            pixel(&screen, 15, 8),
            GB_COLORS[3],
            "right edge should contain mirrored pixel"
        );
    }

    #[test]
    fn lcdc_bit4_selects_unsigned_tile_addressing() {
        // LCDC bit 4 = 1 → tile data at 0x8000 + index*16 (unsigned).
        // Tile index 1 → 0x8010.
        let mut ram = blank_ram();
        ram[0xFF47] = 0xE4; // BGP: identity mapping
        ram[0xFF40] = 0x91; // LCDC: display on, BG on, bit 4 set
        ram[0x9800] = 1; // tile map slot 0 → tile index 1
        write_tile(
            &mut ram,
            0x8010,
            [
                (0xFF, 0xFF),
                (0, 0),
                (0, 0),
                (0, 0),
                (0, 0),
                (0, 0),
                (0, 0),
                (0, 0),
            ],
        );
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
