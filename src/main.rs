mod app;
mod ram;
mod err;

use std::collections::HashMap;

use log::debug;
use ram::{Ram, Registers, Addr};

#[derive(Debug, Clone)]
enum Mnemonic {
    Nop, 
    Ld,
    Inc,
    Dec,
}

#[derive(Debug, Clone)]
enum Location {
    A,
    B,
    C,
    D,
    E,
    H,
    L,
    BC,
    Operand8,
    Operand16,
}

impl Location {
    fn write(&self, registers: &mut Registers, memory: &mut Ram, values: Vec<u8>) {
        debug!("writing [{}] to {:?}", values.iter().map(|n|n.to_string()).collect::<Vec<String>>().join(", "), self);
        match self {
            Location::A => registers.a = values[0],
            Location::BC => registers.set_bc(values[0], values[1]),
            _ => panic!()
        }
    }

    fn read(&self, registers: &Registers, memory: &Ram) -> Vec<u8> {
        match self {
            Location::A => vec![registers.a],
            Location::B => vec![registers.b],
            Location::C => vec![registers.c],
            Location::D => vec![registers.d],
            Location::E => vec![registers.e],
            Location::H => vec![registers.h],
            Location::L => vec![registers.l],
            Location::BC => vec![registers.c, registers.b],
            Location::Operand8 => vec![memory.get(Addr(registers.pc).next().unwrap())],
            Location::Operand16 => {
                let op_pointer = Addr(registers.pc).next().unwrap();
                vec![memory.get(op_pointer), memory.get(op_pointer.next().unwrap())]
            },
        }
    }

    fn len(&self) -> u16 {
        match self {
            Self::Operand8 => 1,
            Self::Operand16 => 2,
            _ => 0
        }
    }
}

#[derive(Debug, Clone)]
enum Instruction {
    Nop,
    Ld(Location, Location),
}

impl Instruction {
    fn decode(opcode: u8, opcode_map: &HashMap<u8, Instruction>) -> Option<Instruction> {
        opcode_map.get(&opcode).cloned()
    }

    fn len(&self) -> u16 {
        1 + match self {
            Self::Ld(dst, src) => dst.len() + src.len(),
            _ => 0
        }
    }

    fn execute(&self, memory: &mut Ram, registers: &mut Registers) {
        match self {
            Self::Nop => (),
            Self::Ld(dst, src) => dst.write(registers, memory, src.read(registers, memory)),
        }
    }
}

fn build_opcode_map() -> HashMap<u8, Instruction> {
    let mut map = HashMap::new();

    map.insert(
        0x00,
        Instruction::Nop,
    );

    map.insert(
        0x01,
        Instruction::Ld(Location::BC, Location::Operand16),
    );

    map.insert(
        0x02,
        Instruction::Ld(Location::BC, Location::A),
    );

    map
}

fn main()  {
    let opcode_map = build_opcode_map();
    let mut memory = Ram::new();
    let mut registers = Registers::new();

    memory.set(Addr(0), 0x01);
    memory.set(Addr(1), 0x17);
    memory.set(Addr(2), 0x04);
    
    registers.pc = 0;

    for _ in 0..2 {
        let pc = Addr(registers.pc);
        println!("*** pc: {:?}", pc);
        let opcode = memory.get(pc);
        let instruction = Instruction::decode(opcode, &opcode_map).unwrap();
        println!("*** instruction: {:?}", instruction);
        instruction.execute(&mut memory, &mut registers);
        println!("{:?}", registers);
        registers.pc += instruction.len();
        break;
    }
}