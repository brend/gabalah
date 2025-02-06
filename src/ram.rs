type Bytes = Vec<u8>;

/// A struct representing the Game Boy's CPU registers
#[derive(Default, Debug)]
pub struct Registers {
    /// accumulator A
    pub a: u8,
    /// general purpose register B
    pub b: u8,
    /// general purpose register D
    pub d: u8,
    /// general purpose register H
    pub h: u8,
    /// flags register F
    pub f: u8,
    /// general purpose register C
    pub c: u8,
    /// general purpose register E
    pub e: u8,
    /// general purpose register L
    pub l: u8,
    /// stack pointer
    pub sp: u16,
    /// program counter
    pub pc: u16,
}

impl Registers {
    /// returns an instance of Registers with every register set to 0
    pub fn new() -> Registers {
        Registers::default()
    }

    /// sets the value of the 16-bit BC register
    pub fn set_bc(&mut self, lo: u8, hi: u8) {
        self.c = lo;
        self.b = hi;
    }
}

/// The size of the Game Boy's RAM in bytes
pub const RAM_SIZE: usize = 64 * 1024;

/// Make a Word (u16) out of two consecutive bytes (u8) in RAM
fn word(l: u8, h: u8) -> u16 {
    (h as u16) << 8 | l as u16
}

/// A struct representing a 16-bit address into the Game Boy's RAM
#[derive(Copy, Clone, Debug)]
pub struct Addr(pub u16);

impl Addr {
    /// Returns the byte address following this one
    pub fn next(&self) -> Option<Addr> {
        if self.0 < u16::MAX {
            Some(Addr(self.0 + 1))
        } else {
            None
        }
    }

    /// Increases this byte address by one
    pub fn inc(&mut self) {
        assert!(self.0 < u16::MAX);
        self.0 += 1;
    }

    /// Creates an address from two bytes
    pub fn from_bytes(addr_bytes: Vec<u8>) -> Addr {
        debug_assert!(addr_bytes.len() == 2);
        Addr(word(addr_bytes[0], addr_bytes[1]))
    }
}

/// A struct representing the Game Boy's random-access memory
#[derive(Debug)]
pub struct Ram {
    cells: [u8; RAM_SIZE],
}

impl Ram {
    /// Returns an instance of zeroed Ram
    pub fn new() -> Ram {
        Ram { cells: [0; RAM_SIZE] }
    }

    /// Sets the byte at the specified address to the specified value
    pub fn set(&mut self, address: Addr, value: u8) {
        self.cells[address.0 as usize] = value;
    }

    /// Sets the word at the specified address to the specified value
    pub fn set_word(&mut self, address: Addr, values: &Bytes) {
        debug_assert!(values.len() == 2);
        debug_assert!(address.0 < u16::MAX);
        self.cells[address.0 as usize] = values[0];
        self.cells[address.0 as usize + 1] = values[1]
    }

    /// Retrieves the byte at the specified address
    pub fn get(&self, address: Addr) -> u8 {
        self.cells[address.0 as usize]
    }

    /// Retrieves the word at the specified address
    pub fn get_word(&self, address: Addr) -> Option<u16> {
        let next = address.next()?;
        Some(word(self.get(address), self.get(next)))
    }
}
