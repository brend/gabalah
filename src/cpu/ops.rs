use std::vec;

use super::alu::Flags;
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
    pub fn target_size(&self) -> usize {
        match self {
            Operand::Immediate(loc) => loc.target_size(),
            Operand::Indirect(_) => 1,
            Operand::HighMemory(_) => 1,
        }
    }

    /// Reads the location represented by the operand and returns a byte
    pub fn read_byte(&self, registers: &Registers, memory: &Ram) -> u8 {
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

    pub fn read_word(&self, registers: &Registers, memory: &Ram) -> u16 {
        match self {
            Operand::Immediate(loc) => loc.read_word(registers, memory),
            _ => panic!("Invalid operand size"),
        }
    }

    pub fn write_byte(&self, registers: &mut Registers, memory: &mut Ram, value: u8) {
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

    pub fn write_word(&self, registers: &mut Registers, _memory: &mut Ram, value: u16) {
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
    pub mnemonic: Mnemonic,
    /// The length of the instruction in bytes
    pub bytes: usize,
    /// The duration of the instruction in CPU cycles
    pub _cycles: Vec<usize>,
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
}