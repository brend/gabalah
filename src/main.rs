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

/// An 8-bit unsigned integer
#[derive(PartialEq, PartialOrd, Clone, Copy, Default)]
pub struct Byte(u8);

impl fmt::Debug for Byte {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Byte {
    fn zero() -> Byte {
        Byte(0)
    }
}

/// A 16-bit unsigned integer
#[derive(PartialEq, PartialOrd, Clone, Copy, Default)]
pub struct Word(u16);

impl fmt::Debug for Word {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A struct representing the Game Boy's CPU registers
#[derive(Default, Debug)]
pub struct Registers {
    /// accumulator A
    a: Byte,
    /// general purpose register B
    b: Byte,
    /// general purpose register D
    d: Byte,
    /// general purpose register H
    h: Byte,
    /// flags register F
    f: Byte,
    /// general purpose register C
    c: Byte,
    /// general purpose register E
    e: Byte,
    /// general purpose register L
    l: Byte,
    /// stack pointer
    sp: Word,
    /// program counter
    pc: Word,
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
    cells: [Byte; RAM_SIZE],
}

impl Ram {
    /// Returns an instance of zeroed Ram
    fn new() -> Ram {
        Ram { cells: [Byte::zero(); RAM_SIZE] }
    }

    /// Sets the byte at the specified address to the specified value
    fn set(&mut self, address: Addr, value: Byte) {
        self.cells[address.0 as usize] = value;
    }

    fn get(&self, address: Addr) -> Byte {
        self.cells[address.0 as usize]
    }
}

fn main()  {
    let mut r = Ram::new();
    let a = Addr(0);
    r.set(a, Byte(17));
    println!("{:?}", r.get(a))
}