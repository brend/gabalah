use crate::cartridge::{CartridgeHeader, CartridgeType};
use log::warn;

const ROM_BANK_SIZE: usize = 16 * 1024;
const SWITCHABLE_ROM_START: usize = 0x4000;
const SWITCHABLE_ROM_END: usize = 0x7FFF;
const VISIBLE_ROM_END: usize = 0x7FFF;

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

impl Default for Ram {
    fn default() -> Self {
        Self::new()
    }
}

/// The Game Boy's random-access memory
#[derive(Debug)]
pub struct Ram {
    cells: [u8; RAM_SIZE],
    rom_loaded: bool,
    cartridge_rom: Vec<u8>,
    cartridge_header: Option<CartridgeHeader>,
    /// MBC1 register: lower 5 bits of ROM bank number (0 maps to 1).
    mbc1_rom_bank_low5: u8,
    /// MBC1 register: upper 2 bits (ROM high bits in mode 0, RAM bank in mode 1).
    mbc1_bank_high2: u8,
    /// MBC1 mode register (0 = ROM banking mode, 1 = RAM banking mode).
    mbc1_mode: u8,
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
            cartridge_rom: Vec::new(),
            cartridge_header: None,
            mbc1_rom_bank_low5: 1,
            mbc1_bank_high2: 0,
            mbc1_mode: 0,
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
        self.cartridge_header = match CartridgeHeader::from_bytes(&rom) {
            Ok(header) => Some(header),
            Err(err) => {
                warn!("Failed to parse cartridge header metadata: {err}");
                None
            }
        };
        self.cartridge_rom = rom;
        self.cells[0x0000..=VISIBLE_ROM_END].fill(0xFF);
        self.mbc1_rom_bank_low5 = 1;
        self.mbc1_bank_high2 = 0;
        self.mbc1_mode = 0;
        self.load_fixed_rom_bank(0);
        self.load_rom_bank(1);
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
            self.cells.copy_within(src_base..src_base + 160, 0xFE00);
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
        if self.rom_loaded && addr <= VISIBLE_ROM_END {
            self.handle_write_with_mbc(address, value);
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

    /// Handles writes to cartridge ROM area with MBC support (if applicable)
    fn handle_write_with_mbc(&mut self, address: Addr, value: u8) {
        match self.cartridge_header.as_ref() {
            Some(header) => match header.cartridge_type {
                CartridgeType::Mbc1 | CartridgeType::Mbc1Ram | CartridgeType::Mbc1RamBattery => {
                    let addr = address.0 as usize;

                    match addr {
                        0x0000..=0x1FFF => {
                            // 0x0000-0x1FFF: RAM Enable
                            //self.ram_enabled = value & 0x0F == 0x0A;
                            // Cartridge RAM is not wired yet.
                        }
                        0x2000..=0x3FFF => {
                            // 0x2000-0x3FFF: ROM Bank Number (lower 5 bits)
                            self.mbc1_rom_bank_low5 = value & 0x1F;
                            self.refresh_mbc1_banks();
                        }
                        0x4000..=0x5FFF => {
                            // 0x4000-0x5FFF: RAM Bank Number or upper 2 bits of ROM Bank Number
                            self.mbc1_bank_high2 = value & 0x03;
                            self.refresh_mbc1_banks();
                        }
                        0x6000..=0x7FFF => {
                            // 0x6000-0x7FFF: ROM/RAM mode select
                            self.mbc1_mode = value & 0x01;
                            self.refresh_mbc1_banks();
                        }
                        _ => {}
                    }
                }
                CartridgeType::Rom => {}
                _ => {}
            },
            _ => {}
        }
    }

    fn load_rom_bank(&mut self, bank: usize) {
        if self.cartridge_rom.is_empty() {
            return;
        }

        let bank_count = self.rom_bank_count();
        let effective_bank = self.normalize_switchable_rom_bank(bank, bank_count);

        let src_start = effective_bank * ROM_BANK_SIZE;
        let src_end = (src_start + ROM_BANK_SIZE).min(self.cartridge_rom.len());
        let dst = &mut self.cells[SWITCHABLE_ROM_START..=SWITCHABLE_ROM_END];
        dst.fill(0xFF);

        if src_start < self.cartridge_rom.len() {
            let src = &self.cartridge_rom[src_start..src_end];
            dst[..src.len()].copy_from_slice(src);
        }
    }

    fn load_fixed_rom_bank(&mut self, bank: usize) {
        if self.cartridge_rom.is_empty() {
            return;
        }

        let bank_count = self.rom_bank_count();
        let effective_bank = bank % bank_count;
        let src_start = effective_bank * ROM_BANK_SIZE;
        let src_end = (src_start + ROM_BANK_SIZE).min(self.cartridge_rom.len());
        let dst = &mut self.cells[0x0000..ROM_BANK_SIZE];
        dst.fill(0xFF);

        if src_start < self.cartridge_rom.len() {
            let src = &self.cartridge_rom[src_start..src_end];
            dst[..src.len()].copy_from_slice(src);
        }
    }

    fn normalize_switchable_rom_bank(&self, bank: usize, bank_count: usize) -> usize {
        if bank_count <= 1 {
            return 0;
        }

        let mut selected = bank;
        if matches!(
            self.cartridge_header.as_ref().map(|h| h.cartridge_type),
            Some(CartridgeType::Mbc1 | CartridgeType::Mbc1Ram | CartridgeType::Mbc1RamBattery)
        ) {
            // MBC1 forbids selecting a switchable bank where lower five bits are zero.
            if (selected & 0x1F) == 0 {
                selected |= 0x01;
            }
        } else if selected == 0 {
            selected = 1;
        }

        selected % bank_count
    }

    fn refresh_mbc1_banks(&mut self) {
        let low5 = {
            let mut v = (self.mbc1_rom_bank_low5 & 0x1F) as usize;
            if v == 0 {
                v = 1;
            }
            v
        };
        let high2 = (self.mbc1_bank_high2 & 0x03) as usize;

        let (fixed_bank, switchable_bank) = if self.mbc1_mode == 0 {
            (0usize, (high2 << 5) | low5)
        } else {
            ((high2 << 5), low5)
        };

        self.load_fixed_rom_bank(fixed_bank);
        self.load_rom_bank(switchable_bank);
    }

    fn rom_bank_count(&self) -> usize {
        if let Some(header) = self.cartridge_header.as_ref() {
            return header.rom_bank_count.max(1);
        }
        self.cartridge_rom.len().div_ceil(ROM_BANK_SIZE).max(1)
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

    pub fn read_ie(&self) -> u8 {
        self.cells[0xFFFF]
    }

    pub fn read_if(&self) -> u8 {
        self.cells[0xFF0F]
    }

    pub fn raise_if(&mut self, mask: u8) {
        self.cells[0xFF0F] |= mask;
    }

    pub fn clear_if(&mut self, mask: u8) {
        self.cells[0xFF0F] &= !mask;
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.cells
    }

    #[allow(dead_code)]
    pub fn cartridge_header(&self) -> Option<&CartridgeHeader> {
        self.cartridge_header.as_ref()
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
