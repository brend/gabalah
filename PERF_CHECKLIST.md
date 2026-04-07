# Performance Optimization Checklist

## Renderer (`src/renderer.rs`)

- [x] **#1 — Bit shifts for tile coordinate math** (`render_bg`, `render_window`, `render_obj`)
  Replace `/8` → `>>3` and `%8` → `&7` for tile column/row and pixel offset calculations.
  23,040 iterations per frame; compiler may already optimize these for power-of-two divisors,
  but explicit shifts are unambiguous.

- [x] **#2 — Hoist tile address + row bytes out of inner pixel loop** (`render_bg`, `render_window`)
  `bg_y`/`win_y`, `tile_row`, and `pixel_y` don't vary with `screen_x` — moved outside inner loop.
  `lo`/`hi` tile row bytes now cached per tile column (recomputed only on `tile_col` change, i.e.
  every ~8 pixels instead of every pixel). Eliminates ~22,720 redundant VRAM reads per frame
  on a fully scrolled background.

- [ ] **#3 — Dirty-tile tracking to skip unchanged tiles entirely**
  Track VRAM writes and only re-render tiles that have changed. Large win for games with
  mostly-static backgrounds. Requires write-tracking in `Ram` and a tile-dirty bitfield.

- [ ] **#4 — Sprite clip-rect pre-computation** (`render_obj`)
  Currently checks `(0..WIDTH).contains(&screen_x)` per pixel. Pre-clamp the col loop bounds
  based on `tile_x` to eliminate per-pixel branches.

## CPU (`src/cpu/`)

- [ ] **#5 — Inline `Location`/`Operand` read/write methods** (`src/cpu/ops.rs`)
  Each instruction dispatches through two nested match chains (Operand → Location).
  Add `#[inline]` to hot methods; verify with `cargo asm` that release builds already inline them.

- [ ] **#6 — Collapse ALU flag writes to a single bitmask assignment** (`src/cpu/alu.rs`)
  Each `set_zero()` / `set_carry()` etc. is a conditional branch. Replace with a single
  expression that builds the full flags byte and writes it once per operation.

- [ ] **#7 — Replace `ime_activation_countdown: i32` with a `bool`** (`src/cpu/core.rs`)
  The countdown is decremented and checked every instruction. A `bool` + a one-shot flag
  eliminates the arithmetic entirely.

## Infrastructure

- [ ] **#8 — Add criterion benchmarks**
  Add benchmarks for `render_frame`, `Cpu::step`, and memory read/write to get baseline numbers
  before further optimization. Without profiling data the ordering of remaining items is a guess.
