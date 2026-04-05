#[cfg(test)]
mod tests {
    use gabalah::cpu::{Cpu, Instruction, Location, Mnemonic};
    use gabalah::cpu::{ZERO_FLAG_BITMASK, SUBTRACTION_FLAG_BITMASK, HALF_CARRY_FLAG_BITMASK, CARRY_FLAG_BITMASK};
    use gabalah::memory::Addr;

    fn setup() -> Cpu {
        let mut cpu = Cpu::new();
        cpu.registers.f = 0;
        cpu
    }

    #[test]
    fn test_ld_immediate() {
        let mut cpu = setup();
        let instruction = Instruction::new(Mnemonic::Ld(Location::A.imm(), Location::Const8.imm()), 2, 8);
        cpu.memory.write_byte(Addr(0x100), 0x42);
        cpu.memory.write_byte(Addr(cpu.registers.pc + 1), 0x42);
        cpu.execute(&instruction);
        assert_eq!(cpu.registers.a, 0x42);
    }

    #[test]
    fn test_inc() {
        let mut cpu = setup();
        cpu.registers.a = 0x10;
        let instruction = Instruction::new(Mnemonic::Inc(Location::A.imm()), 1, 4);
        cpu.execute(&instruction);
        assert_eq!(cpu.registers.a, 0x11, "unexpected INC result");
        assert_eq!(cpu.registers.f, 0, "unexpected flags");
    }

    #[test]
    fn test_inc_wrap() {
        let mut cpu = setup();
        cpu.registers.a = 0xFF;
        let instruction = Instruction::new(Mnemonic::Inc(Location::A.imm()), 1, 4);
        cpu.execute(&instruction);
        assert_eq!(cpu.registers.a, 0x00, "unexpected INC result");
        assert_eq!(cpu.registers.f, ZERO_FLAG_BITMASK | HALF_CARRY_FLAG_BITMASK, "unexpected flags");
    }

    #[test]
    fn test_dec() {
        let mut cpu = setup();
        cpu.registers.a = 0x10;
        let instruction = Instruction::new(Mnemonic::Dec(Location::A.imm()), 1, 4);
        cpu.execute(&instruction);
        assert_eq!(cpu.registers.a, 0x0F, "unexpected DEC result");
        assert_eq!(cpu.registers.f, SUBTRACTION_FLAG_BITMASK | HALF_CARRY_FLAG_BITMASK, "unexpected flags");
    }

    #[test]
    fn test_dec_zero() {
        let mut cpu = setup();
        cpu.registers.a = 0x01;
        let instruction = Instruction::new(Mnemonic::Dec(Location::A.imm()), 1, 4);
        cpu.execute(&instruction);
        assert_eq!(cpu.registers.a, 0x00, "unexpected DEC result");
        assert_eq!(cpu.registers.f, SUBTRACTION_FLAG_BITMASK | ZERO_FLAG_BITMASK, "unexpected flags");
    }

    #[test]
    fn test_add() {
        let mut cpu = setup();
        cpu.registers.a = 0x10;
        let instruction = Instruction::new(Mnemonic::Add(Location::A.imm(), Location::Const8.imm()), 1, 4);
        cpu.memory.write_byte(Addr(cpu.registers.pc + 1), 0x05);
        cpu.execute(&instruction);
        assert_eq!(cpu.registers.a, 0x15);
    }

    #[test]
    fn test_add_wrap() {
        let mut cpu = setup();
        cpu.registers.a = 0xFF;
        let instruction = Instruction::new(Mnemonic::Add(Location::A.imm(), Location::Const8.imm()), 1, 4);
        cpu.memory.write_byte(Addr(cpu.registers.pc + 1), 0x01);
        cpu.execute(&instruction);
        assert_eq!(cpu.registers.a, 0x00);
        assert_eq!(cpu.registers.f, ZERO_FLAG_BITMASK | CARRY_FLAG_BITMASK | HALF_CARRY_FLAG_BITMASK);
    }

    #[test]
    fn test_sub() {
        let mut cpu = setup();
        cpu.registers.a = 0x10;
        let instruction = Instruction::new(Mnemonic::Sub(Location::A.imm(), Location::Const8.imm()), 1, 4);
        cpu.memory.write_byte(Addr(cpu.registers.pc + 1), 0x05);
        cpu.execute(&instruction);
        assert_eq!(cpu.registers.a, 0x0B, "unexpected result");
        assert_eq!(cpu.registers.f, SUBTRACTION_FLAG_BITMASK | HALF_CARRY_FLAG_BITMASK, "unexpected flags");
    }

    #[test]
    fn test_sub_zero() {
        let mut cpu = setup();
        cpu.registers.a = 0x10;
        let instruction = Instruction::new(Mnemonic::Sub(Location::A.imm(), Location::Const8.imm()), 1, 4);
        cpu.memory.write_byte(Addr(cpu.registers.pc + 1), 0x10);
        cpu.execute(&instruction);
        assert_eq!(cpu.registers.a, 0x00);
        assert_eq!(cpu.registers.f, SUBTRACTION_FLAG_BITMASK | ZERO_FLAG_BITMASK);
    }

    #[test]
    fn test_jr() {
        let mut cpu = setup();
        cpu.registers.pc = 0x100;
        let instruction = Instruction::new(Mnemonic::Jr(Location::Const8.imm()), 2, 12);
        cpu.memory.write_byte(Addr(0x101), 0x05);
        cpu.execute(&instruction);
        assert_eq!(cpu.registers.pc, 0x100 + 2 + 5);
    }

    #[test]
    fn test_jrc_nz_taken() {
        let mut cpu = setup();
        cpu.registers.pc = 0x100;
        cpu.registers.f = 0x0;
        let instruction = Instruction::new(Mnemonic::Jrc(Location::FlagNz.imm(), Location::Const8.imm()), 2, 12);
        cpu.memory.write_byte(Addr(0x101), 0xFD);
        cpu.execute(&instruction);
        assert_eq!(cpu.registers.pc, 0x100 + 2 - 3);
    }

    #[test]
    fn test_jrc_nz_not_taken() {
        let mut cpu = setup();
        cpu.registers.pc = 0x100;
        cpu.registers.f = ZERO_FLAG_BITMASK;
        let instruction = Instruction::new(Mnemonic::Jrc(Location::FlagNz.imm(), Location::Const8.imm()), 2, 12);
        cpu.memory.write_byte(Addr(0x101), 0xFD);
        cpu.execute(&instruction);
        assert_eq!(cpu.registers.pc, 0x100 + 2);
    }

    #[test]
    fn test_call_pushes_address_of_next_instruction() {
        let mut cpu = setup();
        cpu.registers.pc = 0x100;
        cpu.registers.sp = 0xFFFE;
        cpu.memory.write_byte(Addr(0x100), 0xCD);
        cpu.memory.write_word(Addr(0x101), 0x1234);
        cpu.step();

        assert_eq!(cpu.registers.pc, 0x1234);
        assert_eq!(cpu.registers.sp, 0xFFFC);
        assert_eq!(cpu.memory.read_word(Addr(0xFFFC)), 0x103);
    }

    #[test]
    fn test_rst_pushes_address_of_next_instruction() {
        let mut cpu = setup();
        cpu.registers.pc = 0x200;
        cpu.registers.sp = 0xFFFE;
        cpu.memory.write_byte(Addr(0x200), 0xC7);
        cpu.step();

        assert_eq!(cpu.registers.pc, 0x00);
        assert_eq!(cpu.memory.read_word(Addr(0xFFFC)), 0x201);
    }

    #[test]
    fn test_add_immediate_opcode_c6() {
        let mut cpu = setup();
        cpu.registers.a = 1;
        cpu.memory.write_byte(Addr(0x100), 0xC6);
        cpu.memory.write_byte(Addr(0x101), 2);
        cpu.step();
        assert_eq!(cpu.registers.a, 3);
    }

    #[test]
    fn test_ld_hli_a() {
        let mut cpu = setup();
        cpu.registers.set_hl(0xC000);
        cpu.registers.a = 0x42;
        cpu.memory.write_byte(Addr(0x100), 0x22);
        cpu.step();
        assert_eq!(cpu.memory.read_byte(Addr(0xC000)), 0x42);
        assert_eq!(cpu.registers.hl(), 0xC001);
    }

    #[test]
    fn test_ld_a_hld() {
        let mut cpu = setup();
        cpu.registers.set_hl(0xC100);
        cpu.memory.write_byte(Addr(0xC100), 0x99);
        cpu.memory.write_byte(Addr(0x100), 0x3A);
        cpu.step();
        assert_eq!(cpu.registers.a, 0x99);
        assert_eq!(cpu.registers.hl(), 0xC0FF);
    }

    #[test]
    fn test_ldh_high_memory_roundtrip() {
        let mut cpu = setup();
        cpu.registers.a = 0x77;
        cpu.memory.write_byte(Addr(0x100), 0xE0);
        cpu.memory.write_byte(Addr(0x101), 0x42);
        cpu.step();
        assert_eq!(cpu.memory.read_byte(Addr(0xFF42)), 0x77);

        cpu.memory.write_byte(Addr(0x102), 0xF0);
        cpu.memory.write_byte(Addr(0x103), 0x42);
        cpu.step();
        assert_eq!(cpu.registers.a, 0x77);
    }

    #[test]
    fn test_add_sp_e8_sets_flags() {
        let mut cpu = setup();
        cpu.registers.sp = 0x00FF;
        cpu.memory.write_byte(Addr(0x100), 0xE8);
        cpu.memory.write_byte(Addr(0x101), 0x01);
        cpu.step();

        assert_eq!(cpu.registers.sp, 0x0100);
        assert_eq!(cpu.registers.f, HALF_CARRY_FLAG_BITMASK | CARRY_FLAG_BITMASK);
    }

    #[test]
    fn test_ldhl_sets_flags() {
        let mut cpu = setup();
        cpu.registers.sp = 0x00FF;
        cpu.memory.write_byte(Addr(0x100), 0xF8);
        cpu.memory.write_byte(Addr(0x101), 0x01);
        cpu.step();

        assert_eq!(cpu.registers.hl(), 0x0100);
        assert_eq!(cpu.registers.f, HALF_CARRY_FLAG_BITMASK | CARRY_FLAG_BITMASK);
    }

    #[test]
    fn test_scf_clears_n_and_h() {
        let mut cpu = setup();
        cpu.registers.f = SUBTRACTION_FLAG_BITMASK | HALF_CARRY_FLAG_BITMASK;
        let instruction = Instruction::new(Mnemonic::Scf, 1, 4);
        cpu.execute(&instruction);
        assert_eq!(cpu.registers.f, CARRY_FLAG_BITMASK);
    }

    #[test]
    fn test_cb_rlc_b() {
        let mut cpu = setup();
        cpu.registers.b = 0b1000_0001;
        cpu.memory.write_byte(Addr(0x100), 0xCB);
        cpu.memory.write_byte(Addr(0x101), 0x00);
        cpu.step();

        assert_eq!(cpu.registers.b, 0b0000_0011);
        assert_eq!(cpu.registers.f & CARRY_FLAG_BITMASK, CARRY_FLAG_BITMASK);
        assert_eq!(cpu.registers.pc, 0x102);
    }

    #[test]
    fn test_cb_bit_sets_zero_and_preserves_carry() {
        let mut cpu = setup();
        cpu.registers.h = 0;
        cpu.registers.f = CARRY_FLAG_BITMASK;
        cpu.memory.write_byte(Addr(0x100), 0xCB);
        cpu.memory.write_byte(Addr(0x101), 0x7C); // BIT 7,H
        cpu.step();

        assert_eq!(cpu.registers.f, ZERO_FLAG_BITMASK | HALF_CARRY_FLAG_BITMASK | CARRY_FLAG_BITMASK);
    }

    #[test]
    fn test_cb_res_on_hl_target() {
        let mut cpu = setup();
        cpu.registers.set_hl(0xC000);
        cpu.memory.write_byte(Addr(0xC000), 0xFF);
        cpu.memory.write_byte(Addr(0x100), 0xCB);
        cpu.memory.write_byte(Addr(0x101), 0x86); // RES 0,(HL)
        cpu.step();

        assert_eq!(cpu.memory.read_byte(Addr(0xC000)), 0xFE);
    }

    #[test]
    fn test_cb_set_on_a() {
        let mut cpu = setup();
        cpu.registers.a = 0;
        cpu.memory.write_byte(Addr(0x100), 0xCB);
        cpu.memory.write_byte(Addr(0x101), 0xDF); // SET 3,A
        cpu.step();

        assert_eq!(cpu.registers.a, 0x08);
    }

    #[test]
    fn test_cb_swap_hl() {
        let mut cpu = setup();
        cpu.registers.set_hl(0xC123);
        cpu.memory.write_byte(Addr(0xC123), 0xF0);
        cpu.memory.write_byte(Addr(0x100), 0xCB);
        cpu.memory.write_byte(Addr(0x101), 0x36); // SWAP (HL)
        cpu.step();

        assert_eq!(cpu.memory.read_byte(Addr(0xC123)), 0x0F);
        assert_eq!(cpu.registers.f, 0);
    }

    #[test]
    fn test_cb_cycles_register_vs_hl_target() {
        let mut cpu = setup();
        cpu.memory.write_byte(Addr(0x100), 0xCB);
        cpu.memory.write_byte(Addr(0x101), 0x00); // RLC B
        cpu.step();
        assert_eq!(cpu.total_cycles, 8);

        cpu.registers.set_hl(0xC000);
        cpu.memory.write_byte(Addr(0x102), 0xCB);
        cpu.memory.write_byte(Addr(0x103), 0x06); // RLC (HL)
        cpu.step();
        assert_eq!(cpu.total_cycles, 24);
    }

    #[test]
    fn test_conditional_jr_cycle_selection() {
        let mut cpu = setup();
        cpu.memory.write_byte(Addr(0x100), 0x20); // JR NZ,e8
        cpu.memory.write_byte(Addr(0x101), 0x02);
        cpu.registers.f = ZERO_FLAG_BITMASK; // NZ false
        cpu.step();
        assert_eq!(cpu.total_cycles, 8);

        cpu.memory.write_byte(Addr(0x102), 0x20); // JR NZ,e8
        cpu.memory.write_byte(Addr(0x103), 0x02);
        cpu.registers.f = 0; // NZ true
        cpu.step();
        assert_eq!(cpu.total_cycles, 20);
    }
}
