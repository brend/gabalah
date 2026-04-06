# Development Checklist

Items are ordered by dependency. Check off items as they are done and keep `STATUS.md` in sync.

---

## Phase 1 — System Integration

- [x] Wire CPU `step()` into app loop
- [x] Make `step()` return cycle count
- [x] Size output to 160×144 (Game Boy native)
- [x] Run ~70,224 cycles per frame
- [x] Load ROM at `0x0000`, start PC at `0x0100`
- [x] Correct post-boot SP (`0xFFFE`)
- [x] Initialize post-boot DMG0 CPU and key IO defaults
- [ ] Parse cartridge header metadata (title/type/ROM size/RAM size)
- [ ] Add explicit cartridge abstraction type
- [x] Add frame limiter (~59.7 fps)

---

## Phase 2 — PPU (Pixel Processing Unit)

- [x] Background tile rendering (tile map + tile data + SCX/SCY)
- [x] Window rendering (WX/WY, `WX-7`, map select)
- [x] Baseline sprite rendering (8×8, transparency, OBP0)
- [x] LY scanline progression (`0xFF44`, 0..153)
- [x] VBlank request at LY=144
- [x] Basic STAT mode/coincidence state updates (`0xFF41`)
- [x] Respect LCDC display enable (`bit 7`) and BG/window gate (`bit 0`) in renderer
- [ ] Enable STAT IRQ generation with accurate edge behavior
- [ ] Implement sprite attribute bits (priority, x/y flip, OBP1 select)
- [ ] Implement 8×16 OBJ mode (LCDC bit 2)
- [ ] Add scanline-accurate render timing/latching for mid-frame register changes

---

## Phase 3 — Interrupts & Timer

- [x] IF register request flow (`0xFF0F`)
- [x] IE mask handling (`0xFFFF`)
- [x] Interrupt dispatch to vectors (VBlank/LCD/Timer/Serial/Joypad)
- [x] Timer tick model (DIV/TIMA/TMA/TAC)
- [x] Timer overflow reload + interrupt request
- [x] DIV write resets counter/register
- [x] HALT wakeup on pending interrupt
- [ ] HALT bug exact behavior (IME=0 + pending interrupt)
- [ ] STOP behavior

---

## Phase 4 — Joypad, Serial, DMA

- [x] Joypad select/read behavior at `0xFF00`
- [x] Map keyboard input to GB buttons
- [x] Request joypad interrupt on new button press
- [x] Serial transfer capture stub (`0xFF01/0xFF02`)
- [x] OAM DMA transfer via `0xFF46`

---

## Phase 5 — Memory Map & Cartridge

- [x] Ignore ROM writes after ROM load (`0x0000..0x7FFF`)
- [x] Echo RAM mirror (`0xE000..0xFDFF`)
- [x] Unusable area semantics (`0xFEA0..0xFEFF`)
- [ ] ROM-only cartridge abstraction (`type 0x00`) via mapper layer
- [ ] MBC1 support
- [ ] MBC3 support
- [ ] MBC5 support
- [ ] Battery-backed save RAM persistence

---

## Phase 6 — Audio (APU)

- [ ] Channel 1 (square + sweep)
- [ ] Channel 2 (square)
- [ ] Channel 3 (wave)
- [ ] Channel 4 (noise)
- [ ] Frame sequencer
- [ ] Mixer/output backend

---

## Tooling / Validation

- [x] Unit tests for renderer paths (BG/window/sprite baseline)
- [x] Unit tests for memory map edge behavior
- [ ] Add automated ROM acceptance harness in CI
- [ ] Pass broader mooneye/blargg PPU timing suites
- [ ] Pass `dmg-acid2`
