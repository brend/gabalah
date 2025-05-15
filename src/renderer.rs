#![deny(clippy::all)]
#![forbid(unsafe_code)]

pub const WIDTH: u32 = 256;
pub const HEIGHT: u32 = 256;

/// ram: vec![0; 0x10000]
pub fn read_pixels(ram: &[u8]) -> Vec<u8> {
    let lcdc = ram[0xFF40];
    let window_tile_map_base = if (lcdc & 0x40) != 0 { 0x9C00 } else { 0x9800 };

    let mut pixpixs = vec![];
    for i in 0..WIDTH * HEIGHT {
        pixpixs.push(if i % 7 == 1 { 128 } else { 64 });
    }

    for base in [0x9800, window_tile_map_base] {
        for tile_map_index in 0..(32 * 32) {
            // read tile index from the tile map
            let tile_index = ram[base + tile_map_index];

            // draw the tile at the appropriate position
            let x = (tile_map_index % 32) as usize * 8;
            let y = (tile_map_index / 32) as usize * 8;
            draw_tile(&ram, &mut pixpixs, tile_index, x, y);
        }
    }

    pixpixs
}

fn draw_tile(ram: &[u8], pixpixs: &mut Vec<u8>, tile_index: u8, x: usize, y: usize) {
    // let tile_address = 0x8000 + (tile_index as usize * 16);
    let tile_address = 0x9000i32 + (tile_index as i8 as i32 * 16);
    let tile_address = tile_address as usize;
    let tile_bytes = &ram[tile_address..(tile_address + 16)];
    // a row is two bytes of data, comprising 8 pixels
    for row_index in 0..8 {
        let lo = tile_bytes[row_index * 2];
        let hi = tile_bytes[row_index * 2 + 1];
        for column_index in 0..8 {
            let bit_index = 7 - column_index; // leftmost pixel is bit 7
            let lo_bit = (lo >> bit_index) & 1;
            let hi_bit = (hi >> bit_index) & 1;
            let palette_index = (hi_bit << 1) | lo_bit;

            set_pixel(
                pixpixs,
                x + column_index,
                y + row_index,
                palette_index as u8,
            );
        }
    }
}

fn set_pixel(pixpixs: &mut Vec<u8>, x: usize, y: usize, palette_index: u8) {
    let color = 255 - (palette_index * 85) as u8;
    let pixel_index = x + y * WIDTH as usize;
    debug_assert!(
        pixel_index < pixpixs.len(),
        "Pixel index out of bounds: {}",
        pixel_index
    );
    // Set the pixel color in the pixel buffer
    pixpixs[pixel_index] = color;
}
