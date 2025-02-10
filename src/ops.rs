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
    /// The 16-bit register BC
    BC,
    /// The 16-bit register HL
    HL,
    /// The 16-bit register DE
    DE,
    /// The stack pointer
    SP,
    /// An 8-bit constant value
    Const8,
    /// A 16-bit constant value
    Const16,
    /// Nonzero flag
    Nz,
}

use Location::*;

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
            A => registers.a = values[0],
            BC => registers.set_bc(values[0], values[1]),
            _ => panic!()
        }
    }

    /// Reads from the location
    fn read(&self, registers: &Registers, memory: &Ram) -> Bytes {
        match self {
            A => vec![registers.a],
            B => vec![registers.b],
            C => vec![registers.c],
            D => vec![registers.d],
            E => vec![registers.e],
            H => vec![registers.h],
            L => vec![registers.l],
            BC => vec![registers.c, registers.b], // TODO: is this the correct order?
            HL => vec![registers.l, registers.h], // TODO: is this the correct order?
            DE => vec![registers.e, registers.d], // TODO: is this the correct order?
            SP => vec![ram::lo(registers.sp), ram::hi(registers.sp)],
            Const8 => vec![memory.get(Addr(registers.pc).next().unwrap())],
            Const16 => {
                let op_pointer = Addr(registers.pc).next().unwrap();
                vec![memory.get(op_pointer), memory.get(op_pointer.next().unwrap())]
            },
            Nz => vec![if (registers.f & (1 << 7)) != 0 { 0x01 } else { 0x00 }],
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
    _cycles: Vec<usize>,
    /// The operands of the instruction
    operands: Vec<Operand>,
}

impl Instruction {
    /// Creates a new instruction with extended parameters
    fn new_ex(mnemonic: Mnemonic, bytes: usize, cycles: Vec<usize>, operands: Vec<Operand>) -> Instruction {
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
                debug_assert!(self.operands.len() == 2, "ld instruction requires 2 operands");
                let dst = self.operands[0];
                let src = self.operands[1];
                dst.write(r, m, src.read(r, m));
            },
            Inc => {
                debug_assert!(self.operands.len() == 1, "inc instruction requires 1 operand");
                let loc = self.operands[0];
                let bytes = loc.read(r, m);
                todo!("flags");
                loc.write(r, m, add(&bytes, 1));
            }
            Dec => {
                debug_assert!(self.operands.len() == 1, "dec instruction requires 1 operand");
                let loc = self.operands[0];
                let bytes = loc.read(r, m);
                todo!("flags");
                loc.write(r, m, sub(&bytes, 1));
            },
            Add => {
                todo!()
            },
            Rlca => {
                todo!()
            },
            Rrca => todo!(),
            Stop => todo!(),
            Rla => todo!(),
            Jr => todo!(),
            Rra => todo!(),
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

        // jump relative if non-zero
        (0x20, I::new_ex(Jr, 2, vec![12, 8], vec![Nz.imm(), Const8.imm()])), 

        // load nn into HL
        (0x21, I::new(Ld, 3, 12, vec![HL.imm(), Const16.imm()])),

        // load A into [HL]. Increment HL
        // TODO: invent a way to implement this -- new type of operand maybe?
    ]);

    map
}
