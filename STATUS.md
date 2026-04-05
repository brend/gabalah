# Project Status

Last updated: 2026-04-05

## What Works

### CPU
- Full base instruction set decoded and executed via `opcode_map` (`HashMap<u8, Instruction>`)
- CB-prefixed instructions (rotate/shift, BIT/RES/SET) via `execute_cb()`
- All flag operations (Z, N, H, C) through the `Flags` trait on `u8`
- 16-bit register pairs AF, BC, DE, HL with get/set helpers
- Stack operations: PUSH, POP, CALL, RET, RETI, RST
- Conditional jumps and calls (JR cc, JP cc, CALL cc, RET cc)
- High-memory addressing (LDH / 0xFF00+n)
- IME (interrupt master enable): EI schedules activation after 1 instruction delay; DI clears immediately
- Cycle counting: `step()` returns cycles consumed; `total_cycles` accumulates
- 28 passing integration tests

### App / Display
- winit event loop with `pixels` backend (160×144, scaled 3×)
- `Emulator::step_frame()` drives the CPU for ~70,224 cycles per frame
- Placeholder framebuffer (solid Game Boy off-colour `#9BBC0F`)
- Escape / window-close exits cleanly
- Windows: DX12 avoided in favour of Vulkan/GL backend

## Known Bugs / Gaps

### Critical (blocks real ROMs)
- ~~**ROM loads at `0x0100` instead of `0x0000`**~~ — fixed. ROM now loads at `0x0000`; PC still initialises to `0x0100` (correct post-boot handoff point).
- **No cartridge abstraction** — `Ram::load_rom()` copies bytes directly into the flat array with no MBC (Memory Bank Controller) support. Even simple ROMs with only a ROM-only mapper need the header parsed.

### Hardware not yet implemented
- **PPU (Pixel Processing Unit)** — `renderer.rs` has a tile decoder stub but nothing is connected. No scanline rendering, no VBLANK, no frame output from emulation.
- **Interrupt system** — IF (`0xFF0F`) and IE (`0xFFFF`) registers are plain memory bytes. No interrupt dispatch pipeline. VBLANK, Timer, Joypad interrupts not fired.
- **Timer** — DIV (`0xFF04`), TIMA (`0xFF05`), TMA (`0xFF06`), TAC (`0xFF07`) not implemented.
- **HALT** — currently a no-op; should suspend CPU until an interrupt is pending.
- **STOP** — currently a no-op.
- **Joypad** — no input mapped to `0xFF00`.
- **Serial port** — not implemented (`0xFF01`/`0xFF02`).
- **Audio (APU)** — not started.

### Minor / polish
- ~~`sp` initialises to `0x0000` instead of the correct post-boot value of `0xFFFE`.~~ — fixed.
- TODO in `cpu.rs:69`: SP-relative 16-bit loads (`LD (nn), SP`) need verification.
- `renderer.rs`, `err.rs` contain dead code — will be cleaned up when PPU work begins.
- No frame-rate limiter; runs as fast as the host allows.

## Test Coverage

| Area | Tests | Status |
|---|---|---|
| Arithmetic (ADD, SUB, ADC, SBC) | ✓ | passing |
| Logical (AND, OR, XOR, CP) | ✓ | passing |
| Loads (LD, LDH, LD (HL±), LDHL) | ✓ | passing |
| Jumps (JR, JP, JRC) | ✓ | passing |
| Stack (PUSH, POP, CALL, RET, RST) | ✓ | passing |
| CB-prefix (rotate, shift, BIT, RES, SET) | ✓ | passing |
| Flags (SCF, CCF, CPL, DAA) | ✓ | passing |
| PPU | — | not started |
| Interrupts | — | not started |
| Timer | — | not started |
