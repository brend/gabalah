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

### Graphics Backend Configuration

Gabalah reads optional graphics settings from `config.json` in the project root.

```json
{
  "graphics_backend": "wgpu_shader",
  "shader": {
    "scanline_strength": 0.22,
    "curvature": 0.10,
    "mode": "palette_mutation",
    "color_intensity": 0.82
  }
}
```

Supported values for `"graphics_backend"`:

- `"pixels"`: existing `pixels` presentation path
- `"wgpu_shader"`: WGSL shader-based backend (scanline + CRT curvature + color modes)

Supported values for `"shader.mode"`:

- `"classic"`: original CRT-like look
- `"prism"`: chromatic split + rainbow tinting
- `"aurora"`: animated neon channel remap
- `"palette_mutation"`: quantized-tone procedural palette remap (wildest mode)

`"shader.color_intensity"` controls how strong the non-classic color modes are (range `0.0..1.5`).
For `palette_mutation`, a readable starting range is usually `0.65..0.95`.

Press `R` while running to reload shader settings from `config.json` without restarting.
Changing `"graphics_backend"` still requires restarting the app.

### Controls

- D-Pad: Arrow keys
- A: `Z`
- B: `X`
- Select: Right Shift
- Start: Enter
- Reload graphics config: `R`
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
