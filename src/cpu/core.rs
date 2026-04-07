use std::sync::LazyLock;

use super::alu::Flags;
use super::ops::{CycleSpec, Instruction};
use super::{
    alu, map, Mnemonic, CARRY_FLAG_BITMASK, HALF_CARRY_FLAG_BITMASK, SUBTRACTION_FLAG_BITMASK,
};
use crate::memory::{Addr, Ram, Registers};

use Mnemonic::*;

static OPCODE_MAP: LazyLock<[Instruction; 256]> = LazyLock::new(map::build_opcode_map);

pub struct Cpu {
    pub memory: Ram,
    pub registers: Registers,
    pub total_cycles: u64,
    pending_ime: bool,
    pub halted: bool,
}

impl Default for Cpu {
    fn default() -> Self {
        Self::new()
    }
}

impl Cpu {
    /// Creates a new CPU
    pub fn new() -> Cpu {
        Cpu {
            memory: Ram::new(),
            registers: Registers::new(),
            total_cycles: 0,
            pending_ime: false,
            halted: false,
        }
    }

    /// Loads a program into memory
    pub fn load_rom(&mut self, rom: Vec<u8>) {
        self.memory.load_rom(rom);
    }

    /// Executes the next instruction, returning the number of cycles consumed
    pub fn step(&mut self) -> usize {
        if self.halted {
            let ie = self.get_ie();
            let ifr = self.get_if();
            if (ie & ifr) != 0 {
                self.halted = false;
            }
            self.total_cycles += 4;
            return 4;
        }
        let opcode = self.memory.read_byte(Addr(self.registers.pc));
        if opcode == 0xCB {
            let cb_opcode = self
                .memory
                .read_byte(Addr(self.registers.pc.wrapping_add(1)));
            let cycles = self.execute_cb(cb_opcode);
            self.total_cycles += cycles as u64;
            return cycles;
        }
        let instruction = OPCODE_MAP[opcode as usize];
        self.execute(&instruction)
    }

    pub fn get_ie(&self) -> u8 {
        self.memory.read_ie()
    }

    pub fn get_if(&self) -> u8 {
        self.memory.read_if()
    }

    pub fn raise_if(&mut self, mask: u8) {
        self.memory.raise_if(mask);
    }

    pub fn clear_if(&mut self, mask: u8) {
        self.memory.clear_if(mask);
    }

    /// Executes an instruction, modifying the state of the CPU
    pub fn execute(&mut self, instruction: &Instruction) -> usize {
        let mut new_pc = None;
        let mut conditional_taken = None;
        let if_contents = self.get_if();
        let ie_contents = self.get_ie();
        let r = &mut self.registers;
        let m = &mut self.memory;

        if self.pending_ime {
            self.pending_ime = false;
            r.ime = true;
        }

        match instruction.mnemonic {
            Nop => (),
            Ld8(dst, src) => {
                let byte = src.read_byte(r, m);
                dst.write_byte(r, m, byte);
            }
            Ld16(dst, src) => {
                let word = src.read_word(r, m);
                dst.write_word(r, m, word);
            }
            Inc8(dst) => {
                let byte = dst.read_byte(r, m);
                let increased = alu::inc8(byte, &mut r.f);
                dst.write_byte(r, m, increased);
            }
            Inc16(dst) => {
                let word = dst.read_word(r, m);
                let increased = alu::inc16(word);
                dst.write_word(r, m, increased);
            }
            Dec8(dst) => {
                let byte = dst.read_byte(r, m);
                let decreased = alu::dec8(byte, &mut r.f);
                dst.write_byte(r, m, decreased);
            }
            Dec16(dst) => {
                let word = dst.read_word(r, m);
                let decreased = alu::dec16(word);
                dst.write_word(r, m, decreased);
            }
            Add8(dst, src) => {
                let dst_byte = dst.read_byte(r, m);
                let src_byte = src.read_byte(r, m);
                let sum = alu::add8(dst_byte, src_byte, &mut r.f);
                dst.write_byte(r, m, sum);
            }
            Add16(dst, src) => {
                let dst_word = dst.read_word(r, m);
                let src_word = src.read_word(r, m);
                let sum = alu::add16(dst_word, src_word, &mut r.f);
                dst.write_word(r, m, sum);
            }
            Adc8(dst, src) => {
                let dst_byte = dst.read_byte(r, m);
                let src_byte = src.read_byte(r, m);
                let sum = alu::adc8(dst_byte, src_byte, &mut r.f);
                dst.write_byte(r, m, sum);
            }
            Sub8(dst, src) => {
                let dst_byte = dst.read_byte(r, m);
                let src_byte = src.read_byte(r, m);
                let difference = alu::sub8(dst_byte, src_byte, &mut r.f);
                dst.write_byte(r, m, difference);
            }
            Sbc8(dst, src) => {
                let dst_byte = dst.read_byte(r, m);
                let src_byte = src.read_byte(r, m);
                let difference = alu::sbc8(dst_byte, src_byte, &mut r.f);
                dst.write_byte(r, m, difference);
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
                conditional_taken = Some(false);
                let flag = cc.read_byte(r, m);
                if flag == 1 {
                    conditional_taken = Some(true);
                    let offset = offset.read_byte(r, m) as i8;
                    new_pc = Some((r.pc as i32 + 2 + offset as i32) as u16);
                }
            }
            Daa => alu::daa(&mut r.a, &mut r.f),
            Cpl => {
                r.a = !r.a;
                r.f |= SUBTRACTION_FLAG_BITMASK | HALF_CARRY_FLAG_BITMASK;
            }
            Scf => {
                r.f |= CARRY_FLAG_BITMASK;
                r.f &= !SUBTRACTION_FLAG_BITMASK;
                r.f &= !HALF_CARRY_FLAG_BITMASK;
            }
            Ccf => {
                r.f ^= CARRY_FLAG_BITMASK;
                r.f &= !SUBTRACTION_FLAG_BITMASK;
                r.f &= !HALF_CARRY_FLAG_BITMASK;
            }
            And(dst, src) => {
                let dst_byte = dst.read_byte(r, m);
                let src_byte = src.read_byte(r, m);
                let result = alu::and(dst_byte, src_byte, &mut r.f);
                dst.write_byte(r, m, result);
            }
            Xor(dst, src) => {
                let dst_byte = dst.read_byte(r, m);
                let src_byte = src.read_byte(r, m);
                let result = alu::xor(dst_byte, src_byte, &mut r.f);
                dst.write_byte(r, m, result);
            }
            Or(dst, src) => {
                let dst_byte = dst.read_byte(r, m);
                let src_byte = src.read_byte(r, m);
                let result = alu::or(dst_byte, src_byte, &mut r.f);
                dst.write_byte(r, m, result);
            }
            Cp(dst, src) => {
                let dst_byte = dst.read_byte(r, m);
                let src_byte = src.read_byte(r, m);
                alu::cp(dst_byte, src_byte, &mut r.f);
            }
            Ret => {
                new_pc = Some(m.read_word(Addr(r.sp)));
                r.sp = r.sp.wrapping_add(2);
            }
            Retc(cc) => {
                conditional_taken = Some(false);
                let flag = cc.read_byte(r, m);
                if flag == 1 {
                    conditional_taken = Some(true);
                    new_pc = Some(m.read_word(Addr(r.sp)));
                    r.sp = r.sp.wrapping_add(2);
                }
            }
            Stop(_op) => (),
            Halt => {
                let pending = (ie_contents & if_contents) != 0;
                if pending && !r.ime {
                    // HALT bug: CPU fails to halt; next byte is read twice.
                    // TODO: implement the double-read; for now just continue.
                } else {
                    self.halted = true;
                }
            }
            Reti => {
                new_pc = Some(m.read_word(Addr(r.sp)));
                r.sp = r.sp.wrapping_add(2);
                r.ime = true;
            }
            Ei => self.pending_ime = true,
            Di => r.ime = false,
            Jp(dst) => {
                debug_assert!(dst.target_size() == 2);
                new_pc = Some(dst.read_word(r, m));
            }
            Jpc(cc, dst) => {
                debug_assert!(dst.target_size() == 2);
                conditional_taken = Some(false);
                let flag = cc.read_byte(r, m);
                if flag == 1 {
                    conditional_taken = Some(true);
                    new_pc = Some(dst.read_word(r, m));
                }
            }
            Call(dst) => {
                debug_assert!(dst.target_size() == 2);
                let ret = r.pc.wrapping_add(instruction.bytes as u16);
                m.write_word(Addr(r.sp.wrapping_sub(2)), ret);
                r.sp = r.sp.wrapping_sub(2);
                new_pc = Some(dst.read_word(r, m));
            }
            Callc(condition, dst) => {
                debug_assert!(dst.target_size() == 2);
                conditional_taken = Some(false);
                let flag = condition.read_byte(r, m);
                if flag == 1 {
                    conditional_taken = Some(true);
                    let ret = r.pc.wrapping_add(instruction.bytes as u16);
                    m.write_word(Addr(r.sp.wrapping_sub(2)), ret);
                    r.sp = r.sp.wrapping_sub(2);
                    new_pc = Some(dst.read_word(r, m));
                }
            }
            Push(src) => {
                debug_assert!(src.target_size() == 2);
                m.write_word(Addr(r.sp.wrapping_sub(2)), src.read_word(r, m));
                r.sp = r.sp.wrapping_sub(2);
            }
            Pop(dst) => {
                dst.write_word(r, m, m.read_word(Addr(r.sp)));
                r.sp = r.sp.wrapping_add(2);
            }
            Rst(dst) => {
                let ret = r.pc.wrapping_add(instruction.bytes as u16);
                m.write_byte(Addr(r.sp.wrapping_sub(1)), (ret >> 8) as u8);
                m.write_byte(Addr(r.sp.wrapping_sub(2)), ret as u8);
                r.sp = r.sp.wrapping_sub(2);
                new_pc = Some(dst as u16);
            }
            Ldhl(op) => {
                let offset = op.read_byte(r, m) as i8;
                let imm = offset as u8;
                let result = r.sp.wrapping_add((offset as i16) as u16);
                r.f = 0;
                r.f.set_half_carry((r.sp & 0x000F) + ((imm as u16) & 0x000F) > 0x000F);
                r.f.set_carry((r.sp & 0x00FF) + ((imm as u16) & 0x00FF) > 0x00FF);
                r.set_hl(result);
            }
            AddSp(op) => {
                let offset = op.read_byte(r, m) as i8;
                let imm = offset as u8;
                let result = r.sp.wrapping_add((offset as i16) as u16);
                r.f = 0;
                r.f.set_half_carry((r.sp & 0x000F) + ((imm as u16) & 0x000F) > 0x000F);
                r.f.set_carry((r.sp & 0x00FF) + ((imm as u16) & 0x00FF) > 0x00FF);
                r.sp = result;
            }
            LdHliA => {
                let hl = r.hl();
                m.write_byte(Addr(hl), r.a);
                r.set_hl(hl.wrapping_add(1));
            }
            LdAHli => {
                let hl = r.hl();
                r.a = m.read_byte(Addr(hl));
                r.set_hl(hl.wrapping_add(1));
            }
            LdHldA => {
                let hl = r.hl();
                m.write_byte(Addr(hl), r.a);
                r.set_hl(hl.wrapping_sub(1));
            }
            LdAHld => {
                let hl = r.hl();
                r.a = m.read_byte(Addr(hl));
                r.set_hl(hl.wrapping_sub(1));
            }
            Invalid(msg) => panic!("Invalid instruction or not implemented: {}", msg),
        }

        if let Some(new_pc) = new_pc {
            r.pc = new_pc;
        } else {
            r.pc =
                r.pc.checked_add(instruction.bytes as u16)
                    .unwrap_or_else(|| panic!("PC overflow at {:#06X}", r.pc));
        }

        let cycles = match instruction.cycles {
            CycleSpec::Fixed(single) => single,
            CycleSpec::Branch { taken, not_taken } => {
                if conditional_taken == Some(true) {
                    taken
                } else {
                    not_taken
                }
            }
        };
        self.total_cycles += cycles as u64;
        cycles
    }

    fn read_cb_target(&self, idx: u8) -> u8 {
        let r = &self.registers;
        match idx {
            0 => r.b,
            1 => r.c,
            2 => r.d,
            3 => r.e,
            4 => r.h,
            5 => r.l,
            6 => self.memory.read_byte(Addr(r.hl())),
            7 => r.a,
            _ => unreachable!(),
        }
    }

    fn write_cb_target(&mut self, idx: u8, value: u8) {
        let r = &mut self.registers;
        match idx {
            0 => r.b = value,
            1 => r.c = value,
            2 => r.d = value,
            3 => r.e = value,
            4 => r.h = value,
            5 => r.l = value,
            6 => self.memory.write_byte(Addr(r.hl()), value),
            7 => r.a = value,
            _ => unreachable!(),
        }
    }

    fn execute_cb(&mut self, opcode: u8) -> usize {
        let x = opcode >> 6;
        let y = (opcode >> 3) & 0x07;
        let z = opcode & 0x07;

        match x {
            0 => {
                let value = self.read_cb_target(z);
                let out: u8;
                match y {
                    0 => {
                        // RLC r
                        let carry = (value & 0x80) != 0;
                        out = value.rotate_left(1);
                        self.registers.f = 0;
                        self.registers.f.set_zero(out == 0);
                        self.registers.f.set_carry(carry);
                    }
                    1 => {
                        // RRC r
                        let carry = (value & 0x01) != 0;
                        out = value.rotate_right(1);
                        self.registers.f = 0;
                        self.registers.f.set_zero(out == 0);
                        self.registers.f.set_carry(carry);
                    }
                    2 => {
                        // RL r
                        let carry_in = self.registers.f.carry() as u8;
                        let carry_out = (value & 0x80) != 0;
                        out = (value << 1) | carry_in;
                        self.registers.f = 0;
                        self.registers.f.set_zero(out == 0);
                        self.registers.f.set_carry(carry_out);
                    }
                    3 => {
                        // RR r
                        let carry_in = (self.registers.f.carry() as u8) << 7;
                        let carry_out = (value & 0x01) != 0;
                        out = (value >> 1) | carry_in;
                        self.registers.f = 0;
                        self.registers.f.set_zero(out == 0);
                        self.registers.f.set_carry(carry_out);
                    }
                    4 => {
                        // SLA r
                        let carry = (value & 0x80) != 0;
                        out = value << 1;
                        self.registers.f = 0;
                        self.registers.f.set_zero(out == 0);
                        self.registers.f.set_carry(carry);
                    }
                    5 => {
                        // SRA r
                        let carry = (value & 0x01) != 0;
                        out = (value >> 1) | (value & 0x80);
                        self.registers.f = 0;
                        self.registers.f.set_zero(out == 0);
                        self.registers.f.set_carry(carry);
                    }
                    6 => {
                        // SWAP r
                        out = value.rotate_left(4);
                        self.registers.f = 0;
                        self.registers.f.set_zero(out == 0);
                    }
                    7 => {
                        // SRL r
                        let carry = (value & 0x01) != 0;
                        out = value >> 1;
                        self.registers.f = 0;
                        self.registers.f.set_zero(out == 0);
                        self.registers.f.set_carry(carry);
                    }
                    _ => unreachable!(),
                }
                self.write_cb_target(z, out);
            }
            1 => {
                // BIT b,r
                let value = self.read_cb_target(z);
                let is_set = (value & (1u8 << y)) != 0;
                let carry = self.registers.f.carry();
                self.registers.f = 0;
                self.registers.f.set_zero(!is_set);
                self.registers.f.set_half_carry(true);
                self.registers.f.set_carry(carry);
            }
            2 => {
                // RES b,r
                let value = self.read_cb_target(z);
                self.write_cb_target(z, value & !(1u8 << y));
            }
            3 => {
                // SET b,r
                let value = self.read_cb_target(z);
                self.write_cb_target(z, value | (1u8 << y));
            }
            _ => unreachable!(),
        }

        self.registers.pc = self.registers.pc.wrapping_add(2);
        match (x, z) {
            (1, 6) => 12, // BIT b,(HL)
            (_, 6) => 16, // rotate/shift/res/set on (HL)
            _ => 8,
        }
    }
}
