# Project Status

Last updated: 2026-04-06

## What Works

### CPU
- Full base instruction set decoded and executed via `opcode_map` (`HashMap<u8, Instruction>`)
- CB-prefixed instructions (rotate/shift, BIT/RES/SET) via `execute_cb()`
- 8-bit and 16-bit arithmetic/logic with flag handling
- Stack operations: PUSH, POP, CALL, RET, RETI, RST
- Conditional control flow (JR cc, JP cc, CALL cc, RET cc)
- IME handling with delayed EI activation
- HALT wakeup on pending interrupt (HALT bug still stubbed)
- Post-boot DMG0 register initialization

### Memory / IO
- ROM loaded at `0x0000`; PC starts at `0x0100`
- ROM write-protection enabled after ROM load (`0x0000..0x7FFF` writes ignored)
- Echo RAM mirroring (`0xE000..0xFDFF` <-> `0xC000..0xDDFF`)
- Unusable area behavior (`0xFEA0..0xFEFF`: reads `0xFF`, writes ignored)
- Joypad register (`0xFF00`) with group-select semantics
- Timer registers (`DIV/TIMA/TMA/TAC`) with cycle-based ticking and overflow detection
- DMA transfer (`0xFF46`) copies 160 bytes into OAM
- Serial capture stub (`0xFF01/0xFF02`) with IF serial bit request
- LY write reset (`0xFF44`) and STAT writable-bit masking (`0xFF41`)

### Interrupts
- IF/IE register flow wired into CPU dispatch
- Interrupt vectors dispatched for bits 0..4 when `IME && (IF & IE) != 0`
- VBlank interrupt requested at line 144
- Timer interrupt requested on TIMA overflow
- Joypad interrupt requested on newly pressed key
- STAT mode/coincidence bits are updated, but STAT IRQ generation is currently disabled

### App / Display
- winit event loop with `pixels` backend (160×144, scaled 3×)
- Frame pacing near 59.7 FPS (`FRAME_DURATION` based on 70,224 cycles/frame)
- Per-frame CPU stepping with LCD timing progression
- Debug frame dump hotkey (`F9`) writes frame + LCD/VRAM/OAM artifacts to `debug_dumps/`

### PPU / Renderer
- Background renderer with SCX/SCY scroll
- Window renderer with WX/WY and `WX-7` positioning behavior
- OBJ renderer (8×8 baseline) with transparency and OBP0 mapping
- LCDC gating:
  - LCD off (`bit 7 = 0`) renders blank frame
  - BG/Window master gate (`bit 0`) controls BG+Window drawing
  - OBJ enable (`bit 1`) controls sprite drawing
- Tile addressing supports both signed (`0x8800` region) and unsigned (`0x8000`) modes

## Known Gaps

### PPU accuracy
- OBJ attributes not implemented yet (priority, X/Y flip, OBP1 selection)
- 8×16 sprite mode not implemented
- STAT interrupt generation disabled pending tighter timing accuracy
- No per-scanline register latching/render pipeline (mid-scanline effects not emulated)

### Cartridge / hardware
- No cartridge abstraction (MBC1/MBC3/MBC5 not implemented)
- No save RAM persistence (`.sav`)
- STOP remains a no-op
- HALT bug behavior not fully implemented
- Audio (APU) not implemented

## Test Coverage

| Area | Tests | Status |
|---|---|---|
| CPU core ops | 28 (`tests/ops.rs`) | passing |
| Memory/IO/timer/joypad/DMA | 23 (`tests/cpu.rs`) | passing |
| Renderer (BG/window/OBJ baseline) | 10 (`src/renderer.rs`) | passing |
| Interrupt conformance ROMs | partial/manual | in progress |
| PPU conformance ROMs | partial/manual | in progress |
