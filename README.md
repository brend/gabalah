# Gabalah

An emulator for the Nintendo Game Boy.

## Prerequisites

In order to build and run Gabalah, all you need is 
a working Rust toolchain, specifically `cargo` and `rustc`.

Refer to [the official site of the Rust programming language](https://www.rust-lang.org) 
to learn more.

## Build and run

Gabalah expects a path to a ROM file as its single command line argument.

``` sh
$ cargo run path/to/some_rom.gb
```

## Running tests

Run the included tests with

``` sh
$ cargo test
```