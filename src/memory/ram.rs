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
            f: 0x00,
            b: 0xFF,
            c: 0x13,
            d: 0x00,
            e: 0xC1,
            h: 0x84,
            l: 0x03,
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
    rom_loaded: bool,
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
    /// Bytes captured from serial transfers (0xFF01 at each 0xFF02 write with bit 7 set)
    pub serial_output: Vec<u8>,
}

impl Ram {
    /// Returns an instance of Ram with post-boot DMG0 hardware register state
    pub fn new() -> Ram {
        let mut ram = Ram {
            cells: [0; RAM_SIZE],
            rom_loaded: false,
            joypad_select: 0x30,
            action_buttons: 0,
            direction_buttons: 0,
            div_counter: 0x183A,
            tima_counter: 0,
            serial_output: Vec::new(),
        };
        ram.cells[0xFF07] = 0xF8; // TAC: upper bits set, timer disabled
        ram.cells[0xFF0F] = 0xE1; // IF: VBlank + upper unused bits set
        ram.cells[0xFF40] = 0x91; // LCDC: display on, BG enabled, unsigned tile data
        ram.cells[0xFF41] = 0x80; // STAT: upper bit set, mode/coincidence initialized to 0
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
        self.rom_loaded = true;
    }

    /// Sets the byte at the specified address to the specified value
    pub fn write_byte(&mut self, address: Addr, value: u8) {
        let addr = address.0 as usize;
        if address.0 == 0xFF00 {
            self.joypad_select = value & 0x30;
            return;
        }
        if address.0 == 0xFF04 {
            self.div_counter = 0;
            self.cells[0xFF04] = 0;
            return;
        }
        if address.0 == 0xFF02 && value & 0x81 == 0x81 {
            self.serial_output.push(self.cells[0xFF01]);
            self.cells[0xFF02] = value & 0x7F;
            self.cells[0xFF0F] |= 0x08;
            return;
        }
        if address.0 == 0xFF46 {
            let src_base = (value as usize) << 8;
            let source_slice = &self.cells[src_base..src_base + 160].to_vec();
            self.cells[0xFE00..0xFE00 + 160].copy_from_slice(source_slice);
            return;
        }
        if address.0 == 0xFF41 {
            // STAT: bits 0-2 are read-only (mode + coincidence), bits 3-6 writable, bit 7 always set.
            let ro = self.cells[0xFF41] & 0x07;
            self.cells[0xFF41] = 0x80 | (value & 0x78) | ro;
            return;
        }
        if address.0 == 0xFF44 {
            // LY resets to zero on write.
            self.cells[0xFF44] = 0;
            return;
        }
        // Cartridge ROM area. After a cartridge is loaded, writes are ignored.
        if self.rom_loaded && addr <= 0x7FFF {
            return;
        }
        // Echo RAM mirrors C000-DDFF.
        if (0xE000..=0xFDFF).contains(&addr) {
            self.cells[addr - 0x2000] = value;
            return;
        }
        // Unusable memory area.
        if (0xFEA0..=0xFEFF).contains(&addr) {
            return;
        }
        self.cells[addr] = value;
    }

    /// Sets the word at the specified address to the specified value
    pub fn write_word(&mut self, address: Addr, value: u16) {
        self.cells[address.0 as usize] = lo(value);
        self.cells[address.0.wrapping_add(1) as usize] = hi(value);
    }

    /// Retrieves the byte at the specified address
    pub fn read_byte(&self, address: Addr) -> u8 {
        let addr = address.0 as usize;
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
        if (0xE000..=0xFDFF).contains(&addr) {
            return self.cells[addr - 0x2000];
        }
        if (0xFEA0..=0xFEFF).contains(&addr) {
            return 0xFF;
        }
        self.cells[addr]
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

    /// Sets LY directly (used by PPU timing logic).
    pub fn set_ly_raw(&mut self, ly: u8) {
        self.cells[0xFF44] = ly;
    }

    /// Sets STAT directly (used by PPU timing logic).
    pub fn set_stat_raw(&mut self, stat: u8) {
        self.cells[0xFF41] = 0x80 | (stat & 0x7F);
    }
}
