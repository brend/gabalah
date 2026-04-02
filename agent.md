# Gabalah Agent Notes

## Project Purpose
- `gabalah` is a Nintendo Game Boy emulator written in Rust.
- Current focus is CPU/instruction groundwork, with rendering/UI scaffolding present.

## Tech Stack
- Rust 2021 (`cargo`, `rustc`)
- Window/render loop: `winit`, `pixels`, `winit_input_helper`

## Repository Layout
- `src/main.rs`: CLI entrypoint, ROM loading, starts app loop.
- `src/app.rs`: window/event loop and current pixel-world rendering.
- `src/cpu/`: CPU core.
  - `cpu.rs`: `Cpu` state, fetch/decode/execute cycle.
  - `map.rs`: opcode map (large, central for instruction coverage).
  - `ops.rs`: instruction/mnemonic/operand model and flag bitmasks.
  - `alu.rs`: arithmetic/logic/flag helpers.
- `src/memory/ram.rs`: registers and RAM model.
- `src/renderer.rs`: tile/pixel decoding helpers (not fully wired into app loop).
- `tests/ops.rs`: instruction behavior tests.
- `tests/cpu.rs`: register tests.

## How To Run
- Run emulator with ROM:
  - `cargo run -- path/to/rom.gb`
- Run tests:
  - `cargo test`

## Current State (as of 2026-04-02)
- Tests pass (`tests/ops.rs` + `tests/cpu.rs`, 13 tests total).
- CPU execution exists with many implemented instructions.
- App loop currently renders a manually toggled pixel buffer; CPU output is not yet driving display.
- `HALT` and `STOP` are not implemented (`todo!()` in `src/cpu/cpu.rs`).
- CB-prefixed opcode handling is not implemented (`0xCB` currently mapped to `Invalid`).
- Several instruction forms involving HL auto-inc/auto-dec addressing are marked TODO in `src/cpu/map.rs`.

## Important Code Notes
- `Registers::new()` starts `pc` at `0x100`.
- ROM loading currently copies bytes starting at `0x100` in RAM (`Ram::load_rom`), not `0x0000`.
- `src/main.rs` contains a legacy `moin()` path with `unimplemented!()` and unreachable code warnings; normal execution goes through `app::run_loop`.

## Working Guidance For Future Changes
- Prefer adding/adjusting tests in `tests/ops.rs` when touching instruction behavior.
- Keep opcode metadata (bytes/cycles) aligned with execute semantics.
- If introducing new memory-mapped behavior (PPU/timers/input/interrupts), centralize address semantics to avoid scattering magic addresses.
- Avoid breaking existing public module exports in `src/lib.rs` (`cpu`, `memory`) used by integration tests.
