# Gabalah — Game Boy Emulator

A Nintendo Game Boy (DMG) emulator written in Rust. The goal is a cycle-accurate emulator capable of running real ROM files.

## Project Structure

```
src/
  main.rs          — entry point: reads ROM from argv, creates CPU, launches app loop
  app.rs           — winit event loop; drives CPU at ~70,224 cycles/frame and feeds active graphics backend
  config.rs        — `config.json` loading for graphics backend and shader options
  ui/
    mod.rs         — graphics backend trait + backend kind/options parsing + factory
    pixels_backend.rs — `pixels` backend adapter
    wgpu_shader_backend.rs — `wgpu` presentation backend with WGSL shader pass
    shaders/crt.wgsl — WGSL shader source (curvature + scanline effect)
  lib.rs           — re-exports cpu and memory modules for integration tests
  cpu/
    mod.rs         — re-exports Cpu, Mnemonic, Instruction, Location, flag bitmasks
    cpu.rs         — Cpu struct, step(), execute(), execute_cb()
    ops.rs         — Instruction, Mnemonic, Operand, Location types
    alu.rs         — arithmetic/logic operations (add, sub, rotate, flags trait)
    map.rs         — builds the full opcode HashMap<u8, Instruction>
  memory/
    mod.rs         — re-exports Ram, Registers, Addr
    ram.rs         — Registers, Ram, IO handlers, timer/DMA/joypad behavior, memory map rules
  renderer.rs      — DMG renderer (BG + window + baseline OBJ), decodes 2bpp tiles to 160×144 RGBA
```

## Key Architectural Facts

- **CPU**: `Cpu::step()` fetches an opcode, delegates to `execute()` (base set) or `execute_cb()` (CB-prefixed), and returns the cycle count consumed.
- **Memory**: ROM is loaded at `0x0000`; PC initialises to `0x0100`. After ROM load, writes to `0x0000..0x7FFF` are ignored. Echo RAM (`0xE000..0xFDFF`) mirrors work RAM and unusable area (`0xFEA0..0xFEFF`) reads as `0xFF`.
- **Registers**: Post-boot DMG0 state: AF=`0x0100`, BC=`0xFF13`, DE=`0x00C1`, HL=`0x8403`, SP=`0xFFFE`, PC=`0x0100`. `ime` (interrupt master enable) is a bool field on `Registers`, initialised `false`.
- **Cycles**: `Cpu::total_cycles` accumulates over the session. The app loop runs ~70,224 cycles per frame (`CYCLES_PER_FRAME` in `app.rs`).
- **Display**: `Emulator::draw()` calls `renderer::render_frame()` at 160×144 (scaled 3× by the window layer). `ui::GraphicsBackend` handles presentation (`pixels` or `wgpu_shader`), selected from `config.json`. Frame rate is capped near ~59.7 fps.
- **Shader config reload**: Pressing `R` reloads shader options from `config.json` at runtime. Backend type changes still require restart.
- **PPU timing**: `app.rs` tracks LY/mode progression from CPU cycles, updates STAT mode/coincidence bits, and requests VBlank. STAT IRQ generation is intentionally disabled for now due to timing inaccuracy.
- **renderer.rs**: Implements BG, window, and baseline sprite drawing. Missing sprite attributes (priority/flip/OBP1) and 8×16 mode.

## Build & Test

```bash
cargo build
cargo test
cargo run -- path/to/rom.gb
```

69 tests currently pass across `tests/cpu.rs`, `tests/ops.rs`, renderer unit tests in `src/renderer.rs`, config/backend parsing tests, and a WGSL syntax smoke test. Keep them green.

## Coding Conventions

- Rust 2021 edition. `#![deny(clippy::all)]` and `#![forbid(unsafe_code)]` in app.rs — honour these elsewhere too.
- Prefer editing existing files over creating new ones.
- Do not add speculative abstractions or future-proofing. Build what the current task requires.
- Do not add comments unless the logic is genuinely non-obvious.

## Reference: Game Boy Hardware Constants

| Item | Value |
|---|---|
| CPU clock | 4,194,304 Hz |
| Cycles/frame | 70,224 (154 lines × 456 dots) |
| Screen | 160×144 pixels |
| ROM base address | `0x0000` |
| Stack starts | `0xFFFE` |
| IF register | `0xFF0F` |
| IE register | `0xFFFF` |
| LCDC register | `0xFF40` |
| VRAM | `0x8000`–`0x9FFF` |
| OAM | `0xFE00`–`0xFE9F` |
| High RAM | `0xFF80`–`0xFFFE` |

See `STATUS.md` for current progress and `CHECKLIST.md` for the ordered task list.
