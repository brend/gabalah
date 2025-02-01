mod app;

/// An 8-bit unsigned integer
#[derive(PartialEq, PartialOrd, Clone, Copy, Default)]
pub struct Byte(u8);

impl std::fmt::Debug for Byte {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

impl std::fmt::Debug for Word {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

const RAM_SIZE: usize = 64 * 1024;

/// A struct representing the Game Boy's random-access memory
#[derive(Debug)]
struct Ram {
    cells: [Byte; RAM_SIZE],
}

impl Ram {
    fn new() -> Ram {
        Ram { cells: [Byte::zero(); RAM_SIZE] }
    }
}

fn main()  {
    let r = Ram::new();
    println!("{:?}", r)
}