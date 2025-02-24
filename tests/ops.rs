#[cfg(test)]
mod tests {
    use gabalah::cpu::{Instruction, Mnemonic, Location};
    use gabalah::cpu::{ZERO_FLAG_BITMASK, SUBTRACTION_FLAG_BITMASK, HALF_CARRY_FLAG_BITMASK, CARRY_FLAG_BITMASK};
    use gabalah::memory::{Registers, Ram, Addr};

    fn setup() -> (Registers, Ram) {
        let registers = Registers::default();
        let memory = Ram::new();
        (registers, memory)
    }

    #[test]
    fn test_ld_immediate() {
        let (mut registers, mut memory) = setup();
        let instruction = Instruction::new(Mnemonic::Ld(Location::A.imm(), Location::Const8.imm()), 2, 8);
        memory.write_byte(Addr(0x100), 0x42);
        memory.write_byte(Addr(registers.pc + 1), 0x42);
        instruction.execute(&mut memory, &mut registers);
        assert_eq!(registers.a, 0x42);
    }

    #[test]
    fn test_inc() {
        let (mut registers, mut memory) = setup();
        registers.a = 0x10;
        let instruction = Instruction::new(Mnemonic::Inc(Location::A.imm()), 1, 4);
        instruction.execute(&mut memory, &mut registers);
        assert_eq!(registers.a, 0x11, "unexpected INC result");
        assert_eq!(registers.f, 0, "unexpected flags");
    }

    #[test]
    fn test_inc_wrap() {
        let (mut registers, mut memory) = setup();
        registers.a = 0xFF;
        let instruction = Instruction::new(Mnemonic::Inc(Location::A.imm()), 1, 4);
        instruction.execute(&mut memory, &mut registers);
        assert_eq!(registers.a, 0x00, "unexpected INC result");
        assert_eq!(registers.f, ZERO_FLAG_BITMASK | HALF_CARRY_FLAG_BITMASK, "unexpected flags");
    }

    #[test]
    fn test_dec() {
        let (mut registers, mut memory) = setup();
        registers.a = 0x10;
        let instruction = Instruction::new(Mnemonic::Dec(Location::A.imm()), 1, 4);
        instruction.execute(&mut memory, &mut registers);
        assert_eq!(registers.a, 0x0F, "unexpected DEC result");
        assert_eq!(registers.f, SUBTRACTION_FLAG_BITMASK | HALF_CARRY_FLAG_BITMASK, "unexpected flags");
    }

    #[test]
    fn test_dec_zero() {
        let (mut registers, mut memory) = setup();
        registers.a = 0x01;
        let instruction = Instruction::new(Mnemonic::Dec(Location::A.imm()), 1, 4);
        instruction.execute(&mut memory, &mut registers);
        assert_eq!(registers.a, 0x00, "unexpected DEC result");
        assert_eq!(registers.f, SUBTRACTION_FLAG_BITMASK | ZERO_FLAG_BITMASK, "unexpected flags");
    }

    #[test]
    fn test_add() {
        let (mut registers, mut memory) = setup();
        registers.a = 0x10;
        let instruction = Instruction::new(Mnemonic::Add(Location::A.imm(), Location::Const8.imm()), 1, 4);
        memory.write_byte(Addr(registers.pc + 1), 0x05);
        instruction.execute(&mut memory, &mut registers);
        assert_eq!(registers.a, 0x15);
    }

    #[test]
    fn test_add_wrap() {
        let (mut registers, mut memory) = setup();
        registers.a = 0xFF;
        let instruction = Instruction::new(Mnemonic::Add(Location::A.imm(), Location::Const8.imm()), 1, 4);
        memory.write_byte(Addr(registers.pc + 1), 0x01);
        instruction.execute(&mut memory, &mut registers);
        assert_eq!(registers.a, 0x00);
        assert_eq!(registers.f, ZERO_FLAG_BITMASK | CARRY_FLAG_BITMASK | HALF_CARRY_FLAG_BITMASK);
    }

    #[test]
    fn test_sub() {
        let (mut registers, mut memory) = setup();
        registers.a = 0x10;
        let instruction = Instruction::new(Mnemonic::Sub(Location::A.imm(), Location::Const8.imm()), 1, 4);
        memory.write_byte(Addr(registers.pc + 1), 0x05);
        instruction.execute(&mut memory, &mut registers);
        assert_eq!(registers.a, 0x0B, "unexpected result");
        assert_eq!(registers.f, SUBTRACTION_FLAG_BITMASK, "unexpected flags");
    }

    #[test]
    fn test_sub_zero() {
        let (mut registers, mut memory) = setup();
        registers.a = 0x10;
        let instruction = Instruction::new(Mnemonic::Sub(Location::A.imm(), Location::Const8.imm()), 1, 4);
        memory.write_byte(Addr(registers.pc + 1), 0x10);
        instruction.execute(&mut memory, &mut registers);
        assert_eq!(registers.a, 0x00);
        assert_eq!(registers.f, SUBTRACTION_FLAG_BITMASK | ZERO_FLAG_BITMASK);
    }

    #[test]
    fn test_jr() {
        let (mut registers, mut memory) = setup();
        registers.pc = 0x100;
        let instruction = Instruction::new(Mnemonic::Jr(Location::Const8.imm()), 2, 12);
        memory.write_byte(Addr(0x101), 0x05);
        instruction.execute(&mut memory, &mut registers);
        assert_eq!(registers.pc, 0x100 + 2 + 5);
    }

    #[test]
    fn test_jrc_nz_taken() {
        let (mut registers, mut memory) = setup();
        registers.pc = 0x100;
        registers.f = 0x0;
        let instruction = Instruction::new(Mnemonic::Jrc(Location::FlagNz.imm(), Location::Const8.imm()), 2, 12);
        memory.write_byte(Addr(0x101), 0xFD);
        instruction.execute(&mut memory, &mut registers);
        assert_eq!(registers.pc, 0x100 + 2 - 3);
    }

    #[test]
    fn test_jrc_nz_not_taken() {
        let (mut registers, mut memory) = setup();
        registers.pc = 0x100;
        registers.f = ZERO_FLAG_BITMASK;
        let instruction = Instruction::new(Mnemonic::Jrc(Location::FlagNz.imm(), Location::Const8.imm()), 2, 12);
        memory.write_byte(Addr(0x101), 0xFD);
        instruction.execute(&mut memory, &mut registers);
        assert_eq!(registers.pc, 0x100 + 2);
    }
}
