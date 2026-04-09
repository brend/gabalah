use gabalah::cartridge::{CartridgeHeader, CartridgeType, CgbMode, Destination};
use gabalah::cpu::Cpu;

fn build_rom() -> Vec<u8> {
    let mut rom = vec![0u8; 0x8000];

    let title = b"TEST GAME";
    rom[0x0134..0x0134 + title.len()].copy_from_slice(title);
    rom[0x0143] = 0x80; // CGB compatible
    rom[0x0144] = b'3';
    rom[0x0145] = b'1'; // new licensee: Nintendo
    rom[0x0146] = 0x03; // SGB enabled
    rom[0x0147] = 0x19; // MBC5
    rom[0x0148] = 0x03; // 16 ROM banks
    rom[0x0149] = 0x03; // 4 RAM banks
    rom[0x014A] = 0x01; // overseas only
    rom[0x014B] = 0x33; // use new licensee code
    rom[0x014C] = 0x07; // version
    rom[0x014D] = 0xA5; // header checksum
    rom[0x014E] = 0xBE;
    rom[0x014F] = 0xEF; // global checksum

    rom
}

fn compute_header_checksum(rom: &[u8]) -> u8 {
    let mut checksum = 0u8;
    for &byte in &rom[0x0134..=0x014C] {
        checksum = checksum.wrapping_sub(byte).wrapping_sub(1);
    }
    checksum
}

fn compute_global_checksum(rom: &[u8]) -> u16 {
    rom.iter()
        .enumerate()
        .filter(|(index, _)| *index != 0x014E && *index != 0x014F)
        .fold(0u16, |sum, (_, &byte)| sum.wrapping_add(byte as u16))
}

fn build_rom_with_valid_checksums() -> Vec<u8> {
    let mut rom = build_rom();
    rom[0x014D] = compute_header_checksum(&rom);
    let global = compute_global_checksum(&rom);
    rom[0x014E] = (global >> 8) as u8;
    rom[0x014F] = (global & 0x00FF) as u8;
    rom
}

#[test]
fn parses_valid_header_fields() {
    let rom = build_rom();
    let header = CartridgeHeader::from_bytes(&rom).expect("header should parse");

    assert_eq!(header.title, "TEST GAME");
    assert_eq!(header.cgb_mode, CgbMode::GbCompatible);
    assert_eq!(header.licensee, "Nintendo");
    assert_eq!(header.sgb_flag, 0x03);
    assert_eq!(header.cartridge_type, CartridgeType::Mbc5);
    assert_eq!(header.rom_bank_count, 16);
    assert_eq!(header.ram_bank_count, 4);
    assert_eq!(header.destination, Destination::OverseasOnly);
    assert_eq!(header.version, 0x07);
    assert_eq!(header.checksum, 0xA5);
    assert_eq!(header.global_checksum, 0xBEEF);
}

#[test]
fn returns_error_for_too_short_buffer() {
    let short = vec![0u8; 0x014F];
    let error = CartridgeHeader::from_bytes(&short).expect_err("short buffer should fail");
    assert!(
        error.contains("buffer too short"),
        "unexpected error message: {error}"
    );
}

#[test]
fn returns_error_for_unsupported_cgb_flag() {
    let mut rom = build_rom();
    rom[0x0143] = 0x12;
    let error = CartridgeHeader::from_bytes(&rom).expect_err("unsupported CGB flag should fail");
    assert_eq!(error, "unsupported CGB flag: 0x12");
}

#[test]
fn maps_unknown_old_licensee_to_placeholder() {
    let mut rom = build_rom();
    rom[0x014B] = 0xAB;
    let header = CartridgeHeader::from_bytes(&rom).expect("header should parse");
    assert_eq!(header.licensee, "Unknown old licensee code 0xAB");
}

#[test]
fn cpu_load_rom_exposes_parsed_cartridge_header() {
    let rom = build_rom();
    let mut cpu = Cpu::new();
    cpu.load_rom(rom);

    let header = cpu
        .cartridge_header()
        .expect("header metadata should be present after ROM load");
    assert_eq!(header.title, "TEST GAME");
    assert_eq!(header.cartridge_type, CartridgeType::Mbc5);
}

#[test]
fn computed_header_checksum_matches_stored_value() {
    let rom = build_rom_with_valid_checksums();
    let header = CartridgeHeader::from_bytes(&rom).expect("header should parse");
    assert_eq!(compute_header_checksum(&rom), header.checksum);
}

#[test]
fn header_checksum_detects_tampered_header_bytes() {
    let mut rom = build_rom_with_valid_checksums();
    rom[0x0134] ^= 0x01; // mutate title byte without recomputing checksum
    let header = CartridgeHeader::from_bytes(&rom).expect("header should still parse");
    assert_ne!(compute_header_checksum(&rom), header.checksum);
}

#[test]
fn computed_global_checksum_matches_stored_value() {
    let rom = build_rom_with_valid_checksums();
    let header = CartridgeHeader::from_bytes(&rom).expect("header should parse");
    assert_eq!(compute_global_checksum(&rom), header.global_checksum);
}

#[test]
fn global_checksum_detects_tampered_rom_bytes() {
    let mut rom = build_rom_with_valid_checksums();
    rom[0x0200] ^= 0xFF; // mutate data outside checksum fields
    let header = CartridgeHeader::from_bytes(&rom).expect("header should still parse");
    assert_ne!(compute_global_checksum(&rom), header.global_checksum);
}
