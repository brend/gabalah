use std::collections::HashMap;

use crate::memory::{Ram, Registers, Addr};
use super::ops::Instruction;
use super::{alu, map, Mnemonic, CARRY_FLAG_BITMASK, HALF_CARRY_FLAG_BITMASK, SUBTRACTION_FLAG_BITMASK};

use Mnemonic::*;

pub struct Cpu {
    pub memory: Ram,
    pub registers: Registers,
    opcode_map: HashMap<u8, Instruction>,
}

impl Cpu {
    /// Creates a new CPU
    pub fn new() -> Cpu {
        Cpu {
            memory: Ram::new(),
            registers: Registers::new(),
            opcode_map: map::build_opcode_map(),
        }
    }

    /// Loads a program into memory
    pub fn load_rom(&mut self, rom: Vec<u8>) {
        self.memory.load_rom(rom);
    }

    /// Executes the next instruction
    pub fn step(&mut self) {
        let opcode = self.memory.read_byte(Addr(self.registers.pc));
        let instruction = self.opcode_map.get(&opcode).unwrap().clone();
        self.execute(&instruction);
    }

    /// Executes an instruction, modifying the state of the CPU
    pub fn execute(&mut self, instruction: &Instruction) {
        let mut new_pc = None;
        let r = &mut self.registers;
        let m = &mut self.memory;

        match instruction.mnemonic {
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
                new_pc = Some(m.read_word(Addr(r.sp)));
                r.sp += 2;
            }
            Retc(cc) => {
                let flag = cc.read_byte(r, m);
                if flag == 1 {
                    new_pc = Some(m.read_word(Addr(r.sp)));
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
                new_pc = Some(dst.read_word(r, m));
            }
            Jpc(cc, dst) => {
                debug_assert!(dst.target_size() == 2);
                let flag = cc.read_byte(r, m);
                if flag == 1 {
                    new_pc = Some(dst.read_word(r, m));
                }
            }
            Call(dst) => {
                debug_assert!(dst.target_size() == 2);
                let ret = r.pc + 2;
                m.write_word(Addr(r.sp - 2), ret);
                r.sp -= 2;
                new_pc = Some(dst.read_word(r, m));
            }
            Callc(condition, dst) => {
                debug_assert!(dst.target_size() == 2);
                let flag = condition.read_byte(r, m);
                if flag == 1 {
                    let ret = r.pc + 2;
                    m.write_word(Addr(r.sp - 2), ret);
                    r.sp -= 2;
                    new_pc = Some(dst.read_word(r, m));
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
                new_pc = Some(dst as u16);
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
            r.pc += instruction.bytes as u16;
        }
    }
}
