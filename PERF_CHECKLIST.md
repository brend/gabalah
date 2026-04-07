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

- [ ] **#3 — Dirty-tile tracking + persistent screen buffer**

  **Why it was reverted**: A first attempt added a `TileCache` (pre-decoded lo/hi bytes per
  tile) and dirty bits in `Ram`, but benchmarked 7% *slower* in steady-state (27.1 µs vs
  25.3 µs). The cache replaced two flat VRAM reads with a bit-check + Box pointer dereference
  + two struct reads — more work when no tiles are dirty. Without skipping screen writes,
  there is no net saving.

  **What is actually needed** — two pieces together:

  **Part A — VRAM write tracking in `Ram`** (`src/memory/ram.rs`)
  - Add `pub tile_dirty: [u64; 6]` (384 bits, one per tile in `0x8000–0x97FF`).
    Initialise to `[!0u64; 6]` (all dirty on boot).
  - In `write_byte`, before the final `self.cells[addr] = value`:
    ```rust
    if (0x8000..=0x97FF).contains(&addr) {
        let tile = (addr - 0x8000) >> 4;
        self.tile_dirty[tile >> 6] |= 1u64 << (tile & 63);
    }
    ```
  - Expose via `as_slice_and_dirty(&mut self) -> (&[u8], &mut [u64; 6])` to let the
    renderer borrow both without a borrow-checker conflict.

  **Part B — persistent screen buffer + skip unchanged pixels** (`src/app.rs`, `src/renderer.rs`)
  - Move the screen buffer ownership into `Emulator` as a `prev_screen: Vec<u8>` field so
    it persists between frames.
  - In `render_frame`, after computing a pixel's shade, compare against `prev_screen` at
    that offset and only call `copy_from_slice` when the value changed. For a static
    background this skips ~100% of screen writes.
  - After rendering, `copy prev_screen → graphics frame_mut()` in `draw()`.
  - The `TileCache` (pre-decoded row bytes) is optional once screen-write skipping is in
    place; omit it unless profiling shows the VRAM reads themselves are the bottleneck.

  **Regression guard** (re-use or adapt from the reverted branch):
  - `tile_cache_reflects_vram_write_after_first_render`: write tile, render, overwrite
    tile + set dirty bit, render again, assert new colour appears.
  - `cached_render_produces_same_output_as_fresh_render`: two consecutive renders with no
    writes produce identical output.
  - Fuzz/stress: N iterations of random VRAM writes, compare dirty-tracking render output
    against a forced full-redraw of the same RAM state byte-for-byte.
  - Benchmark: `render_frame` with zero VRAM writes should measurably improve over the
    25.3 µs baseline; a frame with all tiles written should stay close to it.

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
