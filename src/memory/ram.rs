pub fn word(hi: u8, lo: u8) -> u16 {
    ((hi as u16) << 8) | lo as u16
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
    pub fn af(&self) -> u16 {
        word(self.a, self.f)
    }

    /// returns the value of the 16-bit BC register
    pub fn bc(&self) -> u16 {
        word(self.b, self.c)
    }

    /// returns the value of the 16-bit HL register
    pub fn hl(&self) -> u16 {
        word(self.h, self.l)
    }

    /// returns the value of the 16-bit DE register
    pub fn de(&self) -> u16 {
        word(self.d, self.e)
    }

    /// sets the value of the 16-bit AF register
    pub fn set_af(&mut self, value: u16) {
        self.a = hi(value);
        self.f = lo(value);
    }

    /// sets the value of the 16-bit BC register
    pub fn set_bc(&mut self, value: u16) {
        self.b = hi(value);
        self.c = lo(value);
    }

    /// sets the value of the 16-bit DE register
    pub fn set_de(&mut self, value: u16) {
        self.d = hi(value);
        self.e = lo(value);
    }

    /// sets the value of the 16-bit HL register
    pub fn set_hl(&mut self, value: u16) {
        self.h = hi(value);
        self.l = lo(value);
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
    pub fn write_byte(&mut self, address: Addr, value: u8) {
        self.cells[address.0 as usize] = value;
    }

    /// Sets the word at the specified address to the specified value
    pub fn write_word(&mut self, address: Addr, value: u16) {
        debug_assert!(address.0 < u16::MAX);
        self.cells[address.0 as usize] = lo(value);
        self.cells[address.0 as usize + 1] = hi(value);
    }

    /// Retrieves the byte at the specified address
    pub fn read_byte(&self, address: Addr) -> u8 {
        self.cells[address.0 as usize]
    }

    pub fn read_word(&self, address: Addr) -> u16 {
        debug_assert!(address.0 < u16::MAX);
        word(self.cells[address.0 as usize + 1], self.cells[address.0 as usize])
    }
}
