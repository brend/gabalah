use std::{collections::HashMap, vec};

use log::debug;

use crate::ram;
use crate::ram::{Registers, Ram, Addr};

type Bytes = Vec<u8>;

/// Assembly instruction mnemonics
#[derive(Debug, Clone, Copy)]
enum Mnemonic {
    /// The no-operation
    Nop, 
    /// Load data into a location
    Ld,
    /// Increase the target
    Inc,
    /// Decrease the target
    Dec,
    /// Rotate A left and carry
    Rlca,
    /// Addition
    Add,
}

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
    /// The 16-bit register BC
    BC,
    /// The 16-bit register HL
    HL,
    /// The stack pointer
    SP,
    /// An 8-bit constant value
    Const8,
    /// A 16-bit constant value
    Const16,
}

impl Location {
    /// Creates an immediate value operand from the location
    fn imm(&self) -> Operand {
        Operand::Immediate(*self)
    }

    /// Creates an indirectly referenced (memory) operand from the location
    fn mem(&self) -> Operand {
        Operand::Memory(*self)
    }

    /// Writes to the location
    fn write(&self, registers: &mut Registers, memory: &mut Ram, values: Bytes) {
        debug!("writing [{}] to {:?}", values.iter().map(|n|n.to_string()).collect::<Vec<String>>().join(", "), self);
        match self {
            Location::A => registers.a = values[0],
            Location::BC => registers.set_bc(values[0], values[1]),
            _ => panic!()
        }
    }

    /// Reads from the location
    fn read(&self, registers: &Registers, memory: &Ram) -> Bytes {
        match self {
            Location::A => vec![registers.a],
            Location::B => vec![registers.b],
            Location::C => vec![registers.c],
            Location::D => vec![registers.d],
            Location::E => vec![registers.e],
            Location::H => vec![registers.h],
            Location::L => vec![registers.l],
            Location::BC => vec![registers.c, registers.b], // TODO: is this the correct order?
            Location::HL => vec![registers.l, registers.h], // TODO: is this the correct order?
            Location::SP => vec![ram::lo(registers.sp), ram::hi(registers.sp)],
            Location::Const8 => vec![memory.get(Addr(registers.pc).next().unwrap())],
            Location::Const16 => {
                let op_pointer = Addr(registers.pc).next().unwrap();
                vec![memory.get(op_pointer), memory.get(op_pointer.next().unwrap())]
            },
        }
    }
}

/// An operand of a CPU instruction
#[derive(Debug, Clone, Copy)]
enum Operand {
    /// An immediate value at a given location
    Immediate(Location),
    /// An value indirectly references by the address stored at the given location
    Memory(Location),
}

impl Operand {
    /// Reads the location represented by the operand
    fn read(&self, registers: &Registers, memory: &Ram) -> Bytes {
        match self {
            Operand::Immediate(location) => location.read(registers, memory),
            Operand::Memory(location) => {
                let addr_bytes = location.read(registers, memory);
                let addr = Addr::from_bytes(addr_bytes);
                vec![memory.get(addr)]
            }
        }
    }

    /// Writes to the location represented by the operand 
    fn write(&self, registers: &mut Registers, memory: &mut Ram, values: Bytes) {
        match self {
            Operand::Immediate(location) => location.write(registers, memory, values),
            Operand::Memory(location) => {
                let addr_bytes = location.read(registers, memory);
                let addr = Addr::from_bytes(addr_bytes);
                memory.set_word(addr, &values)
            }
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
    _cycles: usize,
    /// The operands of the instruction
    operands: Vec<Operand>,
}

impl Instruction {
    /// Creates a new instruction
    fn new(mnemonic: Mnemonic, bytes: usize, cycles: usize, operands: Vec<Operand>) -> Instruction {
        Instruction {
            mnemonic, 
            bytes,
            _cycles: cycles,
            operands,
        }
    }

    /// Decodes an instruction from its opcode and the provided opcode map
    pub fn decode(opcode: u8, opcode_map: &HashMap<u8, Instruction>) -> Option<Instruction> {
        opcode_map.get(&opcode).cloned()
    }

    /// Executes the instruction, modifying the state of the CPU
    pub fn execute(&self, m: &mut Ram, r: &mut Registers) {
        match self.mnemonic {
            Mnemonic::Nop => (),
            Mnemonic::Ld => {
                debug_assert!(self.operands.len() == 2, "ld instruction requires 2 operands");
                let dst = self.operands[0];
                let src = self.operands[1];
                dst.write(r, m, src.read(r, m));
            },
            Mnemonic::Inc => {
                debug_assert!(self.operands.len() == 1, "inc instruction requires 1 operand");
                let loc = self.operands[0];
                let bytes = loc.read(r, m);
                todo!("flags");
                loc.write(r, m, add(&bytes, 1));
            }
            Mnemonic::Dec => {
                debug_assert!(self.operands.len() == 1, "dec instruction requires 1 operand");
                let loc = self.operands[0];
                let bytes = loc.read(r, m);
                todo!("flags");
                loc.write(r, m, sub(&bytes, 1));
            },
            Mnemonic::Add => {
                todo!()
            },
            Mnemonic::Rlca => {
                todo!()
            }
        }
    }
}

fn add(a: &Bytes, b: u8) -> Bytes {
    debug_assert!(a.len() > 0 && a.len() < 3);
    if a.len() == 1 {
        vec![add8(a[0], b)]
    } else {
        add16(a, &vec![b, 0])
    }
}

fn add8(a: u8, b: u8) -> u8 {
    ((a as u16 + b as u16) % 256) as u8
}

fn add16(a: &Bytes, b: &Bytes) -> Bytes {
    todo!()
}

fn sub(a: &Bytes, b: u8) -> Bytes {
    todo!()
}

/// Builds and returns a mapping of the 8-bit opcodes to instruction instances
pub fn build_opcode_map() -> HashMap<u8, Instruction> {
    let mut map = HashMap::new();

    // no-op
    map.insert(
        0x00,
        Instruction::new(Mnemonic::Nop, 1, 4, vec![])
    );

    // load nn into BC
    map.insert(
        0x01,
        Instruction::new(Mnemonic::Ld, 3, 12, vec![Location::BC.imm(), Location::Const16.imm()])
    );

    // load A into [BC]
    map.insert(
        0x02,
        Instruction::new(Mnemonic::Ld, 1, 8, vec![Location::BC.mem(), Location::A.imm()])
    );

    // increase BC
    map.insert(
        0x03,
        Instruction::new(Mnemonic::Inc, 1, 8, vec![Location::BC.imm()])
    );

    // increase B
    map.insert(
        0x04,
        Instruction::new(Mnemonic::Inc, 1, 4, vec![Location::B.imm()])
    );

    // decrease B
    map.insert(
        0x05,
        Instruction::new(Mnemonic::Dec, 1, 4, vec![Location::B.imm()])
    );

    // load n into B
    map.insert(
        0x06,
        Instruction::new(Mnemonic::Ld, 2, 8, vec![Location::B.imm(), Location::Const8.imm()])
    );

    // rotate A left; old bit 7 to Carry flag.
    map.insert(
        0x07,
        Instruction::new(Mnemonic::Rlca, 1, 4, vec![])
    );

    // load SP into [nn]
    map.insert(
        0x08,
        Instruction::new(Mnemonic::Ld, 3, 20, vec![Location::Const16.mem(), Location::SP.imm()])
    );

    // add BC to HL
    map.insert(
        0x09,
        Instruction::new(Mnemonic::Add, 1, 8, vec![Location::HL.imm(), Location::BC.imm()])
    );

    // load BC into A
    map.insert(
        0x0A, 
        Instruction::new(Mnemonic::Ld, 1, 8, vec![Location::A.imm(), Location::BC.mem()])
    );

    map
}
