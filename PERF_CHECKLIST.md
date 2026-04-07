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

- [x] **#4 — Sprite clip-rect pre-computation** (`render_obj`)
  Pre-clamp row/col loop ranges from `tile_y`/`tile_x` before entering the loops.
  Eliminates the per-pixel `contains()` bounds checks entirely; partially off-screen sprites
  now iterate only over the visible slice.

## CPU (`src/cpu/`)

- [~] **#5 — Inline `Location`/`Operand` read/write methods** (`src/cpu/ops.rs`)
  Skipped: `cpu_step_nop` measures 8.8 ns — the compiler is already inlining in release mode.
  No measurable headroom.

- [~] **#6 — Collapse ALU flag writes to a single bitmask assignment** (`src/cpu/alu.rs`)
  Skipped: ALU path is only 1.6 ns above NOP (~10.4 ns total). Even eliminating it entirely
  saves ~0.11 ms/frame — well inside budget. Compiler likely already merges the branches.

- [x] **#7 — Replace `ime_activation_countdown: i32` with a `bool`** (`src/cpu/core.rs`)
  Replaced with `pending_ime: bool`. `EI` sets it true; the next `execute()` fires it,
  clears it, and enables `ime`. Eliminates the decrement + two comparisons per instruction.

## Infrastructure

- [x] **#8 — Add criterion benchmarks**
  `benches/renderer.rs`: `render_frame` with a realistic RAM state (scroll, tile data, one sprite).
  `benches/cpu.rs`: `cpu_step_nop` (dispatch overhead) and `cpu_step_alu` (ALU + flag path).
  Run with `cargo bench`; HTML reports land in `target/criterion/`.
