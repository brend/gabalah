/// Create a 16-bit word from two 8-bit values
fn word(lo: u8, hi: u8) -> u16 {
    (hi as u16) << 8 | lo as u16
}

/// A binary value that can be either 8 or 16 bits
#[derive(Debug, Clone, Copy)]
pub enum Bytes {
    /// An 8-bit value
    One(u8),
    /// A 16-bit value
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

impl From<bool> for Bytes {
    fn from(value: bool) -> Self {
        if value {
            Bytes::One(1)
        } else {
            Bytes::One(0)
        }
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

    pub fn word(&self) -> Option<u16> {
        match self {
            Bytes::Two(value) => Some(*value),
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
        Registers {
            pc: 0x100,
            ..Default::default()
        }
    }

    /// returns the value of the 16-bit AF register
    pub fn af(&self) -> Bytes {
        Bytes::from_bytes(self.f, self.a)
    }

    /// returns the value of the 16-bit BC register
    pub fn bc(&self) -> Bytes {
        Bytes::from_bytes(self.c, self.b)
    }

    /// returns the value of the 16-bit HL register
    pub fn hl(&self) -> Bytes {
        Bytes::from_bytes(self.l, self.h)
    }

    /// returns the value of the 16-bit DE register
    pub fn de(&self) -> Bytes {
        Bytes::from_bytes(self.e, self.d)
    }

    /// sets the value of the 16-bit BC register
    pub fn set_bc(&mut self, bytes: &Bytes) {
        self.c = bytes.lo();
        self.b = bytes.hi();
    }

    /// sets the value of the 16-bit HL register
    pub fn set_hl(&mut self, bytes: &Bytes) {
        self.l = bytes.lo();
        self.h = bytes.hi();
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
