#[cfg(test)]
mod tests {
    use super::*;
    use crate::ram::{Registers, Ram};

    fn setup() -> (Registers, Ram) {
        let registers = Registers::default();
        let memory = Ram::new();
        (registers, memory)
    }

    #[test]
    fn test_ld_immediate() {
        let (mut registers, mut memory) = setup();
        let instruction = Instruction::new(Mnemonic::Ld, 2, 8, vec![Location::A.imm(), Location::Const8.imm()]);
        memory.set(Addr(0x100), 0x42);
        memory.set(Addr(registers.pc + 1), 0x42);
        instruction.execute(&mut memory, &mut registers);
        assert_eq!(registers.a, 0x42);
    }

    #[test]
    fn test_inc() {
        let (mut registers, mut memory) = setup();
        registers.a = 0x10;
        let instruction = Instruction::new(Mnemonic::Inc, 1, 4, vec![Location::A.imm()]);
        instruction.execute(&mut memory, &mut registers);
        assert_eq!(registers.a, 0x11, "unexpected INC result");
        assert_eq!(registers.f, 0, "unexpected flags");
    }

    #[test]
    fn test_inc_wrap() {
        let (mut registers, mut memory) = setup();
        registers.a = 0xFF;
        let instruction = Instruction::new(Mnemonic::Inc, 1, 4, vec![Location::A.imm()]);
        instruction.execute(&mut memory, &mut registers);
        assert_eq!(registers.a, 0x00, "unexpected INC result");
        assert_eq!(registers.f, ZERO_FLAG_BITMASK | HALF_CARRY_FLAG_BITMASK, "unexpected flags");
    }

    #[test]
    fn test_dec() {
        let (mut registers, mut memory) = setup();
        registers.a = 0x10;
        let instruction = Instruction::new(Mnemonic::Dec, 1, 4, vec![Location::A.imm()]);
        instruction.execute(&mut memory, &mut registers);
        assert_eq!(registers.a, 0x0F, "unexpected DEC result");
        assert_eq!(registers.f, SUBTRACTION_FLAG_BITMASK | HALF_CARRY_FLAG_BITMASK, "unexpected flags");
    }

    #[test]
    fn test_dec_zero() {
        let (mut registers, mut memory) = setup();
        registers.a = 0x01;
        let instruction = Instruction::new(Mnemonic::Dec, 1, 4, vec![Location::A.imm()]);
        instruction.execute(&mut memory, &mut registers);
        assert_eq!(registers.a, 0x00, "unexpected DEC result");
        assert_eq!(registers.f, SUBTRACTION_FLAG_BITMASK | ZERO_FLAG_BITMASK, "unexpected flags");
    }

    #[test]
    fn test_add() {
        let (mut registers, mut memory) = setup();
        registers.a = 0x10;
        let instruction = Instruction::new(Mnemonic::Add, 1, 4, vec![Location::A.imm(), Location::Const8.imm()]);
        memory.set(Addr(registers.pc + 1), 0x05);
        instruction.execute(&mut memory, &mut registers);
        assert_eq!(registers.a, 0x15);
    }

    #[test]
    fn test_add_wrap() {
        let (mut registers, mut memory) = setup();
        registers.a = 0xFF;
        let instruction = Instruction::new(Mnemonic::Add, 1, 4, vec![Location::A.imm(), Location::Const8.imm()]);
        memory.set(Addr(registers.pc + 1), 0x01);
        instruction.execute(&mut memory, &mut registers);
        assert_eq!(registers.a, 0x00);
        assert_eq!(registers.f, ZERO_FLAG_BITMASK | CARRY_FLAG_BITMASK | HALF_CARRY_FLAG_BITMASK);
    }

    #[test]
    fn test_sub() {
        let (mut registers, mut memory) = setup();
        registers.a = 0x10;
        let instruction = Instruction::new(Mnemonic::Sub, 1, 4, vec![Location::A.imm(), Location::Const8.imm()]);
        memory.set(Addr(registers.pc + 1), 0x05);
        instruction.execute(&mut memory, &mut registers);
        assert_eq!(registers.a, 0x0B);
        assert_eq!(registers.f, SUBTRACTION_FLAG_BITMASK);
    }

    #[test]
    fn test_sub_zero() {
        let (mut registers, mut memory) = setup();
        registers.a = 0x10;
        let instruction = Instruction::new(Mnemonic::Sub, 1, 4, vec![Location::A.imm(), Location::Const8.imm()]);
        memory.set(Addr(registers.pc + 1), 0x10);
        instruction.execute(&mut memory, &mut registers);
        assert_eq!(registers.a, 0x00);
        assert_eq!(registers.f, SUBTRACTION_FLAG_BITMASK | ZERO_FLAG_BITMASK);
    }
}

use std::{collections::HashMap, vec};

use log::debug;

use crate::alu;
use crate::ram::{Addr, Ram, Registers, Flags, Bytes};

const ZERO_FLAG_BITMASK: u8 = 1 << 7;
const SUBTRACTION_FLAG_BITMASK: u8 = 1 << 6;
const HALF_CARRY_FLAG_BITMASK: u8 = 1 << 5;
const CARRY_FLAG_BITMASK: u8 = 1 << 4;

/// Assembly instruction mnemonics
#[derive(Debug, Clone, Copy)]
enum Mnemonic {
    /// The no-operation
    Nop,
    /// Stop
    Stop,
    /// Load data into a location
    Ld,
    /// Increase the target
    Inc,
    /// Decrease the target
    Dec,
    /// Rotate A left and carry
    Rlca,
    /// Rotate A right; old bit 0 to Carry flag
    Rrca,
    /// Addition
    Add,
    /// Rotate A left through Carry flag
    Rla,
    /// Jump relative
    Jr,
    /// Rotate A right through Carry flag,
    Rra,
    /// Decimally adjust A
    Daa,
    /// Complement A
    Cpl,
    /// Set Carry flag
    Scf,
    /// Complement Carry flag
    Ccf,
    /// Halt
    Halt,
    /// Add with carry
    Adc,
    /// Subtract
    Sub,
    /// Subtract with carry
    Sbc,
    /// And
    And,
    /// Xor
    Xor,
    /// Or
    Or,
    /// Compare
    Cp,
    /// Return
    Ret,
    /// Pop
    Pop,
    /// Jump
    Jp,
    /// Call
    Call,
    /// Push
    Push,
    /// Restart
    Rst,
    /// Return and enable interrupts
    Reti,
    /// Enable interrupts
    Ei, 
    /// Disable interrupts
    Di,
    /// LDHL
    Ldhl,
}

use Mnemonic::*;

/// Represents the location of an instruction's operands
#[derive(Debug, Clone, Copy)]
enum Location {
    /// The accumulator register A
    A,
    /// The general purpose register B
    B,
    /// The general purpose register C
    C,
    /// The general purpose register D
    D,
    /// The general purpose register E
    E,
    /// The general purpose register H
    H,
    /// The general purpose register L
    L,
    /// The 16-bit register pair AF
    AF,
    /// The 16-bit register pair BC
    BC,
    /// The 16-bit register pair HL
    HL,
    /// The 16-bit register pair DE
    DE,
    /// The stack pointer
    SP,
    /// An 8-bit constant value
    Const8,
    /// A 16-bit constant value
    Const16,
    /// Nonzero flag
    FlagNz,
    /// Zero flag
    FlagZ,
    /// No Carry flag (Carry flag is clear)
    FlagNc,
    /// Carry (Carry flag is set)
    FlagC,
}

use Location::*;

impl Location {
    /// Returns the number of bytes the location occupies
    fn bytes(&self) -> usize {
        match self {
            A | B | C | D | E | H | L | FlagNz | FlagZ | FlagNc | FlagC => 1,
            BC | HL | DE | SP | Const8 => 2,
            AF | Const16 => 2,
        }
    }

    /// Creates an immediate value operand from the location
    fn imm(&self) -> Operand {
        Operand::Immediate(*self)
    }

    /// Creates an indirectly referenced (memory) operand from the location
    fn mem(&self) -> Operand {
        Operand::Memory(*self)
    }

    /// Creates an indirectly referenced (memory) operand from the location in high memory
    fn himem(&self) -> Operand {
        Operand::HighMemory(*self)
    }

    /// Writes to the location
    fn write(&self, registers: &mut Registers, memory: &mut Ram, values: Bytes) {
        debug!(
            "writing [{:?}] to {:?}",
            values,
            self
        );
        match self {
            A => registers.a = values.single().expect("expected single byte"),
            BC => registers.set_bc(&values),
            _ => panic!(),
        }
    }

    /// Reads from the location
    fn read(&self, registers: &Registers, memory: &Ram) -> Bytes {
        match self {
            A => registers.a.into(),
            B => registers.b.into(),
            C => registers.c.into(),
            D => registers.d.into(),
            E => registers.e.into(),
            H => registers.h.into(),
            L => registers.l.into(),
            AF => Bytes::from_bytes(registers.f, registers.a), // TODO: is this the correct order?
            BC => Bytes::from_bytes(registers.c, registers.b), // TODO: is this the correct order?
            HL => Bytes::from_bytes(registers.l, registers.h), // TODO: is this the correct order?
            DE => Bytes::from_bytes(registers.e, registers.d), // TODO: is this the correct order?
            SP => registers.sp.into(),
            Const8 => memory.get(Addr(registers.pc).next().unwrap()).into(),
            Const16 => {
                let op_pointer = Addr(registers.pc).next().unwrap();
                Bytes::from_bytes(
                    memory.get(op_pointer),
                    memory.get(op_pointer.next().unwrap()),
                )
            }
            FlagNz => (registers.f & ZERO_FLAG_BITMASK).into(),
            FlagZ => (registers.f & ZERO_FLAG_BITMASK).into(),
            FlagNc => (registers.f & CARRY_FLAG_BITMASK).into(),
            FlagC => (registers.f & CARRY_FLAG_BITMASK).into(),
        }
    }
}

/// An operand of a CPU instruction
#[derive(Debug, Clone, Copy)]
enum Operand {
    /// An immediate value at a given location
    Immediate(Location),
    /// A value indirectly referenced by the address stored at the given location
    Memory(Location),
    /// A value indirectly referenced by the address stored at the given location in high memory
    HighMemory(Location),
}

impl Operand {
    /// Reads the location represented by the operand
    fn read(&self, registers: &Registers, memory: &Ram) -> Bytes {
        match self {
            Operand::Immediate(location) => location.read(registers, memory),
            Operand::Memory(location) => {
                let addr_bytes = location.read(registers, memory);
                let addr = addr_bytes.into();
                memory.get(addr).into()
            },
            Operand::HighMemory(location) => {
                let addr_bytes = location.read(registers, memory);
                let addr_lo_byte = addr_bytes.single().expect("expected single byte");
                let addr_bytes = Bytes::from_bytes(addr_lo_byte, 0xFF); // TODO:Is this the right order?
                let addr = addr_bytes.into();
                memory.get(addr).into()
            },
        }
    }

    /// Writes to the location represented by the operand
    fn write(&self, registers: &mut Registers, memory: &mut Ram, values: Bytes) {
        match self {
            Operand::Immediate(location) => location.write(registers, memory, values),
            Operand::Memory(location) => {
                let addr_bytes = location.read(registers, memory);
                memory.set_word(addr_bytes.into(), &values)
            },
            Operand::HighMemory(location) => {
                let addr_bytes = location.read(registers, memory);
                let addr_lo_byte = addr_bytes.single().expect("expected single byte");
                let addr_bytes = Bytes::from_bytes(addr_lo_byte, 0xFF); // TODO:Is this the right order?
                memory.set_word(addr_bytes.into(), &values)
            },
        }
    }
}

/// An instruction of the Game Boy's CPU
#[derive(Debug, Clone)]
pub struct Instruction {
    /// The instruction's assembly mnemonic, e.g. ld, inc
    mnemonic: Mnemonic,
    /// The length of the instruction in bytes
    pub bytes: usize,
    /// The duration of the instruction in CPU cycles
    _cycles: Vec<usize>,
    /// The operands of the instruction
    operands: Vec<Operand>,
}

impl Instruction {
    /// Creates a new instruction with extended parameters
    fn new_ex(
        mnemonic: Mnemonic,
        bytes: usize,
        cycles: Vec<usize>,
        operands: Vec<Operand>,
    ) -> Instruction {
        Instruction {
            mnemonic,
            bytes,
            _cycles: cycles,
            operands,
        }
    }

    /// Creates a new instruction
    fn new(mnemonic: Mnemonic, bytes: usize, cycles: usize, operands: Vec<Operand>) -> Instruction {
        I::new_ex(mnemonic, bytes, vec![cycles], operands)
    }

    /// Decodes an instruction from its opcode and the provided opcode map
    pub fn decode(opcode: u8, opcode_map: &HashMap<u8, Instruction>) -> Option<Instruction> {
        opcode_map.get(&opcode).cloned()
    }

    /// Executes the instruction, modifying the state of the CPU
    pub fn execute(&self, m: &mut Ram, r: &mut Registers) {
        match self.mnemonic {
            Nop => (),
            Ld => {
                debug_assert!(
                    self.operands.len() == 2,
                    "ld instruction requires 2 operands"
                );
                let dst = self.operands[0];
                let src = self.operands[1];
                dst.write(r, m, src.read(r, m));
            }
            Inc => {
                debug_assert!(
                    self.operands.len() == 1,
                    "inc instruction requires 1 operand"
                );
                let location = self.operands[0];
                let bytes = location.read(r, m);
                let flags = r.flags();
                let (increased, flags) = alu::inc(&bytes, flags);
                location.write(r, m, increased);
                r.set_flags(flags);
            }
            Dec => {
                debug_assert!(
                    self.operands.len() == 1,
                    "dec instruction requires 1 operand"
                );
                let location = self.operands[0];
                let bytes = location.read(r, m);
                let flags = r.flags();
                let (decreased, flags) = alu::dec(&bytes, flags);
                location.write(r, m, decreased);
                r.set_flags(flags);
            }
            Add => {
                debug_assert!(
                    self.operands.len() == 2,
                    "add instruction requires 2 operands"
                );
                let dst = self.operands[0];
                let src = self.operands[1];
                let dst_bytes = dst.read(r, m);
                let src_bytes = src.read(r, m);
                let flags = r.flags();
                let (result, flags) = alu::add(&dst_bytes, &src_bytes, flags);
                dst.write(r, m, result);
                r.set_flags(flags);
            }
            Rlca => {
                let flags = r.flags();
                let (result, flags) = alu::rlc(r.a);
                r.a = result;
                r.set_flags(flags);
            }
            Rrca => {
                let flags = r.flags();
                let (result, flags) = alu::rrc(r.a);
                r.a = result;
                r.set_flags(flags);
            },
            Stop => todo!(),
            Rla => {
                let flags = r.flags();
                let (result, flags) = alu::rl(r.a, flags);
                r.a = result;
                r.set_flags(flags);
            },
            Jr => todo!(),
            Rra => todo!(),
            Daa => todo!(),
            Cpl => todo!(),
            Scf => todo!(),
            Ccf => todo!(),
            Halt => todo!(),
            Adc => todo!(),
            Sub => todo!(),
            Sbc => todo!(),
            And => todo!(),
            Xor => todo!(),
            Or => todo!(),
            Cp => todo!(),
            Ret => todo!(),
            Pop => todo!(),
            Jp => todo!(),
            Call => todo!(),
            Push => todo!(),
            Rst => todo!(),
            Reti => todo!(),
            Ei => todo!(),
            Di => todo!(),
            Ldhl => todo!(),
        }
    }
}

type I = Instruction;

/// Builds and returns a mapping of the 8-bit opcodes to instruction instances
pub fn build_opcode_map() -> HashMap<u8, Instruction> {
    let map: HashMap<u8, Instruction> = HashMap::from_iter([
        // no-op
        (0x00, I::new(Nop, 1, 4, vec![])),
        // load nn into BC
        (0x01, I::new(Ld, 3, 12, vec![BC.imm(), Const16.imm()])),
        // load A into [BC]
        (0x02, I::new(Ld, 1, 8, vec![BC.mem(), A.imm()])),
        // increase BC
        (0x03, I::new(Inc, 1, 8, vec![BC.imm()])),
        // increase B
        (0x04, I::new(Inc, 1, 4, vec![B.imm()])),
        // decrease B
        (0x05, I::new(Dec, 1, 4, vec![B.imm()])),
        // load n into B
        (0x06, I::new(Ld, 2, 8, vec![B.imm(), Const8.imm()])),
        // rotate A left; old bit 7 to Carry flag.
        (0x07, I::new(Rlca, 1, 4, vec![])),
        // load SP into [nn]
        (0x08, I::new(Ld, 3, 20, vec![Const16.mem(), SP.imm()])),
        // add BC to HL
        (0x09, I::new(Add, 1, 8, vec![HL.imm(), BC.imm()])),
        // load BC into A
        (0x0A, I::new(Ld, 1, 8, vec![A.imm(), BC.mem()])),
        // decrease BC
        (0x0B, I::new(Dec, 1, 8, vec![BC.imm()])),
        // increase C
        (0x0C, I::new(Inc, 1, 4, vec![C.imm()])),
        // decrease C
        (0x0D, I::new(Dec, 1, 4, vec![C.imm()])),
        // load n into C
        (0x0E, I::new(Ld, 2, 8, vec![C.imm(), Const8.imm()])),
        // rotate A right; old bit 0 to Carry flag
        (0x0F, I::new(Rrca, 1, 4, vec![])),
        // stop
        (0x10, I::new(Stop, 2, 4, vec![Const8.imm()])),
        // load nn into DE
        (0x11, I::new(Ld, 3, 12, vec![DE.imm(), Const16.imm()])),
        // load A into [DE]
        (0x12, I::new(Ld, 1, 8, vec![DE.mem(), A.imm()])),
        // increase DE
        (0x13, I::new(Inc, 1, 8, vec![DE.imm()])),
        // increase D
        (0x14, I::new(Inc, 1, 4, vec![D.imm()])),
        // decrease D
        (0x15, I::new(Dec, 1, 4, vec![D.imm()])),
        // load n into D
        (0x16, I::new(Ld, 2, 6, vec![D.imm(), Const8.imm()])),
        // rotate A left through Carry flag
        (0x17, I::new(Rla, 1, 4, vec![])),
        // jump relative
        (0x18, I::new(Jr, 2, 12, vec![Const8.imm()])),
        // add DE to HL
        (0x19, I::new(Add, 1, 8, vec![HL.imm(), DE.imm()])),
        // load [DE] into A
        (0x1A, I::new(Ld, 1, 8, vec![A.imm(), DE.mem()])),
        // decrease DE
        (0x1B, I::new(Dec, 1, 8, vec![DE.imm()])),
        // increase E
        (0x1C, I::new(Inc, 1, 4, vec![E.imm()])),
        // decrease E
        (0x1D, I::new(Dec, 1, 4, vec![E.imm()])),
        // load n into E
        (0x1E, I::new(Ld, 2, 8, vec![E.imm(), Const8.imm()])),
        // rotate A right through Carry flag
        (0x1F, I::new(Rra, 1, 4, vec![])),
        // jump relative if nonzero
        (
            0x20,
            I::new_ex(Jr, 2, vec![12, 8], vec![FlagNz.imm(), Const8.imm()]),
        ),
        // load nn into HL
        (0x21, I::new(Ld, 3, 12, vec![HL.imm(), Const16.imm()])),
        // load A into [HL]. Increment HL
        // 0x22
        // TODO: invent a way to implement this -- new type of operand maybe?

        // increase HL
        (0x23, I::new(Inc, 1, 8, vec![HL.imm()])),
        // increase H
        (0x24, I::new(Inc, 1, 4, vec![H.imm()])),
        // decrease H
        (0x25, I::new(Dec, 1, 4, vec![H.imm()])),
        // load n into H
        (0x26, I::new(Ld, 2, 8, vec![H.imm(), Const8.imm()])),
        // decimal adjust A
        (0x27, I::new(Daa, 1, 4, vec![])),
        // jump relative if zero
        (0x28, I::new_ex(Jr, 2, vec![12, 8], vec![FlagZ.imm(), Const8.imm()])),
        // add HL to HL
        (0x29, I::new(Add, 1, 8, vec![HL.imm(), HL.imm()])),
        // load [HL] into A. Increment HL
        // 0x2A
        // TODO: invent a way to implement this
        // decrease HL
        (0x2B, I::new(Dec, 1, 8, vec![HL.imm()])),
        // increase L
        (0x2C, I::new(Inc, 1, 4, vec![L.imm()])),
        // decrease L
        (0x2D, I::new(Dec, 1, 4, vec![L.imm()])),
        // load n into L
        (0x2E, I::new(Ld, 2, 8, vec![L.imm(), Const8.imm()])),
        // complement A
        (0x2F, I::new(Cpl, 1, 4, vec![])),
        // jump relative if C flag is clear
        (0x30, I::new_ex(Jr, 2, vec![12, 8], vec![FlagNc.imm(), Const8.imm()])),
        // load nn into SP
        (0x31, I::new(Ld, 3, 12, vec![SP.imm(), Const16.imm()])),
        // load A into [HL]. Decrement HL
        // 0x32
        // TODO: invent a way to implement this
        // increase SP
        (0x33, I::new(Inc, 1, 8, vec![SP.imm()])),
        // increase (HL)
        (0x34, I::new(Inc, 1, 12, vec![HL.mem()])),
        // decrease (HL)
        (0x35, I::new(Dec, 1, 12, vec![HL.mem()])),
        // load n into (HL)
        (0x36, I::new(Ld, 2, 12, vec![HL.mem(), Const8.imm()])),
        // set C flag
        (0x37, I::new(Scf, 1, 4, vec![FlagC.imm()])),
        // jump relative if C flag is set
        (0x38, I::new_ex(Jr, 2, vec![12, 8], vec![FlagC.imm(), Const8.imm()])),
        // add SP to HL
        (0x39, I::new(Add, 1, 8, vec![HL.imm(), SP.imm()])),
        // load [HL] into A. Decrement HL
        // 0x3A
        // TODO: invent a way to implement this
        // decrease SP
        (0x3B, I::new(Dec, 1, 8, vec![SP.imm()])),
        // increase A
        (0x3C, I::new(Inc, 1, 4, vec![A.imm()])),
        // decrease A
        (0x3D, I::new(Dec, 1, 4, vec![A.imm()])),
        // load n into A
        (0x3E, I::new(Ld, 2, 8, vec![A.imm(), Const8.imm()])),
        // complement carry flag
        (0x3F, I::new(Ccf, 1, 4, vec![])),
        // load B into B
        (0x40, I::new(Ld, 1, 4, vec![B.imm(), B.imm()])),
        // load C into B
        (0x41, I::new(Ld, 1, 4, vec![B.imm(), C.imm()])),
        // load D into B
        (0x42, I::new(Ld, 1, 4, vec![B.imm(), D.imm()])),
        // load E into B
        (0x43, I::new(Ld, 1, 4, vec![B.imm(), E.imm()])),
        // load H into B
        (0x44, I::new(Ld, 1, 4, vec![B.imm(), H.imm()])),
        // load L into B
        (0x45, I::new(Ld, 1, 4, vec![B.imm(), L.imm()])),
        // load [HL] into B
        (0x46, I::new(Ld, 1, 8, vec![B.imm(), HL.mem()])),
        // load A into B
        (0x47, I::new(Ld, 1, 4, vec![B.imm(), A.imm()])),
        // load B into C
        (0x48, I::new(Ld, 1, 4, vec![C.imm(), B.imm()])),
        // load C into C
        (0x49, I::new(Ld, 1, 4, vec![C.imm(), C.imm()])),
        // load D into C
        (0x4A, I::new(Ld, 1, 4, vec![C.imm(), D.imm()])),
        // load E into C
        (0x4B, I::new(Ld, 1, 4, vec![C.imm(), E.imm()])),
        // load H into C
        (0x4C, I::new(Ld, 1, 4, vec![C.imm(), H.imm()])),
        // load L into C
        (0x4D, I::new(Ld, 1, 4, vec![C.imm(), L.imm()])),
        // load [HL] into C
        (0x4E, I::new(Ld, 1, 8, vec![C.imm(), HL.mem()])),
        // load A into C
        (0x4F, I::new(Ld, 1, 4, vec![C.imm(), A.imm()])),
        // load B into D
        (0x50, I::new(Ld, 1, 4, vec![D.imm(), B.imm()])),
        // load C into D
        (0x51, I::new(Ld, 1, 4, vec![D.imm(), C.imm()])),
        // load D into D
        (0x52, I::new(Ld, 1, 4, vec![D.imm(), D.imm()])),
        // load E into D
        (0x53, I::new(Ld, 1, 4, vec![D.imm(), E.imm()])),
        // load H into D
        (0x54, I::new(Ld, 1, 4, vec![D.imm(), H.imm()])),
        // load L into D
        (0x55, I::new(Ld, 1, 4, vec![D.imm(), L.imm()])),
        // load [HL] into D
        (0x56, I::new(Ld, 1, 8, vec![D.imm(), HL.mem()])),
        // load A into D
        (0x57, I::new(Ld, 1, 4, vec![D.imm(), A.imm()])),
        // load B into E
        (0x58, I::new(Ld, 1, 4, vec![E.imm(), B.imm()])),
        // load C into E
        (0x59, I::new(Ld, 1, 4, vec![E.imm(), C.imm()])),
        // load D into E
        (0x5A, I::new(Ld, 1, 4, vec![E.imm(), D.imm()])),
        // load E into E
        (0x5B, I::new(Ld, 1, 4, vec![E.imm(), E.imm()])),
        // load H into E
        (0x5C, I::new(Ld, 1, 4, vec![E.imm(), H.imm()])),
        // load L into E
        (0x5D, I::new(Ld, 1, 4, vec![E.imm(), L.imm()])),
        // load [HL] into E
        (0x5E, I::new(Ld, 1, 8, vec![E.imm(), HL.mem()])),
        // load A into E
        (0x5F, I::new(Ld, 1, 4, vec![E.imm(), A.imm()])),
        // load B into H
        (0x60, I::new(Ld, 1, 4, vec![H.imm(), B.imm()])),
        // load C into H
        (0x61, I::new(Ld, 1, 4, vec![H.imm(), C.imm()])),
        // load D into H
        (0x62, I::new(Ld, 1, 4, vec![H.imm(), D.imm()])),
        // load E into H
        (0x63, I::new(Ld, 1, 4, vec![H.imm(), E.imm()])),
        // load H into H
        (0x64, I::new(Ld, 1, 4, vec![H.imm(), H.imm()])),
        // load L into H
        (0x65, I::new(Ld, 1, 4, vec![H.imm(), L.imm()])),
        // load [HL] into H
        (0x66, I::new(Ld, 1, 8, vec![H.imm(), HL.mem()])),
        // load A into H
        (0x67, I::new(Ld, 1, 4, vec![H.imm(), A.imm()])),
        // load B into L
        (0x68, I::new(Ld, 1, 4, vec![L.imm(), B.imm()])),
        // load C into L
        (0x69, I::new(Ld, 1, 4, vec![L.imm(), C.imm()])),
        // load D into L
        (0x6A, I::new(Ld, 1, 4, vec![L.imm(), D.imm()])),
        // load E into L
        (0x6B, I::new(Ld, 1, 4, vec![L.imm(), E.imm()])),
        // load H into L
        (0x6C, I::new(Ld, 1, 4, vec![L.imm(), H.imm()])),
        // load L into L
        (0x6D, I::new(Ld, 1, 4, vec![L.imm(), L.imm()])),
        // load [HL] into L
        (0x6E, I::new(Ld, 1, 8, vec![L.imm(), HL.mem()])),
        // load A into L
        (0x6F, I::new(Ld, 1, 4, vec![L.imm(), A.imm()])),
        // load B into [HL]
        (0x70, I::new(Ld, 1, 8, vec![HL.mem(), B.imm()])),
        // load C into [HL]
        (0x71, I::new(Ld, 1, 8, vec![HL.mem(), C.imm()])),
        // load D into [HL]
        (0x72, I::new(Ld, 1, 8, vec![HL.mem(), D.imm()])),
        // load E into [HL]
        (0x73, I::new(Ld, 1, 8, vec![HL.mem(), E.imm()])),
        // load H into [HL]
        (0x74, I::new(Ld, 1, 8, vec![HL.mem(), H.imm()])),
        // load L into [HL]
        (0x75, I::new(Ld, 1, 8, vec![HL.mem(), L.imm()])),
        // halt
        (0x76, I::new(Halt, 1, 4, vec![])),
        // load A into [HL]
        (0x77, I::new(Ld, 1, 8, vec![HL.mem(), A.imm()])),
        // load B into A
        (0x78, I::new(Ld, 1, 4, vec![A.imm(), B.imm()])),
        // load C into A
        (0x79, I::new(Ld, 1, 4, vec![A.imm(), C.imm()])),
        // load D into A
        (0x7A, I::new(Ld, 1, 4, vec![A.imm(), D.imm()])),
        // load E into A
        (0x7B, I::new(Ld, 1, 4, vec![A.imm(), E.imm()])),
        // load H into A
        (0x7C, I::new(Ld, 1, 4, vec![A.imm(), H.imm()])),
        // load L into A
        (0x7D, I::new(Ld, 1, 4, vec![A.imm(), L.imm()])),
        // load [HL] into A
        (0x7E, I::new(Ld, 1, 8, vec![A.imm(), HL.mem()])),
        // load A into A
        (0x7F, I::new(Ld, 1, 4, vec![A.imm(), A.imm()])),
        // add B to A
        (0x80, I::new(Add, 1, 4, vec![A.imm(), B.imm()])),
        // add C to A
        (0x81, I::new(Add, 1, 4, vec![A.imm(), C.imm()])),
        // add D to A
        (0x82, I::new(Add, 1, 4, vec![A.imm(), D.imm()])),
        // add E to A
        (0x83, I::new(Add, 1, 4, vec![A.imm(), E.imm()])),
        // add H to A
        (0x84, I::new(Add, 1, 4, vec![A.imm(), H.imm()])),
        // add L to A
        (0x85, I::new(Add, 1, 4, vec![A.imm(), L.imm()])),
        // add [HL] to A
        (0x86, I::new(Add, 1, 8, vec![A.imm(), HL.mem()])),
        // add A to A
        (0x87, I::new(Add, 1, 4, vec![A.imm(), A.imm()])),
        // add B to A with carry
        (0x88, I::new(Adc, 1, 4, vec![A.imm(), B.imm()])),
        // add C to A with carry
        (0x89, I::new(Adc, 1, 4, vec![A.imm(), C.imm()])),
        // add D to A with carry
        (0x8A, I::new(Adc, 1, 4, vec![A.imm(), D.imm()])),
        // add E to A with carry
        (0x8B, I::new(Adc, 1, 4, vec![A.imm(), E.imm()])),
        // add H to A with carry
        (0x8C, I::new(Adc, 1, 4, vec![A.imm(), H.imm()])),
        // add L to A with carry
        (0x8D, I::new(Adc, 1, 4, vec![A.imm(), L.imm()])),
        // add [HL] to A with carry
        (0x8E, I::new(Adc, 1, 8, vec![A.imm(), HL.mem()])),
        // add A to A with carry
        (0x8F, I::new(Adc, 1, 4, vec![A.imm(), A.imm()])),
        // subtract B from A
        (0x90, I::new(Sub, 1, 4, vec![A.imm(), B.imm()])),
        // subtract C from A
        (0x91, I::new(Sub, 1, 4, vec![A.imm(), C.imm()])),
        // subtract D from A
        (0x92, I::new(Sub, 1, 4, vec![A.imm(), D.imm()])),
        // subtract E from A
        (0x93, I::new(Sub, 1, 4, vec![A.imm(), E.imm()])),
        // subtract H from A
        (0x94, I::new(Sub, 1, 4, vec![A.imm(), H.imm()])),
        // subtract L from A
        (0x95, I::new(Sub, 1, 4, vec![A.imm(), L.imm()])),
        // subtract [HL] from A
        (0x96, I::new(Sub, 1, 8, vec![A.imm(), HL.mem()])),
        // subtract A from A
        (0x97, I::new(Sub, 1, 4, vec![A.imm(), A.imm()])),
        // subtract B from A with carry
        (0x98, I::new(Sbc, 1, 4, vec![A.imm(), B.imm()])),
        // subtract C from A with carry
        (0x99, I::new(Sbc, 1, 4, vec![A.imm(), C.imm()])),
        // subtract D from A with carry
        (0x9A, I::new(Sbc, 1, 4, vec![A.imm(), D.imm()])),
        // subtract E from A with carry
        (0x9B, I::new(Sbc, 1, 4, vec![A.imm(), E.imm()])),
        // subtract H from A with carry
        (0x9C, I::new(Sbc, 1, 4, vec![A.imm(), H.imm()])),
        // subtract L from A with carry
        (0x9D, I::new(Sbc, 1, 4, vec![A.imm(), L.imm()])),
        // subtract [HL] from A with carry
        (0x9E, I::new(Sbc, 1, 8, vec![A.imm(), HL.mem()])),
        // subtract A from A with carry
        (0x9F, I::new(Sbc, 1, 4, vec![A.imm(), A.imm()])),
        // and B with A
        (0xA0, I::new(And, 1, 4, vec![A.imm(), B.imm()])),
        // and C with A
        (0xA1, I::new(And, 1, 4, vec![A.imm(), C.imm()])),
        // and D with A
        (0xA2, I::new(And, 1, 4, vec![A.imm(), D.imm()])),
        // and E with A
        (0xA3, I::new(And, 1, 4, vec![A.imm(), E.imm()])),
        // and H with A
        (0xA4, I::new(And, 1, 4, vec![A.imm(), H.imm()])),
        // and L with A
        (0xA5, I::new(And, 1, 4, vec![A.imm(), L.imm()])),
        // and [HL] with A
        (0xA6, I::new(And, 1, 8, vec![A.imm(), HL.mem()])),
        // and A with A
        (0xA7, I::new(And, 1, 4, vec![A.imm(), A.imm()])),
        // xor B with A
        (0xA8, I::new(Xor, 1, 4, vec![A.imm(), B.imm()])),
        // xor C with A
        (0xA9, I::new(Xor, 1, 4, vec![A.imm(), C.imm()])),
        // xor D with A
        (0xAA, I::new(Xor, 1, 4, vec![A.imm(), D.imm()])),
        // xor E with A
        (0xAB, I::new(Xor, 1, 4, vec![A.imm(), E.imm()])),
        // xor H with A
        (0xAC, I::new(Xor, 1, 4, vec![A.imm(), H.imm()])),
        // xor L with A
        (0xAD, I::new(Xor, 1, 4, vec![A.imm(), L.imm()])),
        // xor [HL] with A
        (0xAE, I::new(Xor, 1, 8, vec![A.imm(), HL.mem()])),
        // xor A with A
        (0xAF, I::new(Xor, 1, 4, vec![A.imm(), A.imm()])),
        // or B with A
        (0xB0, I::new(Or, 1, 4, vec![A.imm(), B.imm()])),
        // or C with A
        (0xB1, I::new(Or, 1, 4, vec![A.imm(), C.imm()])),
        // or D with A
        (0xB2, I::new(Or, 1, 4, vec![A.imm(), D.imm()])),
        // or E with A
        (0xB3, I::new(Or, 1, 4, vec![A.imm(), E.imm()])),
        // or H with A
        (0xB4, I::new(Or, 1, 4, vec![A.imm(), H.imm()])),
        // or L with A
        (0xB5, I::new(Or, 1, 4, vec![A.imm(), L.imm()])),
        // or [HL] with A
        (0xB6, I::new(Or, 1, 8, vec![A.imm(), HL.mem()])),
        // or A with A
        (0xB7, I::new(Or, 1, 4, vec![A.imm(), A.imm()])),
        // compare B with A
        (0xB8, I::new(Cp, 1, 4, vec![A.imm(), B.imm()])),
        // compare C with A
        (0xB9, I::new(Cp, 1, 4, vec![A.imm(), C.imm()])),
        // compare D with A
        (0xBA, I::new(Cp, 1, 4, vec![A.imm(), D.imm()])),
        // compare E with A
        (0xBB, I::new(Cp, 1, 4, vec![A.imm(), E.imm()])),
        // compare H with A
        (0xBC, I::new(Cp, 1, 4, vec![A.imm(), H.imm()])),
        // compare L with A
        (0xBD, I::new(Cp, 1, 4, vec![A.imm(), L.imm()])),
        // compare [HL] with A
        (0xBE, I::new(Cp, 1, 8, vec![A.imm(), HL.mem()])),
        // compare A with A
        (0xBF, I::new(Cp, 1, 4, vec![A.imm(), A.imm()])),
        // return if nonzero
        (0xC0, I::new_ex(Ret, 1, vec![20, 8], vec![FlagNz.imm()])),
        // pop BC
        (0xC1, I::new(Pop, 1, 12, vec![BC.imm()])),
        // jump to nn if nonzero
        (0xC2, I::new_ex(Jp, 3, vec![16, 12], vec![FlagNz.imm(), Const16.imm()])),
        // jump to nn
        (0xC3, I::new(Jp, 3, 16, vec![Const16.imm()])),
        // call nn if nonzero
        (0xC4, I::new_ex(Call, 3, vec![24, 12], vec![FlagNz.imm(), Const16.imm()])),
        // push BC
        (0xC5, I::new(Push, 1, 16, vec![BC.imm()])),
        // add n to A
        (0xC6, I::new(Ld, 2, 8, vec![A.imm(), Const8.imm()])),
        // restart from 0x00
        // 0xC7
        // TODO: implement this
        // return if zero
        (0xC8, I::new_ex(Ret, 1, vec![20, 8], vec![FlagZ.imm()])),
        // return
        (0xC9, I::new(Ret, 1, 16, vec![])),
        // jump to nn if zero
        (0xCA, I::new_ex(Jp, 3, vec![16, 12], vec![FlagZ.imm(), Const16.imm()])),
        // extended operations
        // 0xCB
        // ???
        // call nn if zero
        (0xCC, I::new_ex(Call, 3, vec![24, 12], vec![FlagZ.imm(), Const16.imm()])),
        // call nn
        (0xCD, I::new(Call, 3, 24, vec![Const16.imm()])),
        // add n to A with carry
        (0xCE, I::new(Ld, 2, 8, vec![A.imm(), Const8.imm()])),
        // restart from 0x08
        // 0xCF
        // TODO: implement this
        // return if no carry
        (0xD0, I::new_ex(Ret, 1, vec![20, 8], vec![FlagNc.imm()])),
        // pop DE
        (0xD1, I::new(Pop, 1, 12, vec![DE.imm()])),
        // jump to nn if no carry
        (0xD2, I::new_ex(Jp, 3, vec![16, 12], vec![FlagNc.imm(), Const16.imm()])),
        // extended operations
        // 0xD3
        // ???
        // call nn if no carry
        (0xD4, I::new_ex(Call, 3, vec![24, 12], vec![FlagNc.imm(), Const16.imm()])),
        // push DE
        (0xD5, I::new(Push, 1, 16, vec![DE.imm()])),
        // subtract n from A
        (0xD6, I::new(Ld, 2, 8, vec![A.imm(), Const8.imm()])),
        // restart from 0x10
        // 0xD7
        // TODO: implement this
        // return if carry
        (0xD8, I::new_ex(Ret, 1, vec![20, 8], vec![FlagC.imm()])),
        // return and enable interrupts
        (0xD9, I::new(Reti, 1, 16, vec![])),
        // jump to nn if carry
        (0xDA, I::new_ex(Jp, 3, vec![16, 12], vec![FlagC.imm(), Const16.imm()])),
        // extended operations
        // 0xDB
        // ???
        // call nn if carry
        (0xDC, I::new_ex(Call, 3, vec![24, 12], vec![FlagC.imm(), Const16.imm()])),
        // extended operations
        // 0xDD
        // ???
        // subtract n from A with carry
        (0xDE, I::new(Ld, 2, 8, vec![A.imm(), Const8.imm()])),
        // restart from 0x18
        // 0xDF
        // TODO: implement this
        // load A into [0xFF + n]
        (0xE0, I::new(Ld, 2, 12, vec![Const8.himem(), A.imm()])),
        // pop HL
        (0xE1, I::new(Pop, 1, 12, vec![HL.imm()])),
        // load A into [0xFF + C]
        (0xE2, I::new(Ld, 1, 8, vec![C.himem(), A.imm()])),
        // extended operations
        // 0xE3
        // ???
        // extended operations
        // 0xE4
        // ???
        // push HL
        (0xE5, I::new(Push, 1, 16, vec![HL.imm()])),
        // and n with A
        (0xE6, I::new(Ld, 2, 8, vec![A.imm(), Const8.imm()])),
        // restart from 0x20
        // 0xE7
        // TODO: implement this
        // add SP to HL
        (0xE8, I::new(Add, 2, 16, vec![SP.imm(), Const8.imm()])),
        // jump to HL
        (0xE9, I::new(Jp, 1, 4, vec![HL.imm()])),
        // load A into [nn]
        (0xEA, I::new(Ld, 3, 16, vec![Const16.mem(), A.imm()])),
        // extended operations
        // 0xEB
        // ???
        // extended operations
        // 0xEC
        // ???
        // extended operations
        // 0xED
        // ???
        // xor n with A
        (0xEE, I::new(Ld, 2, 8, vec![A.imm(), Const8.imm()])),
        // restart from 0x28
        // 0xEF
        // TODO: implement this
        // load [0xFF + n] into A
        (0xF0, I::new(Ld, 2, 12, vec![A.imm(), Const8.himem()])),
        // pop AF
        (0xF1, I::new(Pop, 1, 12, vec![AF.imm()])),
        // load [0xFF + C] into A
        (0xF2, I::new(Ld, 1, 8, vec![A.imm(), C.himem()])),
        // disable interrupts
        (0xF3, I::new(Di, 1, 4, vec![])),
        // extended operations
        // 0xF4
        // ???
        // push AF
        (0xF5, I::new(Push, 1, 16, vec![AF.imm()])),
        // or n with A
        (0xF6, I::new(Ld, 2, 8, vec![A.imm(), Const8.imm()])),
        // restart from 0x30
        // 0xF7
        // TODO: implement this
        // load SP + n into HL
        (0xF8, I::new(Ldhl, 2, 12, vec![Const8.imm()])),
        // load HL into [SP]
        (0xF9, I::new(Ld, 1, 8, vec![SP.mem(), HL.imm()])),
        // load [nn] into A
        (0xFA, I::new(Ld, 3, 16, vec![A.imm(), Const16.mem()])),
        // enable interrupts
        (0xFB, I::new(Ei, 1, 4, vec![])),
        // extended operations
        // 0xFC
        // ???
        // extended operations
        // 0xFD
        // ???
        // compare n with A
        (0xFE, I::new(Cp, 2, 8, vec![A.imm(), Const8.imm()])),
        // restart from 0x38
        // 0xFF
        // TODO: implement this
    ]);

    map
}
