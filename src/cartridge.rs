use std::fmt;
use std::str;

use log::warn;

const ROM_BANK_SIZE: usize = 16 * 1024;
const EXTERNAL_RAM_BANK_SIZE: usize = 8 * 1024;
const FIXED_ROM_END: usize = 0x3FFF;
const SWITCHABLE_ROM_START: usize = 0x4000;
const SWITCHABLE_ROM_END: usize = 0x7FFF;
const EXTERNAL_RAM_START: usize = 0xA000;
const EXTERNAL_RAM_END: usize = 0xBFFF;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CgbMode {
    None = 0x00,
    GbCompatible = 0x80,
    GbcOnly = 0xC0,
}

impl CgbMode {
    fn from_byte(value: u8) -> Option<Self> {
        match value {
            0x00 => Some(Self::None),
            0x80 => Some(Self::GbCompatible),
            0xC0 => Some(Self::GbcOnly),
            _ => None,
        }
    }
}

impl fmt::Display for CgbMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            CgbMode::None => "DMG (Original Game Boy)",
            CgbMode::GbCompatible => "Dual Mode (CGB compatible)",
            CgbMode::GbcOnly => "CGB Only",
        };
        write!(f, "{label}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CartridgeType {
    Rom,
    Mbc1,
    Mbc1Ram,
    Mbc1RamBattery,
    Mbc2,
    Mbc2Battery,
    RomRam11,
    RomRamBattery11,
    Mmm01,
    Mmm01Ram,
    Mmm01RamBattery,
    Mbc3TimerBattery,
    Mbc3TimerRamBattery12,
    Mbc3,
    Mbc3Ram12,
    Mbc3RamBattery12,
    Mbc5,
    Mbc5Ram,
    Mbc5RamBattery,
    Mbc5Rumble,
    Mbc5RumbleRam,
    Mbc5RumbleRamBattery,
    Mbc6,
    Mbc7SensorRumbleRamBattery,
    PocketCamera,
    BandaiTama5,
    HuC3,
    HuC1RamBattery,
}

impl fmt::Display for CartridgeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            CartridgeType::Rom => "ROM ONLY",
            CartridgeType::Mbc1 => "MBC1",
            CartridgeType::Mbc1Ram => "MBC1+RAM",
            CartridgeType::Mbc1RamBattery => "MBC1+RAM+BATTERY",
            CartridgeType::Mbc2 => "MBC2",
            CartridgeType::Mbc2Battery => "MBC2+BATTERY",
            CartridgeType::RomRam11 => "ROM+RAM",
            CartridgeType::RomRamBattery11 => "ROM+RAM+BATTERY",
            CartridgeType::Mmm01 => "MMM01",
            CartridgeType::Mmm01Ram => "MMM01+RAM",
            CartridgeType::Mmm01RamBattery => "MMM01+RAM+BATTERY",
            CartridgeType::Mbc3TimerBattery => "MBC3+TIMER+BATTERY",
            CartridgeType::Mbc3TimerRamBattery12 => "MBC3+TIMER+RAM+BATTERY",
            CartridgeType::Mbc3 => "MBC3",
            CartridgeType::Mbc3Ram12 => "MBC3+RAM",
            CartridgeType::Mbc3RamBattery12 => "MBC3+RAM+BATTERY",
            CartridgeType::Mbc5 => "MBC5",
            CartridgeType::Mbc5Ram => "MBC5+RAM",
            CartridgeType::Mbc5RamBattery => "MBC5+RAM+BATTERY",
            CartridgeType::Mbc5Rumble => "MBC5+RUMBLE",
            CartridgeType::Mbc5RumbleRam => "MBC5+RUMBLE+RAM",
            CartridgeType::Mbc5RumbleRamBattery => "MBC5+RUMBLE+RAM+BATTERY",
            CartridgeType::Mbc6 => "MBC6",
            CartridgeType::Mbc7SensorRumbleRamBattery => "MBC7+SENSOR+RUMBLE+RAM+BATTERY",
            CartridgeType::PocketCamera => "POCKET CAMERA",
            CartridgeType::BandaiTama5 => "BANDAI TAMA5",
            CartridgeType::HuC3 => "HuC3",
            CartridgeType::HuC1RamBattery => "HuC1+RAM+BATTERY",
        };
        write!(f, "{label}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Destination {
    JapanAndOverseas,
    OverseasOnly,
}

impl fmt::Display for Destination {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Destination::JapanAndOverseas => "Japan + Overseas",
            Destination::OverseasOnly => "Overseas Only",
        };
        write!(f, "{label}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CartridgeHeader {
    pub title: String,
    pub cgb_mode: CgbMode,
    pub licensee: String,
    pub sgb_flag: u8,
    pub cartridge_type: CartridgeType,
    pub rom_bank_count: usize,
    pub ram_bank_count: usize,
    pub destination: Destination,
    pub version: u8,
    pub checksum: u8,
    pub global_checksum: u16,
}

impl CartridgeHeader {
    pub fn from_bytes(buffer: &[u8]) -> Result<Self, String> {
        if buffer.len() < 0x0150 {
            return Err(
                "buffer too short: cartridge header is located between 0x0100 and 0x014F"
                    .to_string(),
            );
        }

        let title = String::from_utf8_lossy(&buffer[0x0134..0x0143])
            .trim_end_matches('\0')
            .to_string();
        let cgb_mode = CgbMode::from_byte(buffer[0x0143])
            .ok_or_else(|| format!("unsupported CGB flag: 0x{:02X}", buffer[0x0143]))?;
        let licensee = Self::read_licensee(buffer)?;
        let sgb_flag = buffer[0x0146];
        let cartridge_type = Self::read_cartridge_type(buffer)
            .ok_or_else(|| format!("unsupported cartridge type: 0x{:02X}", buffer[0x0147]))?;
        let rom_bank_count = Self::read_rom_bank_count(buffer)
            .ok_or_else(|| format!("unsupported ROM size code: 0x{:02X}", buffer[0x0148]))?;
        let ram_bank_count = Self::read_ram_bank_count(buffer)
            .ok_or_else(|| format!("unsupported RAM size code: 0x{:02X}", buffer[0x0149]))?;
        let destination = Self::read_destination(buffer)
            .ok_or_else(|| format!("unsupported destination code: 0x{:02X}", buffer[0x014A]))?;
        let version = buffer[0x014C];
        let checksum = buffer[0x014D];
        let global_checksum = u16::from_be_bytes([buffer[0x014E], buffer[0x014F]]);

        Ok(Self {
            title,
            cgb_mode,
            licensee,
            sgb_flag,
            cartridge_type,
            rom_bank_count,
            ram_bank_count,
            destination,
            version,
            checksum,
            global_checksum,
        })
    }

    fn read_destination(buffer: &[u8]) -> Option<Destination> {
        match buffer[0x014A] {
            0x00 => Some(Destination::JapanAndOverseas),
            0x01 => Some(Destination::OverseasOnly),
            _ => None,
        }
    }

    fn read_ram_bank_count(buffer: &[u8]) -> Option<usize> {
        match buffer[0x0149] {
            0x00 => Some(0),
            0x02 => Some(1),
            0x03 => Some(4),
            0x04 => Some(16),
            0x05 => Some(8),
            _ => None,
        }
    }

    fn read_rom_bank_count(buffer: &[u8]) -> Option<usize> {
        match buffer[0x0148] {
            0x00 => Some(2),
            0x01 => Some(4),
            0x02 => Some(8),
            0x03 => Some(16),
            0x04 => Some(32),
            0x05 => Some(64),
            0x06 => Some(128),
            0x07 => Some(256),
            0x08 => Some(512),
            0x52 => Some(72),
            0x53 => Some(80),
            0x54 => Some(96),
            _ => None,
        }
    }

    fn read_cartridge_type(buffer: &[u8]) -> Option<CartridgeType> {
        type CT = CartridgeType;

        Some(match buffer[0x0147] {
            0x00 => CT::Rom,
            0x01 => CT::Mbc1,
            0x02 => CT::Mbc1Ram,
            0x03 => CT::Mbc1RamBattery,
            0x05 => CT::Mbc2,
            0x06 => CT::Mbc2Battery,
            0x08 => CT::RomRam11,
            0x09 => CT::RomRamBattery11,
            0x0B => CT::Mmm01,
            0x0C => CT::Mmm01Ram,
            0x0D => CT::Mmm01RamBattery,
            0x0F => CT::Mbc3TimerBattery,
            0x10 => CT::Mbc3TimerRamBattery12,
            0x11 => CT::Mbc3,
            0x12 => CT::Mbc3Ram12,
            0x13 => CT::Mbc3RamBattery12,
            0x19 => CT::Mbc5,
            0x1A => CT::Mbc5Ram,
            0x1B => CT::Mbc5RamBattery,
            0x1C => CT::Mbc5Rumble,
            0x1D => CT::Mbc5RumbleRam,
            0x1E => CT::Mbc5RumbleRamBattery,
            0x20 => CT::Mbc6,
            0x22 => CT::Mbc7SensorRumbleRamBattery,
            0xFC => CT::PocketCamera,
            0xFD => CT::BandaiTama5,
            0xFE => CT::HuC3,
            0xFF => CT::HuC1RamBattery,
            _ => return None,
        })
    }

    fn read_licensee(buffer: &[u8]) -> Result<String, String> {
        let licensee = match buffer[0x14B] {
            0x00 => "None",
            0x01 => "Nintendo",
            0x08 => "Capcom",
            0x09 => "HOT-B",
            0x0A => "Jaleco",
            0x0B => "Coconuts Japan",
            0x0C => "Elite Systems",
            0x13 => "EA (Electronic Arts)",
            0x18 => "Hudson Soft",
            0x19 => "ITC Entertainment",
            0x1A => "Yanoman",
            0x1D => "Japan Clary",
            0x1F => "Virgin Games Ltd.3",
            0x24 => "PCM Complete",
            0x25 => "San-X",
            0x28 => "Kemco",
            0x29 => "SETA Corporation",
            0x30 => "Infogrames5",
            0x31 => "Nintendo",
            0x32 => "Bandai",
            0x33 => return Self::read_new_licensee(buffer),
            0x34 => "Konami",
            0x35 => "HectorSoft",
            0x38 => "Capcom",
            0x39 => "Banpresto",
            0x3C => "Entertainment Interactive (stub)",
            0x3E => "Gremlin",
            0x41 => "Ubi Soft1",
            0x42 => "Atlus",
            0x44 => "Malibu Interactive",
            0x46 => "Angel",
            0x47 => "Spectrum HoloByte",
            0x49 => "Irem",
            0x4A => "Virgin Games Ltd.3",
            0x4D => "Malibu Interactive",
            0x4F => "U.S. Gold",
            0x50 => "Absolute",
            0x51 => "Acclaim Entertainment",
            0x52 => "Activision",
            0x53 => "Sammy USA Corporation",
            0x54 => "GameTek",
            0x55 => "Park Place15",
            0x56 => "LJN",
            0x57 => "Matchbox",
            0x59 => "Milton Bradley Company",
            0x5A => "Mindscape",
            0x5B => "Romstar",
            0x5C => "Naxat Soft16",
            0x5D => "Tradewest",
            0x60 => "Titus Interactive",
            0x61 => "Virgin Games Ltd.3",
            0x67 => "Ocean Software",
            0x69 => "EA (Electronic Arts)",
            0x6E => "Elite Systems",
            0x6F => "Electro Brain",
            0x70 => "Infogrames5",
            0x71 => "Interplay Entertainment",
            0x72 => "Broderbund",
            0x73 => "Sculptured Software6",
            0x75 => "The Sales Curve Limited7",
            0x78 => "THQ",
            0x79 => "Accolade8",
            0x7A => "Triffix Entertainment",
            0x7C => "MicroProse",
            0x7F => "Kemco",
            0x80 => "Misawa Entertainment",
            0x83 => "LOZC G.",
            0x86 => "Tokuma Shoten",
            0x8B => "Bullet-Proof Software2",
            0x8C => "Vic Tokai Corp.17",
            0x8E => "Ape Inc.18",
            0x8F => "I'Max19",
            0x91 => "Chunsoft Co.9",
            0x92 => "Video System",
            0x93 => "Tsubaraya Productions",
            0x95 => "Varie",
            0x96 => "Yonezawa10/S'Pal",
            0x97 => "Kemco",
            0x99 => "Arc",
            0x9A => "Nihon Bussan",
            0x9B => "Tecmo",
            0x9C => "Imagineer",
            0x9D => "Banpresto",
            0x9F => "Nova",
            0xA1 => "Hori Electric",
            0xA2 => "Bandai",
            0xA4 => "Konami",
            0xA6 => "Kawada",
            0xA7 => "Takara",
            0xA9 => "Technos Japan",
            0xAA => "Broderbund",
            0xAC => "Toei Animation",
            0xAD => "Toho",
            0xAF => "Namco",
            0xB0 => "Acclaim Entertainment",
            0xB1 => "ASCII Corporation or Nexsoft",
            0xB2 => "Bandai",
            0xB4 => "Square Enix",
            0xB6 => "HAL Laboratory",
            0xB7 => "SNK",
            0xB9 => "Pony Canyon",
            0xBA => "Culture Brain",
            0xBB => "Sunsoft",
            0xBD => "Sony Imagesoft",
            0xBF => "Sammy Corporation",
            0xC0 => "Taito",
            0xC2 => "Kemco",
            0xC3 => "Square",
            0xC4 => "Tokuma Shoten",
            0xC5 => "Data East",
            0xC6 => "Tonkin House",
            0xC8 => "Koei",
            0xC9 => "UFL",
            0xCA => "Ultra Games",
            0xCB => "VAP, Inc.",
            0xCC => "Use Corporation",
            0xCD => "Meldac",
            0xCE => "Pony Canyon",
            0xCF => "Angel",
            0xD0 => "Taito",
            0xD1 => "SOFEL (Software Engineering Lab)",
            0xD2 => "Quest",
            0xD3 => "Sigma Enterprises",
            0xD4 => "ASK Kodansha Co.",
            0xD6 => "Naxat Soft16",
            0xD7 => "Copya System",
            0xD9 => "Banpresto",
            0xDA => "Tomy",
            0xDB => "LJN",
            0xDD => "Nippon Computer Systems",
            0xDE => "Human Ent.",
            0xDF => "Altron",
            0xE0 => "Jaleco",
            0xE1 => "Towa Chiki",
            0xE2 => "Yutaka # Needs more info",
            0xE3 => "Varie",
            0xE5 => "Epoch",
            0xE7 => "Athena",
            0xE8 => "Asmik Ace Entertainment",
            0xE9 => "Natsume",
            0xEA => "King Records",
            0xEB => "Atlus",
            0xEC => "Epic/Sony Records",
            0xEE => "IGS",
            0xF0 => "A Wave",
            0xF3 => "Extreme Entertainment",
            0xFF => "LJN",
            _ => return Ok(format!("Unknown old licensee code 0x{:02X}", buffer[0x14B])),
        };

        Ok(licensee.to_string())
    }

    fn read_new_licensee(buffer: &[u8]) -> Result<String, String> {
        let licensee_code = str::from_utf8(&buffer[0x0144..=0x0145])
            .map_err(|_| "invalid new licensee code encoding".to_string())?;

        let licensee = match licensee_code {
            "00" => "None",
            "01" => "Nintendo Research & Development 1",
            "08" => "Capcom",
            "13" => "EA (Electronic Arts)",
            "18" => "Hudson Soft",
            "19" => "B-AI",
            "20" => "KSS",
            "22" => "Planning Office WADA",
            "24" => "PCM Complete",
            "25" => "San-X",
            "28" => "Kemco",
            "29" => "SETA Corporation",
            "30" => "Viacom",
            "31" => "Nintendo",
            "32" => "Bandai",
            "33" => "Ocean Software/Acclaim Entertainment",
            "34" => "Konami",
            "35" => "HectorSoft",
            "37" => "Taito",
            "38" => "Hudson Soft",
            "39" => "Banpresto",
            "41" => "Ubi Soft1",
            "42" => "Atlus",
            "44" => "Malibu Interactive",
            "46" => "Angel",
            "47" => "Bullet-Proof Software2",
            "49" => "Irem",
            "50" => "Absolute",
            "51" => "Acclaim Entertainment",
            "52" => "Activision",
            "53" => "Sammy USA Corporation",
            "54" => "Konami",
            "55" => "Hi Tech Expressions",
            "56" => "LJN",
            "57" => "Matchbox",
            "58" => "Mattel",
            "59" => "Milton Bradley Company",
            "60" => "Titus Interactive",
            "61" => "Virgin Games Ltd.3",
            "64" => "Lucasfilm Games4",
            "67" => "Ocean Software",
            "69" => "EA (Electronic Arts)",
            "70" => "Infogrames5",
            "71" => "Interplay Entertainment",
            "72" => "Broderbund",
            "73" => "Sculptured Software6",
            "75" => "The Sales Curve Limited7",
            "78" => "THQ",
            "79" => "Accolade8",
            "80" => "Misawa Entertainment",
            "83" => "LOZC G.",
            "86" => "Tokuma Shoten",
            "87" => "Tsukuda Original",
            "91" => "Chunsoft Co.9",
            "92" => "Video System",
            "93" => "Ocean Software/Acclaim Entertainment",
            "95" => "Varie",
            "96" => "Yonezawa10/S'Pal",
            "97" => "Kaneko",
            "99" => "Pack-In-Video",
            "9H" => "Bottom Up",
            "A4" => "Konami (Yu-Gi-Oh!)",
            "BL" => "MTO",
            "DK" => "Kodansha",
            _ => return Ok(format!("Unknown new licensee code {licensee_code}")),
        };

        Ok(licensee.to_string())
    }
}

#[derive(Debug, Clone)]
pub struct Cartridge {
    rom: Vec<u8>,
    external_ram: Vec<u8>,
    header: Option<CartridgeHeader>,
    mapper: MapperState,
}

#[derive(Debug, Clone)]
enum MapperState {
    RomOnly,
    Mbc1(Mbc1State),
}

#[derive(Debug, Clone, Copy)]
struct Mbc1State {
    rom_bank_low5: u8,
    bank_high2: u8,
    mode: u8,
    ram_enabled: bool,
}

impl Default for Mbc1State {
    fn default() -> Self {
        Self {
            rom_bank_low5: 1,
            bank_high2: 0,
            mode: 0,
            ram_enabled: false,
        }
    }
}

impl Cartridge {
    pub fn new(rom: Vec<u8>) -> Self {
        let header = match CartridgeHeader::from_bytes(&rom) {
            Ok(header) => Some(header),
            Err(err) => {
                warn!("Failed to parse cartridge header metadata: {err}");
                None
            }
        };

        let mapper = match header.as_ref().map(|h| h.cartridge_type) {
            Some(CartridgeType::Mbc1 | CartridgeType::Mbc1Ram | CartridgeType::Mbc1RamBattery) => {
                MapperState::Mbc1(Mbc1State::default())
            }
            _ => MapperState::RomOnly,
        };

        Self {
            rom,
            external_ram: vec![0xFF; Self::external_ram_len(header.as_ref())],
            header,
            mapper,
        }
    }

    pub fn header(&self) -> Option<&CartridgeHeader> {
        self.header.as_ref()
    }

    pub fn copy_visible_windows_into(&self, fixed: &mut [u8], switchable: &mut [u8]) {
        self.copy_bank_into(self.fixed_bank(), fixed);
        self.copy_bank_into(self.switchable_bank(), switchable);
    }

    pub fn read_byte(&self, address: u16) -> u8 {
        let addr = address as usize;
        if addr > SWITCHABLE_ROM_END {
            return 0xFF;
        }

        let (bank, offset) = if addr <= FIXED_ROM_END {
            (self.fixed_bank(), addr)
        } else {
            (self.switchable_bank(), addr - SWITCHABLE_ROM_START)
        };
        self.read_bank_byte(bank, offset)
    }

    pub fn write_rom_control(&mut self, address: u16, value: u8) {
        let MapperState::Mbc1(state) = &mut self.mapper else {
            return;
        };

        match address {
            0x0000..=0x1FFF => state.ram_enabled = value & 0x0F == 0x0A,
            0x2000..=0x3FFF => state.rom_bank_low5 = value & 0x1F,
            0x4000..=0x5FFF => state.bank_high2 = value & 0x03,
            0x6000..=0x7FFF => state.mode = value & 0x01,
            _ => {}
        }
    }

    pub fn has_battery_backed_ram(&self) -> bool {
        self.has_battery() && !self.external_ram.is_empty()
    }

    pub fn battery_backed_ram(&self) -> Option<&[u8]> {
        if !self.has_battery_backed_ram() {
            return None;
        }
        Some(&self.external_ram)
    }

    pub fn load_battery_backed_ram(&mut self, data: &[u8]) -> bool {
        if !self.has_battery_backed_ram() {
            return false;
        }
        self.external_ram.fill(0xFF);
        let copy_len = self.external_ram.len().min(data.len());
        self.external_ram[..copy_len].copy_from_slice(&data[..copy_len]);
        true
    }

    pub(crate) fn read_external_ram(&self, address: u16) -> u8 {
        if !self.external_ram_accessible() {
            return 0xFF;
        }
        let Some(index) = self.external_ram_index(address) else {
            return 0xFF;
        };
        self.external_ram.get(index).copied().unwrap_or(0xFF)
    }

    pub(crate) fn write_external_ram(&mut self, address: u16, value: u8) {
        if !self.external_ram_accessible() {
            return;
        }
        let Some(index) = self.external_ram_index(address) else {
            return;
        };
        if let Some(byte) = self.external_ram.get_mut(index) {
            *byte = value;
        }
    }

    fn read_bank_byte(&self, bank: usize, offset: usize) -> u8 {
        let index = bank.saturating_mul(ROM_BANK_SIZE).saturating_add(offset);
        self.rom.get(index).copied().unwrap_or(0xFF)
    }

    fn copy_bank_into(&self, bank: usize, dst: &mut [u8]) {
        dst.fill(0xFF);
        let src_start = bank.saturating_mul(ROM_BANK_SIZE);
        if src_start >= self.rom.len() {
            return;
        }
        let src_end = (src_start + dst.len()).min(self.rom.len());
        let src = &self.rom[src_start..src_end];
        dst[..src.len()].copy_from_slice(src);
    }

    fn fixed_bank(&self) -> usize {
        let bank_count = self.rom_bank_count();
        match self.mapper {
            MapperState::RomOnly => 0,
            MapperState::Mbc1(state) => {
                if state.mode == 0 {
                    0
                } else {
                    (((state.bank_high2 & 0x03) as usize) << 5) % bank_count
                }
            }
        }
    }

    fn switchable_bank(&self) -> usize {
        let bank_count = self.rom_bank_count();
        if bank_count <= 1 {
            return 0;
        }

        match self.mapper {
            MapperState::RomOnly => 1 % bank_count,
            MapperState::Mbc1(state) => {
                let mut low5 = (state.rom_bank_low5 & 0x1F) as usize;
                if low5 == 0 {
                    low5 = 1;
                }
                let high2 = (state.bank_high2 & 0x03) as usize;
                let mut selected = if state.mode == 0 {
                    (high2 << 5) | low5
                } else {
                    low5
                };

                if (selected & 0x1F) == 0 {
                    selected |= 0x01;
                }
                selected % bank_count
            }
        }
    }

    fn rom_bank_count(&self) -> usize {
        if let Some(header) = self.header.as_ref() {
            return header.rom_bank_count.max(1);
        }
        self.rom.len().div_ceil(ROM_BANK_SIZE).max(1)
    }

    fn external_ram_accessible(&self) -> bool {
        if self.external_ram.is_empty() {
            return false;
        }
        match self.mapper {
            MapperState::RomOnly => true,
            MapperState::Mbc1(state) => state.ram_enabled,
        }
    }

    fn external_ram_bank(&self) -> usize {
        let bank_count = self.external_ram_bank_count();
        if bank_count <= 1 {
            return 0;
        }

        match self.mapper {
            MapperState::RomOnly => 0,
            MapperState::Mbc1(state) => {
                let bank = if state.mode == 0 {
                    0
                } else {
                    (state.bank_high2 & 0x03) as usize
                };
                bank % bank_count
            }
        }
    }

    fn external_ram_bank_count(&self) -> usize {
        self.external_ram.len() / EXTERNAL_RAM_BANK_SIZE
    }

    fn external_ram_index(&self, address: u16) -> Option<usize> {
        let addr = address as usize;
        if !(EXTERNAL_RAM_START..=EXTERNAL_RAM_END).contains(&addr) {
            return None;
        }

        let bank = self.external_ram_bank();
        let offset = addr - EXTERNAL_RAM_START;
        Some(
            bank.saturating_mul(EXTERNAL_RAM_BANK_SIZE)
                .saturating_add(offset),
        )
    }

    fn external_ram_len(header: Option<&CartridgeHeader>) -> usize {
        let banks = header.map(|h| h.ram_bank_count).unwrap_or(0);
        banks.saturating_mul(EXTERNAL_RAM_BANK_SIZE)
    }

    fn has_battery(&self) -> bool {
        matches!(
            self.header.as_ref().map(|h| h.cartridge_type),
            Some(
                CartridgeType::Mbc1RamBattery
                    | CartridgeType::Mbc2Battery
                    | CartridgeType::RomRamBattery11
                    | CartridgeType::Mmm01RamBattery
                    | CartridgeType::Mbc3TimerBattery
                    | CartridgeType::Mbc3TimerRamBattery12
                    | CartridgeType::Mbc3RamBattery12
                    | CartridgeType::Mbc5RamBattery
                    | CartridgeType::Mbc5RumbleRamBattery
                    | CartridgeType::Mbc7SensorRumbleRamBattery
                    | CartridgeType::HuC1RamBattery
            )
        )
    }
}
