
#[derive(Debug, Clone, Copy)]
pub enum Bytes {
    One(u8),
    Two(u16),
}

impl From<u8> for Bytes {
    fn from(value: u8) -> Self {
        Bytes::One(value)
    }
}

impl From<u16> for Bytes {
    fn from(value: u16) -> Self {
        Bytes::Two(value)
    }
}

impl Bytes {
    pub fn from_bytes(lo: u8, hi: u8) -> Self {
        Bytes::Two((hi as u16) << 8 | lo as u16)
    }

    pub fn single(&self) -> Option<u8> {
        match self {
            Bytes::One(value) => Some(*value),
            _ => None,
        }
    }

    pub fn is_one(&self) -> bool {
        match self {
            Bytes::One(_) => true,
            _ => false,
        }
    }

    pub fn is_two(&self) -> bool {
        match self {
            Bytes::Two(_) => true,
            _ => false,
        }
    }

    pub fn lo(&self) -> u8 {
        match self {
            Bytes::Two(value) => lo(*value),
            _ => panic!("Expected a 16-bit value"),
        }
    }

    pub fn hi(&self) -> u8 {
        match self {
            Bytes::Two(value) => hi(*value),
            _ => panic!("Expected a 16-bit value"),
        }
    }
}

/// The contents of the flags register F 
/// in structured form
#[derive(Debug, Clone, Copy)]
pub struct Flags {
    pub zero: bool,
    pub subtraction: bool,
    pub half_carry: bool,
    pub carry: bool,
}

/// The Game Boy's CPU registers
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
    pub fn set_bc(&mut self, bytes: &Bytes) {
        self.c = bytes.lo();
        self.b = bytes.hi();
    }

    /// returns the flags register
    pub fn flags(&self) -> Flags {
        Flags {
            zero: self.f & 0b1000_0000 != 0,
            subtraction: self.f & 0b0100_0000 != 0,
            half_carry: self.f & 0b0010_0000 != 0,
            carry: self.f & 0b0001_0000 != 0,
        }
    }

    /// sets the flags register
    pub fn set_flags(&mut self, flags: Flags) {
        self.f = 0;
        if flags.zero {
            self.f |= 0b1000_0000;
        }
        if flags.subtraction {
            self.f |= 0b0100_0000;
        }
        if flags.half_carry {
            self.f |= 0b0010_0000;
        }
        if flags.carry {
            self.f |= 0b0001_0000;
        }
    }
}

/// The size of the Game Boy's RAM in bytes
pub const RAM_SIZE: usize = 64 * 1024;

/// Return the high byte of the provided word
pub fn hi(word: u16) -> u8 {
    (word >> 8) as u8
}

/// Return the low byte of the provided word
pub fn lo(word: u16) -> u8 {
    (word & 0x00FF) as u8
}

/// A 16-bit address into the Game Boy's RAM
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
}

impl From<Bytes> for Addr {
    fn from(bytes: Bytes) -> Self {
        Addr((bytes.hi() as u16) << 8 | bytes.lo() as u16)
    }
}

/// The Game Boy's random-access memory
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
        debug_assert!(values.is_two());
        debug_assert!(address.0 < u16::MAX);
        self.cells[address.0 as usize] = values.lo();
        self.cells[address.0 as usize + 1] = values.hi();
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
