# Project Status

Last updated: 2026-04-11

## What Works

### CPU
- Full base instruction set decoded and executed via static opcode table (`[Instruction; 256]`)
- CB-prefixed instructions (rotate/shift, BIT/RES/SET) via `execute_cb()`
- 8-bit and 16-bit arithmetic/logic with flag handling
- Stack operations: PUSH, POP, CALL, RET, RETI, RST
- Conditional control flow (JR cc, JP cc, CALL cc, RET cc)
- IME handling with delayed EI activation
- HALT wakeup on pending interrupt
- HALT bug behavior implemented (`IME=0` + pending interrupt does not halt; next opcode fetch is duplicated)
- Post-boot DMG0 register initialization

### Memory / IO
- ROM loaded at `0x0000`; PC starts at `0x0100`
- Cartridge header metadata parsed on ROM load and exposed through CPU/RAM accessors
- Runtime cartridge abstraction (`Cartridge`) owns ROM + mapper state and is wired into RAM
- CPU memory access is encapsulated behind `Cpu` facade methods (no direct field access outside CPU internals)
- ROM write-protection enabled after ROM load (`0x0000..0x7FFF` writes ignored)
- Echo RAM mirroring (`0xE000..0xFDFF` <-> `0xC000..0xDDFF`)
- Unusable area behavior (`0xFEA0..0xFEFF`: reads `0xFF`, writes ignored)
- Joypad register (`0xFF00`) with group-select semantics
- Timer registers (`DIV/TIMA/TMA/TAC`) with cycle-based ticking and overflow detection
- DMA transfer (`0xFF46`) copies 160 bytes into OAM
- Serial capture stub (`0xFF01/0xFF02`) with IF serial bit request
- LY write reset (`0xFF44`) and STAT writable-bit masking (`0xFF41`)
- Basic MBC1 ROM banking (lower/upper ROM bank bits + mode select for fixed/switchable windows)

### Interrupts
- IF/IE register flow wired into CPU dispatch
- Interrupt vectors dispatched for bits 0..4 when `IME && (IF & IE) != 0`
- VBlank interrupt requested at line 144
- Timer interrupt requested on TIMA overflow
- Joypad interrupt requested on newly pressed key
- STAT IRQ generation enabled for mode transitions and LY==LYC edge

### App / Display
- winit event loop with pluggable graphics backends (160×144, scaled 3×)
- `graphics_backend` selection via `config.json` (`pixels` or `wgpu_shader`)
- `pixels` backend path retained behind the graphics abstraction
- `wgpu_shader` backend with runtime WGSL shader library loaded from `./shaders`
- Bundled runtime shaders: `crt.wgsl` (CRT-only), `funk_spectrum.wgsl` (non-CRT color remix), `no_effect.wgsl` (passthrough)
- Runtime shader cycling hotkeys: `Q` (previous), `E` (next)
- Active shader persistence via `shader.active_file` in `config.json`
- Frame pacing near 59.7 FPS (`FRAME_DURATION` based on 70,224 cycles/frame)
- Per-frame CPU stepping with LCD timing progression
- Runtime shader config hot-reload via `R` (re-reads shader fields and rescans `./shaders`)
- Debug frame dump hotkey (`F9`) writes frame + LCD/VRAM/OAM artifacts to `debug_dumps/`

### PPU / Renderer
- Background renderer with SCX/SCY scroll
- Window renderer with WX/WY and `WX-7` positioning behavior
- OBJ renderer with transparency, X/Y flip, OBP0/OBP1 palette select, priority (attribute bit 7), and 8×16 mode
- LCDC gating:
  - LCD off (`bit 7 = 0`) renders blank frame
  - BG/Window master gate (`bit 0`) controls BG+Window drawing
  - OBJ enable (`bit 1`) controls sprite drawing
- Tile addressing supports both signed (`0x8800` region) and unsigned (`0x8000`) modes
- Scanline-latched BG/Window register rendering (`SCX/SCY/WX/WY/LCDC/BGP`) for per-line split effects

## Known Gaps

### PPU accuracy
- LCD mode transitions are still coarse at instruction granularity; not yet sliced at dot-level boundaries
- Future improvement: dot-level mode transition slicing for tighter STAT edge timing and latch points

### UI/backend limitations
- Backend type changes still require restart (runtime reload applies backend options only)

### Cartridge / hardware
- Cartridge mapper abstraction exists (`RomOnly`, `Mbc1` runtime state)
- Mapper coverage is partial (MBC1 ROM banking only; MBC3/MBC5 not implemented)
- MBC1 external RAM banking/enable behavior not yet implemented
- Header checksum/global checksum are parsed but not yet enforced for ROM rejection
- No save RAM persistence (`.sav`)
- STOP remains a no-op
- Audio (APU) not implemented

## Test Coverage

| Area | Tests | Status |
|---|---|---|
| CPU core ops | 34 (`tests/ops.rs`) | passing |
| Memory/IO/timer/joypad/DMA/MBC1 | 28 (`tests/cpu.rs`) | passing |
| Cartridge header parser + runtime mapper behavior | 11 (`tests/cartridge.rs`) | passing |
| Architecture boundary guard (no external `cpu.memory` access) | 1 (`tests/architecture.rs`) | passing |
| Renderer (BG/window/OBJ + scanline latch path) | 14 (`src/renderer.rs`) | passing |
| Graphics config/backend parsing | 10 (`src/config.rs`, `src/ui/mod.rs`) | passing |
| WGSL shader contract/discovery tests | 5 (`src/ui/wgpu_shader_backend.rs`) | passing |
| Interrupt conformance ROMs | partial/manual | in progress |
| PPU conformance ROMs | partial/manual | in progress |
