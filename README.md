# Gabalah

An emulator for the Nintendo Game Boy.

## Prerequisites

In order to build and run Gabalah, all you need is 
a working Rust toolchain, specifically `cargo` and `rustc`.

Refer to [the official site of the Rust programming language](https://www.rust-lang.org) 
to learn more.

## Build and run

Gabalah expects a path to a ROM file as its single command line argument.

``` sh
$ cargo run path/to/some_rom.gb
```

### Controls

- D-Pad: Arrow keys
- A: `Z`
- B: `X`
- Select: Right Shift
- Start: Enter
- Debug frame dump: `F9`

### Debug Frame Dumps

Press `F9` while the emulator is running to dump the current frame and PPU state
to `debug_dumps/`:

- `frame_XXXX.ppm` — rendered frame image
- `frame_XXXX.txt` — key LCD/interrupt registers
- `frame_XXXX_vram.bin` — VRAM snapshot (`0x8000..0x9FFF`)
- `frame_XXXX_oam.bin` — OAM snapshot (`0xFE00..0xFE9F`)

## Running the included tests

Run the included tests with

``` sh
$ cargo test
```

## Emulation Accuracy

During development of Gabalah, I'll try to use [blargg's test roms](https://github.com/L-P/blargg-test-roms/tree/master) to improve 
the accuracy of the emulation. 
