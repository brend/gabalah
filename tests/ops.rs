#[cfg(test)]
mod tests {
    use gabalah::cpu::{Cpu, Instruction, Location, Mnemonic};
    use gabalah::cpu::{ZERO_FLAG_BITMASK, SUBTRACTION_FLAG_BITMASK, HALF_CARRY_FLAG_BITMASK, CARRY_FLAG_BITMASK};
    use gabalah::memory::{Registers, Ram, Addr};

    fn setup() -> Cpu {
        Cpu::new()
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
        assert_eq!(cpu.registers.f, SUBTRACTION_FLAG_BITMASK, "unexpected flags");
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
}
