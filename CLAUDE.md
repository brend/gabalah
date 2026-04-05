# Gabalah — Game Boy Emulator

A Nintendo Game Boy (DMG) emulator written in Rust. The goal is a cycle-accurate emulator capable of running real ROM files.

## Project Structure

```
src/
  main.rs          — entry point: reads ROM from argv, creates CPU, launches app loop
  app.rs           — winit/pixels event loop; drives CPU at ~70,224 cycles/frame
  lib.rs           — re-exports cpu and memory modules for integration tests
  cpu/
    mod.rs         — re-exports Cpu, Mnemonic, Instruction, Location, flag bitmasks
    cpu.rs         — Cpu struct, step(), execute(), execute_cb()
    ops.rs         — Instruction, Mnemonic, Operand, Location types
    alu.rs         — arithmetic/logic operations (add, sub, rotate, flags trait)
    map.rs         — builds the full opcode HashMap<u8, Instruction>
  memory/
    mod.rs         — re-exports Ram, Registers, Addr
    ram.rs         — Registers, Ram (flat 64KB), Addr, word/hi/lo helpers
  renderer.rs      — BG tile renderer; reads LCDC/BGP/SCX/SCY, decodes 2bpp tiles, outputs 160×144 RGBA
  err.rs           — placeholder error type (currently unused)
```

## Key Architectural Facts

- **CPU**: `Cpu::step()` fetches an opcode, delegates to `execute()` (base set) or `execute_cb()` (CB-prefixed), and returns the cycle count consumed.
- **Memory**: `Ram` is a flat `[u8; 65536]`. ROM is loaded at `0x0100` (should be `0x0000` — known bug, tracked in `STATUS.md`).
- **Registers**: `pc` initialises to `0x0100`. `ime` (interrupt master enable) is a bool field on `Registers`.
- **Cycles**: `Cpu::total_cycles` accumulates over the session. The app loop runs ~70,224 cycles per frame (`CYCLES_PER_FRAME` in `app.rs`).
- **Display**: `Emulator::draw()` calls `renderer::render_frame()`, which renders the BG tile layer at 160×144 scaled 3×. Frame rate is capped at ~59.7 fps via `FRAME_DURATION`.
- **renderer.rs**: Full BG renderer — reads tile map (LCDC bit 3), tile data (LCDC bit 4, signed/unsigned), SCX/SCY scroll, BGP palette. Window and sprite layers not yet implemented.

## Build & Test

```bash
cargo build
cargo test
cargo run -- path/to/rom.gb
```

28 integration tests live in `tests/cpu.rs` and `tests/ops.rs`. All pass. Keep them green.

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
