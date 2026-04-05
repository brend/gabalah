# Development Checklist

Items are ordered by dependency — each group generally requires the previous to be complete.
Check off items as they are done and update `STATUS.md` accordingly.

---

## Phase 1 — System Integration (current)

- [x] Wire CPU `step()` into the app event loop
- [x] Make `step()` return cycle count
- [x] Size window to 160×144 (Game Boy native)
- [x] Run ~70,224 CPU cycles per frame
- [x] Fix ROM load address: load at `0x0000`, set PC to `0x0100`
- [x] Fix SP initial value: `0xFFFE`
- [x] Initialise hardware registers to post-boot DMG values (AF=0x01B0, BC=0x0013, DE=0x00D8, HL=0x014D, SP=0xFFFE, PC=0x0100)
- [ ] Parse cartridge header (title, cartridge type, ROM/RAM size)
- [ ] Implement ROM-only mapper (type `0x00`) — simplest case, no banking
- [x] Add frame-rate limiter (~59.7 fps)

---

## Phase 2 — PPU (Pixel Processing Unit)

- [x] Background tile rendering (BG tile map + tile data, scrolling SCX/SCY)
- [x] Palette register: BGP (`0xFF47`) decoded to 4-shade colour output
- [x] Connect PPU framebuffer to `Emulator::draw()` in `app.rs`
- [x] Scanline counter: increment LY (`0xFF44`) each scanline, wrap at 154
- [ ] Implement LCDC register (`0xFF40`) read/write effects fully (bit 7 BG enable, etc.)
- [ ] Implement STAT register (`0xFF41`) and mode transitions (OAM scan → drawing → HBlank → VBlank)
- [ ] LYC=LY coincidence flag and STAT interrupt
- [ ] Window layer rendering (WX/WY)
- [ ] Sprite (OAM) rendering (8×8 and 8×16 modes, priority, flip)
- [ ] Palette registers: OBP0 (`0xFF48`), OBP1 (`0xFF49`) (for sprites)

---

## Phase 3 — Interrupts & Timer

- [ ] Implement IF register (`0xFF0F`) — set bits when interrupts are requested
- [ ] Implement IE register (`0xFFFF`) — mask which interrupts are enabled
- [ ] Interrupt dispatch: when `ime` is true and `IF & IE != 0`, call the appropriate vector
  - VBLANK: `0x0040`
  - LCD STAT: `0x0048`
  - Timer: `0x0050`
  - Serial: `0x0058`
  - Joypad: `0x0060`
- [ ] Request VBLANK interrupt at end of line 144
- [ ] Implement HALT: suspend `step()` until an interrupt is pending
- [ ] Implement timer registers: DIV (`0xFF04`), TIMA (`0xFF05`), TMA (`0xFF06`), TAC (`0xFF07`)
- [ ] Timer overflow fires Timer interrupt and reloads TIMA from TMA
- [ ] DIV increments at 16,384 Hz (every 256 CPU cycles); write to DIV resets it

---

## Phase 4 — Joypad & I/O

- [ ] Implement joypad register (`0xFF00`): bit 4 selects direction keys, bit 5 selects action keys
- [ ] Map winit key events to Game Boy buttons (A, B, Start, Select, Up, Down, Left, Right)
- [ ] Request Joypad interrupt on button press
- [ ] Implement serial port stub (`0xFF01`/`0xFF02`) — minimum: acknowledge writes without crashing

---

## Phase 5 — Memory Bank Controllers

- [ ] MBC1 (used by many early games): ROM banking up to 2 MB, RAM banking up to 32 KB
- [ ] MBC3 (adds RTC): ROM/RAM banking + real-time clock registers
- [ ] MBC5 (most common in later games): ROM banking up to 8 MB, RAM banking up to 128 KB
- [ ] External RAM save/load (battery-backed RAM to `.sav` file)

---

## Phase 6 — Audio (APU)

- [ ] Square wave channel 1 (with sweep)
- [ ] Square wave channel 2
- [ ] Wave channel (channel 3, custom waveform)
- [ ] Noise channel (channel 4)
- [ ] Frame sequencer (controls length, envelope, sweep)
- [ ] Mix channels and output to audio backend (rodio or cpal)

---

## Ongoing / Maintenance

- [ ] Resolve TODO at `cpu.rs:69`: verify SP-relative 16-bit LD behaviour
- [ ] Remove dead code in `err.rs` once superseded
- [ ] Pass Blargg's CPU instruction tests (`cpu_instrs.gb`)
- [ ] Pass Blargg's instruction timing tests (`instr_timing.gb`)
- [ ] Pass dmg-acid2 PPU conformance test
