# Gabalah Agent Notes

## Project Purpose
- `gabalah` is a Rust Game Boy emulator project.
- The strongest part of the codebase today is the CPU instruction core and its tests.
- The emulator shell exists, but the app is not yet running a real emulation loop or presenting Game Boy video output.

## Tech Stack
- Rust 2021
- Rendering/windowing: `pixels`, `winit`, `winit_input_helper`
- Logging: `log`, `env_logger`
- Test style: Rust unit/integration tests via `cargo test`

## Repository Layout
- `src/main.rs`: CLI entrypoint, ROM loading, creates the CPU, starts the window loop.
- `src/app.rs`: app/window loop, temporary interactive pixel demo, current Windows graphics workaround.
- `src/cpu/cpu.rs`: CPU state, instruction execution, CB-prefixed opcode execution, cycle accounting.
- `src/cpu/map.rs`: main opcode map for the non-CB instruction set.
- `src/cpu/ops.rs`: instruction, operand, addressing, and flag definitions.
- `src/cpu/alu.rs`: ALU helpers and flag math.
- `src/memory/ram.rs`: RAM/register storage and ROM loading.
- `src/renderer.rs`: tile decoding helpers for turning VRAM-like data into pixels; not yet integrated into the app loop.
- `tests/ops.rs`: CPU/instruction behavior tests.
- `tests/cpu.rs`: register packing/masking tests.

## Current State (as of 2026-04-03)
- `cargo check` passes.
- `cargo test` passes with 30 tests total:
  - 28 in `tests/ops.rs`
  - 2 in `tests/cpu.rs`
- The CPU can execute a substantial subset of the base instruction set.
- CB-prefixed instructions are implemented directly in `Cpu::execute_cb`, including rotate/shift/bit/set/reset behavior and the register-vs-`(HL)` cycle split.
- The main app still renders a manually toggled pixel buffer rather than stepping the CPU and drawing emulator output.
- The Windows app path now avoids the DX12 backend in `pixels` because DX12 was producing invalid render-target state errors on some Windows machines.

## What Is Working Well
- CPU execution has a reasonable skeleton: opcode fetch, decode, execute, cycle tracking, PC updates, stack behavior, and conditional cycle selection.
- The opcode map is broad enough to support meaningful progress on CPU bring-up.
- Integration tests cover several high-value behaviors:
  - immediate loads/arithmetic
  - relative jumps and conditional timing
  - stack/call/rst behavior
  - auto-increment/decrement HL forms
  - high-memory load/store helpers
  - CB instruction behavior and timing
- Register masking for `AF` is tested correctly.

## Biggest Gaps
- `HALT` and `STOP` are placeholders in `src/cpu/cpu.rs`; they currently do nothing beyond normal control flow.
- Interrupt handling is incomplete. `EI` deferred enable and `RETI` exist, but there is no interrupt request/service pipeline, no IF/IE modeling, and no HALT/interrupt interaction.
- ROM loading is not cartridge-accurate yet:
  - `Ram::load_rom` copies ROM bytes starting at `0x0100`
  - real cartridge ROM should occupy `0x0000..`
  - there is no boot ROM/cartridge abstraction or banking support
- The app is not yet an emulator frontend:
  - `Cpu` is passed into `run_loop` but not used
  - no frame stepping
  - no timing synchronization
  - no joypad input mapping
  - no PPU-backed framebuffer
- `src/renderer.rs` looks like an early tile/VRAM visualization helper, but it is disconnected from emulation state and currently unused.
- The project has many compiler warnings, mostly from unfinished code paths and currently unused modules/functions.

## Important Implementation Notes
- `Registers::new()` initializes `pc` to `0x0100`.
- `Ram::load_rom()` currently writes ROM bytes starting at `0x0100`, which is convenient for current tests but not representative of real Game Boy memory layout.
- `main.rs` requires a ROM path, loads it into CPU memory, and then launches the app loop.
- The current UI behavior in `app.rs` is effectively a sandbox/demo for pixel rendering and mouse toggling, not emulation output.
- Because the binary path is still mostly UI scaffolding, many core CPU methods appear as dead code in the `bin` target even though integration tests exercise them.

## Assessment
- The project is in a promising prototype state, not yet a playable emulator.
- The CPU core has enough structure and test coverage to justify continuing from the current architecture.
- The highest-risk area is not opcode breadth anymore; it is the missing system integration layer:
  - correct memory map
  - interrupts/timers
  - PPU/frame generation
  - emulator loop wiring
- Right now the codebase is strongest as a tested CPU sandbox plus an unrelated rendering shell.

## Most Important Next Steps
1. Fix memory/cart mapping so ROM is loaded into the correct address range and introduce a cartridge abstraction before more emulator subsystems depend on the current fake layout.
2. Implement interrupt plumbing properly: IF/IE registers, interrupt service, `HALT` semantics, and timer/joypad hooks.
3. Connect the emulator core to the app loop:
   - step the CPU
   - accumulate cycles
   - produce a framebuffer
   - render actual emulator output instead of the manual pixel toy world
4. Define a minimal PPU path that can turn memory state into a stable 160x144 framebuffer, even if it starts as background-only output.
5. Expand tests around system behavior, especially:
   - interrupt timing
   - `HALT`/`STOP`
   - memory-map expectations
   - representative opcode coverage gaps
6. Reduce warning noise in touched areas so regressions are easier to notice while the emulator loop is being built.

## Suggested Short-Term Execution Plan
- Milestone 1: Correct cartridge/memory loading
  - introduce `Cartridge`
  - load ROM at `0x0000`
  - keep tests green by updating assumptions explicitly
- Milestone 2: Make the CPU loop real
  - wire CPU stepping into `app.rs`
  - define per-frame cycle budget
  - remove or isolate the temporary click-to-toggle pixel demo
- Milestone 3: First visible emulation output
  - feed a framebuffer from emulator state into `pixels`
  - use `src/renderer.rs` only if it meaningfully helps; otherwise replace it with a simpler framebuffer path
- Milestone 4: Hardware behavior
  - interrupts
  - timers
  - joypad
  - PPU accuracy improvements

## Guidance For Future Changes
- When changing instruction semantics, add or adjust tests first in `tests/ops.rs`.
- Prefer fixing architectural mismatches early, especially ROM loading and memory layout, before building timers/PPU/input on top.
- Keep cycle accounting explicit whenever adding CPU/system behavior.
- Avoid deepening the current temporary rendering path unless it directly helps wire the real emulator framebuffer.
