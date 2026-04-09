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

### Cartridge Metadata

On ROM load, Gabalah parses the Game Boy cartridge header (`0x0100..0x014F`) and stores metadata
for later use (title, licensee, CGB/SGB flags, cartridge type, ROM/RAM bank counts, destination,
version, and both checksum fields).

Current access points:

- `Cpu::cartridge_header() -> Option<&CartridgeHeader>`
- `Ram::cartridge_header() -> Option<&CartridgeHeader>`

This currently parses and exposes checksum fields; checksum enforcement/validation is not yet
wired into ROM load rejection logic.

### Graphics Backend Configuration

Gabalah reads optional graphics settings from `config.json` in the project root.

```json
{
  "graphics_backend": "wgpu_shader",
  "window": {
    "scale": 3.0
  },
  "controls": {
    "joypad": {
      "up": "up",
      "down": "down",
      "left": "left",
      "right": "right",
      "a": "z",
      "b": "x",
      "select": "right_shift",
      "start": "enter"
    },
    "hotkeys": {
      "reload_graphics_config": "r",
      "previous_shader": "q",
      "next_shader": "e",
      "debug_frame_dump": "f9",
      "exit": "escape"
    }
  },
  "debug_dump": {
    "enabled": true,
    "output_directory": "debug_dumps"
  },
  "shader": {
    "directory": "shaders",
    "scanline_strength": 0.22,
    "curvature": 0.10,
    "mode": "palette_mutation",
    "color_intensity": 0.82,
    "active_file": "crt.wgsl"
  }
}
```

Supported values for `"graphics_backend"`:

- `"pixels"`: existing `pixels` presentation path
- `"wgpu_shader"`: WGSL runtime shader-library backend

`"window.scale"` controls the initial window size multiplier. It must be a finite number greater
than `0`. If omitted, Gabalah uses `3.0`.

Supported values for `"shader.mode"`:

- `"classic"`
- `"prism"`
- `"aurora"`
- `"palette_mutation"`

`"shader.mode"` and `"shader.color_intensity"` are passed as uniforms to the active shader.
How they are interpreted depends on that shader file.

Runtime WGSL shaders are loaded from `"shader.directory"` and default to `./shaders` in the
project root. Every file must provide
`vs_main`/`fs_main` and the expected texture/sampler/uniform bindings.
`"shader.active_file"` selects the preferred shader filename and is updated when cycling shaders.

`"controls"` is optional. If omitted, Gabalah keeps the current defaults shown above. Supported key
names include letters, digits, arrows, `enter`, `escape`, `tab`, `space`, `left_shift`,
`right_shift`, `left_ctrl`, `right_ctrl`, `left_alt`, `right_alt`, and `f1` through `f12`.

`"debug_dump.enabled"` controls whether the dump hotkey can queue a capture.
`"debug_dump.output_directory"` controls where frame dumps are written.

Bundled runtime shaders:

- `crt.wgsl`: dedicated CRT pass (curvature + scanlines + phosphor/flicker)
- `funk_spectrum.wgsl`: aggressive non-CRT color remap
- `heart_pixels.wgsl`: renders each source pixel as a tiny heart shape
- `no_effect.wgsl`: passthrough (use this to effectively disable shader effects)
- `wiggle_ripple.wgsl`: per-pixel ripple displacement with a soft, pleasing wobble

Press `R` while running to reload shader settings, debug dump settings, and rescan the configured
shader directory without restarting.
Changing `"graphics_backend"` still requires restarting the app.

### Controls

- D-Pad: configurable, defaults to Arrow keys
- A: configurable, defaults to `Z`
- B: configurable, defaults to `X`
- Select: configurable, defaults to Right Shift
- Start: configurable, defaults to Enter
- Reload graphics config: configurable, defaults to `R`
- Previous shader: configurable, defaults to `Q`
- Next shader: configurable, defaults to `E`
- Debug frame dump: configurable, defaults to `F9`
- Exit: configurable, defaults to `Escape`

### Debug Frame Dumps

Press `F9` while the emulator is running to dump the current frame and PPU state
to the configured debug dump directory:

- `frame_XXXX.ppm` — rendered frame image
- `frame_XXXX.txt` — key LCD/interrupt registers
- `frame_XXXX_vram.bin` — VRAM snapshot (`0x8000..0x9FFF`)
- `frame_XXXX_oam.bin` — OAM snapshot (`0xFE00..0xFE9F`)

## Running the included tests

Run the included tests with

``` sh
$ cargo test
```

Cartridge parser tests can also be run directly with:

``` sh
$ cargo test --test cartridge
```

## Emulation Accuracy

During development of Gabalah, I'll try to use [blargg's test roms](https://github.com/L-P/blargg-test-roms/tree/master) to improve 
the accuracy of the emulation. 
