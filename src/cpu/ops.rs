use std::{collections::HashMap, vec};

use super::alu::{self, Flags};
use crate::memory::{Addr, Ram, Registers};

pub const ZERO_FLAG_BITMASK: u8 = 1 << 7;
pub const SUBTRACTION_FLAG_BITMASK: u8 = 1 << 6;
pub const HALF_CARRY_FLAG_BITMASK: u8 = 1 << 5;
pub const CARRY_FLAG_BITMASK: u8 = 1 << 4;

/// Assembly instruction mnemonics
#[derive(Debug, Clone, Copy)]
pub enum Mnemonic {
    /// The no-operation
    Nop,
    /// Stop
    Stop(Operand),
    /// Load data from a source to a destination
    Ld(Operand, Operand),
    /// Increase the target
    Inc(Operand),
    /// Decrease the target
    Dec(Operand),
    /// Rotate A left and carry
    Rlca,
    /// Rotate A right; old bit 0 to Carry flag
    Rrca,
    /// Addition
    Add(Operand, Operand),
    /// Add with carry
    Adc(Operand, Operand),
    /// Subtract
    Sub(Operand, Operand),
    /// Subtract with carry
    Sbc(Operand, Operand),
    /// Rotate A left through Carry flag
    Rla,
    /// Jump relative
    Jr(Operand),
    /// Jum relative if condition
    Jrc(Operand, Operand),
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
    /// And
    And(Operand, Operand),
    /// Xor
    Xor(Operand, Operand),
    /// Or
    Or(Operand, Operand),
    /// Compare
    Cp(Operand, Operand),
    /// Return
    Ret,
    /// Conditional return
    Retc(Operand),
    /// Pop
    Pop(Operand),
    /// Jump
    Jp(Operand),
    /// Conditional jump
    Jpc(Operand, Operand),
    /// Call
    Call(Operand),
    /// Conditional Call
    Callc(Operand, Operand),
    /// Push
    Push(Operand),
    /// Restart
    Rst(u8),
    /// Return and enable interrupts
    Reti,
    /// Enable interrupts
    Ei,
    /// Disable interrupts
    Di,
    /// LDHL
    Ldhl(Operand),
    /// Invalid instruction
    Invalid(&'static str),
}

use Mnemonic::*;

/// Represents the location of an instruction's operands
#[derive(Debug, Clone, Copy)]
pub enum Location {
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
    /// Writes to the location
    fn write_byte(&self, registers: &mut Registers, value: u8) {
        match self {
            A => registers.a = value,
            B => registers.b = value,
            C => registers.c = value,
            D => registers.d = value,
            E => registers.e = value,
            H => registers.h = value,
            L => registers.l = value,
            _ => panic!("Invalid location for write_byte"),
        }
    }

    fn write_word(&self, registers: &mut Registers, value: u16) {
        match self {
            AF => registers.set_af(value),
            BC => registers.set_bc(value),
            DE => registers.set_de(value),
            HL => registers.set_hl(value),
            _ => panic!("Invalid location for write_word"),
        }
    }

    /// Reads from the location
    fn read_byte(&self, r: &Registers, memory: &Ram) -> u8 {
        match self {
            A => r.a,
            B => r.b,
            C => r.c,
            D => r.d,
            E => r.e,
            H => r.h,
            L => r.l,
            FlagNz => !r.f.zero() as u8,
            FlagZ => r.f.zero() as u8,
            FlagNc => !r.f.carry() as u8,
            FlagC => r.f.carry() as u8,
            Const8 => memory.read_byte(Addr(r.pc + 1)),
            _ => panic!("Invalid location for read_byte"),
        }
    }

    fn read_word(&self, r: &Registers, memory: &Ram) -> u16 {
        match self {
            AF => r.af(),
            BC => r.bc(),
            DE => r.de(),
            HL => r.hl(),
            Const16 => memory.read_word(Addr(r.pc + 1)),
            _ => panic!("Invalid location for read_word"),
        }
    }

    fn target_size(&self) -> usize {
        match self {
            A | B | C | D | E | H | L | FlagNz | FlagZ | FlagNc | FlagC => 1,
            SP | AF | BC | DE | HL | Const16 => 2,
            Const8 => 1,
        }
    }

    pub fn imm(&self) -> Operand {
        Operand::Immediate(*self)
    }

    pub fn ind(&self) -> Operand {
        Operand::Indirect(*self)
    }

    pub fn high(&self) -> Operand {
        Operand::HighMemory(*self)
    }
}

/// An operand of a CPU instruction
#[derive(Debug, Clone, Copy)]
pub enum Operand {
    /// An immediate value at a given location
    Immediate(Location),
    /// A value indirectly referenced by the address stored at the given location
    Indirect(Location),
    /// A value indirectly referenced by the address stored at the given location in high memory
    HighMemory(Location),
}

impl Operand {
    fn target_size(&self) -> usize {
        match self {
            Operand::Immediate(loc) => loc.target_size(),
            Operand::Indirect(_) => 1,
            Operand::HighMemory(_) => 1,
        }
    }

    /// Reads the location represented by the operand and returns a byte
    fn read_byte(&self, registers: &Registers, memory: &Ram) -> u8 {
        match self {
            Operand::Immediate(loc) => loc.read_byte(registers, memory),
            Operand::Indirect(loc) => {
                let addr = loc.read_word(registers, memory);
                memory.read_byte(Addr(addr))
            }
            Operand::HighMemory(loc) => {
                let addr = loc.read_word(registers, memory);
                memory.read_byte(Addr(0xFF00 + addr))
            }
        }
    }

    fn read_word(&self, registers: &Registers, memory: &Ram) -> u16 {
        match self {
            Operand::Immediate(loc) => loc.read_word(registers, memory),
            _ => panic!("Invalid operand size"),
        }
    }

    fn write_byte(&self, registers: &mut Registers, memory: &mut Ram, value: u8) {
        match self {
            Operand::Immediate(loc) => loc.write_byte(registers, value),
            Operand::Indirect(loc) => {
                let addr = loc.read_word(registers, memory);
                memory.write_byte(Addr(addr), value);
            }
            Operand::HighMemory(loc) => {
                let addr = loc.read_word(registers, memory);
                memory.write_byte(Addr(0xFF00 + addr), value);
            }
        }
    }

    fn write_word(&self, registers: &mut Registers, _memory: &mut Ram, value: u16) {
        match self {
            Operand::Immediate(loc) => loc.write_word(registers, value),
            _ => panic!("Invalid operand size"),
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
}

impl Instruction {
    /// Creates a new instruction with extended parameters
    pub fn new_ex(mnemonic: Mnemonic, bytes: usize, cycles: Vec<usize>) -> Instruction {
        Instruction {
            mnemonic,
            bytes,
            _cycles: cycles,
        }
    }

    /// Creates a new instruction
    pub fn new(mnemonic: Mnemonic, bytes: usize, cycles: usize) -> Instruction {
        Instruction::new_ex(mnemonic, bytes, vec![cycles])
    }

    /// Decodes an instruction from its opcode and the provided opcode map
    pub fn decode(opcode: u8, opcode_map: &HashMap<u8, Instruction>) -> Option<Instruction> {
        opcode_map.get(&opcode).cloned()
    }

    /// Executes the instruction, modifying the state of the CPU
    pub fn execute(&self, m: &mut Ram, r: &mut Registers) {
        let mut new_pc = None;

        match self.mnemonic {
            Nop => (),
            Ld(dst, src) => {
                if dst.target_size() == 1 {
                    let byte = src.read_byte(r, m);
                    dst.write_byte(r, m, byte);
                } else {
                    // TODO: Handle Stack Pointer shenanigans
                    let word = src.read_word(r, m);
                    dst.write_word(r, m, word);
                }
            }
            Inc(dst) => {
                if dst.target_size() == 1 {
                    let byte = dst.read_byte(r, m);
                    let increased = alu::inc8(byte, &mut r.f);
                    dst.write_byte(r, m, increased);
                } else {
                    let word = dst.read_word(r, m);
                    let increased = alu::inc16(word);
                    dst.write_word(r, m, increased);
                }
            }
            Dec(dst) => {
                if dst.target_size() == 1 {
                    let byte = dst.read_byte(r, m);
                    let decreased = alu::dec8(byte, &mut r.f);
                    dst.write_byte(r, m, decreased);
                } else {
                    let word = dst.read_word(r, m);
                    let decreased = alu::dec16(word);
                    dst.write_word(r, m, decreased);
                }
            }
            Add(dst, src) => {
                if dst.target_size() == 1 {
                    let dst_byte = dst.read_byte(r, m);
                    let src_byte = src.read_byte(r, m);
                    let sum = alu::add8(dst_byte, src_byte, &mut r.f);
                    dst.write_byte(r, m, sum);
                } else {
                    let dst_word = dst.read_word(r, m);
                    let src_word = src.read_word(r, m);
                    let sum = alu::add16(dst_word, src_word, &mut r.f);
                    dst.write_word(r, m, sum);
                }
            }
            Adc(dst, src) => {
                if dst.target_size() == 1 {
                    let dst_byte = dst.read_byte(r, m);
                    let src_byte = src.read_byte(r, m);
                    let sum = alu::adc8(dst_byte, src_byte, &mut r.f);
                    dst.write_byte(r, m, sum);
                } else {
                    let dst_word = dst.read_word(r, m);
                    let src_word = src.read_word(r, m);
                    let sum = alu::adc16(dst_word, src_word, &mut r.f);
                    dst.write_word(r, m, sum);
                }
            }
            Sub(dst, src) => {
                if dst.target_size() == 1 {
                    let dst_byte = dst.read_byte(r, m);
                    let src_byte = src.read_byte(r, m);
                    let difference = alu::sub8(dst_byte, src_byte, &mut r.f);
                    dst.write_byte(r, m, difference);
                } else {
                    let dst_word = dst.read_word(r, m);
                    let src_word = src.read_word(r, m);
                    let difference = alu::sub16(dst_word, src_word, &mut r.f);
                    dst.write_word(r, m, difference);
                }
            }
            Sbc(dst, src) => {
                if dst.target_size() == 1 {
                    let dst_byte = dst.read_byte(r, m);
                    let src_byte = src.read_byte(r, m);
                    let difference = alu::sbc8(dst_byte, src_byte, &mut r.f);
                    dst.write_byte(r, m, difference);
                } else {
                    let dst_word = dst.read_word(r, m);
                    let src_word = src.read_word(r, m);
                    let difference = alu::sbc16(dst_word, src_word, &mut r.f);
                    dst.write_word(r, m, difference);
                }
            }
            Rlca => r.a = alu::rlc(r.a, &mut r.f),
            Rrca => r.a = alu::rrc(r.a, &mut r.f),
            Rla => r.a = alu::rl(r.a, &mut r.f),
            Rra => r.a = alu::rr(r.a, &mut r.f),
            Jr(offset) => {
                let offset = offset.read_byte(r, m) as i8;
                new_pc = Some((r.pc as i32 + 2 + offset as i32) as u16);
            }
            Jrc(cc, offset) => {
                let flag = cc.read_byte(r, m);
                if flag == 1 {
                    let offset = offset.read_byte(r, m) as i8;
                    new_pc = Some((r.pc as i32 + 2 + offset as i32) as u16);
                }
            }
            Daa => alu::daa(&mut r.a, &mut r.f),
            Cpl => {
                r.a = !r.a;
                r.f |= SUBTRACTION_FLAG_BITMASK | HALF_CARRY_FLAG_BITMASK;
            }
            Scf => r.f |= CARRY_FLAG_BITMASK,
            Ccf => {
                r.f ^= CARRY_FLAG_BITMASK;
                r.f &= !SUBTRACTION_FLAG_BITMASK;
                r.f &= !HALF_CARRY_FLAG_BITMASK;
            }
            And(dst, src) => {
                let dst_byte = dst.read_byte(r, m);
                let src_byte = src.read_byte(r, m);
                let result = alu::and(dst_byte, src_byte, &mut r.f);
                dst.write_byte(r, m, result.into());
            }
            Xor(dst, src) => {
                let dst_byte = dst.read_byte(r, m);
                let src_byte = src.read_byte(r, m);
                let result = alu::xor(dst_byte, src_byte, &mut r.f);
                dst.write_byte(r, m, result.into());
            }
            Or(dst, src) => {
                let dst_byte = dst.read_byte(r, m);
                let src_byte = src.read_byte(r, m);
                let result = alu::or(dst_byte, src_byte, &mut r.f);
                dst.write_byte(r, m, result.into());
            }
            Cp(dst, src) => {
                let dst_byte = dst.read_byte(r, m);
                let src_byte = src.read_byte(r, m);
                alu::cp(dst_byte, src_byte, &mut r.f);
            }
            Ret => {
                r.pc = m.read_word(Addr(r.sp));
                r.sp += 2;
            }
            Retc(cc) => {
                let flag = cc.read_byte(r, m);
                if flag == 1 {
                    r.pc = m.read_word(Addr(r.sp));
                    r.sp += 2;
                }
            }
            Stop(_op) => todo!(),
            Halt => todo!(),          
            Reti => todo!(),
            Ei => todo!(),
            Di => todo!(),
            Jp(dst) => {
                debug_assert!(dst.target_size() == 2);
                r.pc = dst.read_word(r, m);
            }
            Jpc(cc, dst) => {
                debug_assert!(dst.target_size() == 2);
                let flag = cc.read_byte(r, m);
                if flag == 1 {
                    r.pc = dst.read_word(r, m);
                }
            }
            Call(dst) => {
                debug_assert!(dst.target_size() == 2);
                let ret = r.pc + 2;
                m.write_word(Addr(r.sp - 2), ret);
                r.sp -= 2;
                r.pc = dst.read_word(r, m);
            }
            Callc(condition, dst) => {
                debug_assert!(dst.target_size() == 2);
                let flag = condition.read_byte(r, m);
                if flag == 1 {
                    let ret = r.pc + 2;
                    m.write_word(Addr(r.sp - 2), ret);
                    r.sp -= 2;
                    r.pc = dst.read_word(r, m);
                }
            }
            Push(src) => {
                debug_assert!(src.target_size() == 2);
                m.write_word(Addr(r.sp - 2), src.read_word(r, m));
                r.sp -= 2;
            }
            Pop(dst) => {
                dst.write_word(r, m, m.read_word(Addr(r.sp)));
                r.sp += 2;
            }
            Rst(dst) => {
                let ret = r.pc;
                m.write_byte(Addr(r.sp - 1), (ret >> 8) as u8);
                m.write_byte(Addr(r.sp - 2), ret as u8);
                r.sp -= 2;
                r.pc = dst as u16;
            }
            Ldhl(op) => {
                let offset = op.read_byte(r, m) as i8;
                let sp = r.sp as i32;
                let result = (sp + offset as i32) as u16;
                r.set_hl(result);
            },
            Invalid(msg) => panic!("Invalid instruction or not implemented: {}", msg),
        }

        if let Some(new_pc) = new_pc {
            r.pc = new_pc;
        } else {
            r.pc += self.bytes as u16;
        }
    }
}