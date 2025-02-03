mod app;
mod ram;
mod err;

use std::collections::HashMap;

use ram::{Ram, Registers, Addr};

#[derive(Debug, Clone)]
enum Mnemonic {
    Nop, 
    Ld,
    Inc,
    Dec,
}

#[derive(Debug, Clone)]
enum Dest {
    None,
    A,
    BC,
}

impl Dest {
    fn write(&self, registers: &mut Registers, memory: &mut Ram, values: Vec<u8>) {
        match self {
            Dest::A => registers.a = values[0],
            Dest::BC => registers.set_bc(values[0], values[1]),
            _ => panic!()
        }
    }

    // fn write16(&self, registers: &mut Registers, lo: u8, hi: u8) {
    //     match self {
    //         Dest::BC => registers.set_bc(lo, hi),
    //         _ => panic!()
    //     }
    // }
}

#[derive(Debug, Clone)]
struct Instruction {
    opcode: u8,
    mnemonic: Mnemonic,
    bytes: i32, 
    dest: Dest,
}

impl Instruction {
    fn decode(opcode: u8, opcode_map: &HashMap<u8, Instruction>) -> Option<Instruction> {
        opcode_map.get(&opcode).cloned()
    }

    fn execute(&self, memory: &mut Ram, registers: &mut Registers) {
        let operands = self.read_operands(memory, Addr(registers.pc));

        match self.mnemonic {
            Mnemonic::Nop => (),
            Mnemonic::Ld => self.dest.write(registers, memory, operands),
            Mnemonic::Inc => self.dest.write(self.src + 1),
            Mnemonic::Dec => (),
        }
    }

    fn read_operands(&self, memory: &Ram, location: Addr) -> Vec<u8> {
        let mut operands = vec![];
        let mut location = location;
        for _ in 1..self.bytes {
            location.inc();
            operands.push(memory.get(location));
        }
        operands
    }
}

fn build_opcode_map() -> HashMap<u8, Instruction> {
    let mut map = HashMap::new();

    map.insert(
        0x00,
        Instruction {
            opcode: 0x00,
            mnemonic: Mnemonic::Nop,
            bytes: 1,
            dest: Dest::None,
        }
    );

    map.insert(
        0x01,
        Instruction {
            opcode: 0x01,
            mnemonic: Mnemonic::Ld,
            bytes: 3,
            dest: Dest::BC,
        }
    );

    map.insert(
        0x02,
        Instruction {
            opcode: 0x02,
            mnemonic: Mnemonic::Ld,
            bytes: 1,
            dest: Dest::A,
        }
    );

    map
}

fn main()  {
    let opcode_map = build_opcode_map();
    let mut memory = Ram::new();
    let mut registers = Registers::new();
    // ... time passes...
    let mut pc = Addr(registers.pc);
    let opcode = memory.get(pc);
    let instruction = Instruction::decode(opcode, &opcode_map).unwrap();
    instruction.execute(&mut memory, &mut registers);
}