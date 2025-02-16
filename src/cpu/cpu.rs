use crate::memory::{Ram, Registers, Addr};
use super::ops::Instruction;
use super::ops;

/// sets up the Game Boy CPU including registers and memory
/// and runs the program stored therein
pub fn run() {
    let opcode_map = ops::build_opcode_map();
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
        registers.pc += instruction.bytes as u16;
        break;
    }
}