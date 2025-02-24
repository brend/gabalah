use crate::memory::{Ram, Registers, Addr};
use super::ops::Instruction;
use super::map;

/// sets up the Game Boy CPU including registers and memory
/// and runs the program stored therein
pub fn run() {
    let opcode_map = map::build_opcode_map();
    let mut memory = Ram::new();
    let mut registers = Registers::new();

    memory.write_byte(Addr(0), 0x01);
    memory.write_byte(Addr(1), 0x17);
    memory.write_byte(Addr(2), 0x04);
    
    registers.pc = 0;

    for _ in 0..2 {
        let pc = Addr(registers.pc);
        println!("*** pc: {:?}", pc);
        let opcode = memory.read_byte(pc);
        let instruction = Instruction::decode(opcode, &opcode_map).unwrap();
        println!("*** instruction: {:?}", instruction);
        instruction.execute(&mut memory, &mut registers);
        println!("{:?}", registers);
        registers.pc += instruction.bytes as u16;
        break;
    }
}