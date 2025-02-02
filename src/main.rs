mod app;

use std::fmt;

/// Error datatype
#[derive(Debug)]
enum Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ERROR!?")
    }
}

impl std::error::Error for Error {}

/// A struct representing the Game Boy's CPU registers
#[derive(Default, Debug)]
pub struct Registers {
    /// accumulator A
    a: u8,
    /// general purpose register B
    b: u8,
    /// general purpose register D
    d: u8,
    /// general purpose register H
    h: u8,
    /// flags register F
    f: u8,
    /// general purpose register C
    c: u8,
    /// general purpose register E
    e: u8,
    /// general purpose register L
    l: u8,
    /// stack pointer
    sp: u16,
    /// program counter
    pc: u16,
}

impl Registers {
    /// returns an instance of Registers with every register set to 0
    fn new() -> Registers {
        Registers::default()
    }
}

/// The size of the Game Boy's RAM in bytes
const RAM_SIZE: usize = 64 * 1024;

/// A struct representing a 16-bit address into the Game Boy's RAM
#[derive(Copy, Clone, Debug)]
struct Addr(u16);

/// A struct representing the Game Boy's random-access memory
#[derive(Debug)]
struct Ram {
    cells: [u8; RAM_SIZE],
}

impl Ram {
    /// Returns an instance of zeroed Ram
    fn new() -> Ram {
        Ram { cells: [0; RAM_SIZE] }
    }

    /// Sets the byte at the specified address to the specified value
    fn set(&mut self, address: Addr, value: u8) {
        self.cells[address.0 as usize] = value;
    }

    fn get(&self, address: Addr) -> u8 {
        self.cells[address.0 as usize]
    }
}

fn main()  {
    let mut r = Ram::new();
    let a = Addr(0);
    r.set(a, 17);
    println!("{:?}", r.get(a))
}