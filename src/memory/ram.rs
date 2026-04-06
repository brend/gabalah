const ROM_SIZE: usize = 32 * 1024;

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
    /// interrupt master enable
    pub ime: bool,
}

impl Registers {
    /// returns an instance of Registers with every register set to 0
    pub fn new() -> Registers {
        Registers {
            a: 0x01,
            f: 0xB0,
            b: 0x00,
            c: 0x13,
            d: 0x00,
            e: 0xD8,
            h: 0x01,
            l: 0x4D,
            sp: 0xFFFE,
            pc: 0x0100,
            ime: false,
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
        self.f = lo(value) & 0xF0;
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
    /// Bits 4-5 of the last write to 0xFF00: selects which button group to read
    joypad_select: u8,
    /// Active-high bitmask of pressed action buttons (bit 0=A, 1=B, 2=Select, 3=Start)
    pub action_buttons: u8,
    /// Active-high bitmask of pressed direction buttons (bit 0=Right, 1=Left, 2=Up, 3=Down)
    pub direction_buttons: u8,
    /// Internal 16-bit counter backing DIV (0xFF04); DIV register = high byte
    div_counter: u32,
    /// Accumulated cycles since last TIMA increment
    tima_counter: u32,
}

impl Ram {
    /// Returns an instance of Ram with post-boot DMG0 hardware register state
    pub fn new() -> Ram {
        let mut ram = Ram {
            cells: [0; RAM_SIZE],
            joypad_select: 0x30,
            action_buttons: 0,
            direction_buttons: 0,
            div_counter: 0x183A,
            tima_counter: 0,
        };
        ram.cells[0xFF07] = 0xF8; // TAC: upper bits set, timer disabled
        ram.cells[0xFF0F] = 0xE1; // IF: VBlank + upper unused bits set
        ram.cells[0xFF40] = 0x91; // LCDC: display on, BG enabled, unsigned tile data
        ram.cells[0xFF47] = 0xFC; // BGP: shades 3,3,2,0
        ram.cells[0xFF48] = 0xFF; // OBP0
        ram.cells[0xFF49] = 0xFF; // OBP1
        ram
    }

    /// Loads a ROM into memory
    pub fn load_rom(&mut self, rom: Vec<u8>) {
        assert!(rom.len() <= ROM_SIZE, "maximum ROM size exceeded");
        let base_addr = 0x0000;
        for (i, byte) in rom.iter().enumerate() {
            self.cells[base_addr + i] = *byte;
        }
    }

    /// Sets the byte at the specified address to the specified value
    pub fn write_byte(&mut self, address: Addr, value: u8) {
        if address.0 == 0xFF00 {
            self.joypad_select = value & 0x30;
            return;
        }
        if address.0 == 0xFF04 {
            self.div_counter = 0;
            self.cells[0xFF04] = 0;
            return;
        }
        if address.0 == 0xFF46 {
            let src_base = (value as usize) << 8;
            let source_slice = &self.cells[src_base..src_base + 160].to_vec();
            self.cells[0xFE00..0xFE00 + 160].copy_from_slice(source_slice);
            return;
        }
        self.cells[address.0 as usize] = value;
    }

    /// Sets the word at the specified address to the specified value
    pub fn write_word(&mut self, address: Addr, value: u16) {
        self.cells[address.0 as usize] = lo(value);
        self.cells[address.0.wrapping_add(1) as usize] = hi(value);
    }

    /// Retrieves the byte at the specified address
    pub fn read_byte(&self, address: Addr) -> u8 {
        if address.0 == 0xFF00 {
            let mut lo = 0x0Fu8; // all buttons not pressed (active low)
            if self.joypad_select & 0x20 == 0 {
                lo &= !self.action_buttons;
            }
            if self.joypad_select & 0x10 == 0 {
                lo &= !self.direction_buttons;
            }
            return 0xC0 | (self.joypad_select & 0x30) | (lo & 0x0F);
        }
        if address.0 == 0xFF04 {
            return (self.div_counter >> 8) as u8;
        }
        self.cells[address.0 as usize]
    }

    /// Advances timer state by `cycles` CPU cycles. Returns true if TIMA overflowed.
    pub fn tick(&mut self, cycles: u32) -> bool {
        self.div_counter = self.div_counter.wrapping_add(cycles);
        self.cells[0xFF04] = (self.div_counter >> 8) as u8;

        let tac = self.cells[0xFF07];
        if tac & 0x04 == 0 {
            return false;
        }

        let threshold = match tac & 0x03 {
            0 => 1024u32,
            1 => 16,
            2 => 64,
            _ => 256,
        };

        self.tima_counter += cycles;
        let mut overflow = false;
        while self.tima_counter >= threshold {
            self.tima_counter -= threshold;
            let tima = self.cells[0xFF05];
            if tima == 0xFF {
                self.cells[0xFF05] = self.cells[0xFF06];
                overflow = true;
            } else {
                self.cells[0xFF05] = tima + 1;
            }
        }
        overflow
    }

    pub fn read_word(&self, address: Addr) -> u16 {
        word(
            self.cells[address.0.wrapping_add(1) as usize],
            self.cells[address.0 as usize],
        )
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.cells
    }
}
