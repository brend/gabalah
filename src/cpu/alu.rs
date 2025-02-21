use crate::memory::Bytes;

use super::{CARRY_FLAG_BITMASK, HALF_CARRY_FLAG_BITMASK, SUBTRACTION_FLAG_BITMASK, ZERO_FLAG_BITMASK};

pub trait Flags {
    fn zero(&self) -> bool;
    fn set_zero(&mut self, value: bool);
    fn subtraction(&self) -> bool;
    fn set_subtraction(&mut self, value: bool);
    fn half_carry(&self) -> bool;
    fn set_half_carry(&mut self, value: bool);
    fn carry(&self) -> bool;
    fn set_carry(&mut self, value: bool);
}

impl Flags for u8 {
    fn zero(&self) -> bool {
        self & ZERO_FLAG_BITMASK != 0
    }

    fn set_zero(&mut self, value: bool) {
        if value {
            *self |= ZERO_FLAG_BITMASK;
        } else {
            *self &= !ZERO_FLAG_BITMASK;
        }
    }

    fn subtraction(&self) -> bool {
        self & SUBTRACTION_FLAG_BITMASK != 0
    }

    fn set_subtraction(&mut self, value: bool) {
        if value {
            *self |= SUBTRACTION_FLAG_BITMASK;
        } else {
            *self &= !SUBTRACTION_FLAG_BITMASK;
        }
    }

    fn half_carry(&self) -> bool {
        self & HALF_CARRY_FLAG_BITMASK != 0
    }

    fn set_half_carry(&mut self, value: bool) {
        if value {
            *self |= HALF_CARRY_FLAG_BITMASK;
        } else {
            *self &= !HALF_CARRY_FLAG_BITMASK;
        }
    }

    fn carry(&self) -> bool {
        self & CARRY_FLAG_BITMASK != 0
    }

    fn set_carry(&mut self, value: bool) {
        if value {
            *self |= CARRY_FLAG_BITMASK;
        } else {
            *self &= !CARRY_FLAG_BITMASK;
        }
    }
}

pub fn inc(value: &Bytes, flags: &mut u8) -> Bytes {
    match value {
        Bytes::One(value) => {
            let result = value.wrapping_add(1);
            flags.set_zero(result == 0);
            flags.set_subtraction(false);
            flags.set_half_carry((value & 0x0F) + 1 > 0x0F);
            result.into()
        },
        Bytes::Two(value) => {
            let result = value.wrapping_add(1);
            result.into()
        }
    }
}

pub fn dec(value: &Bytes, flags: &mut u8) -> Bytes {
    match value {
        Bytes::One(value) => {
            let result = value.wrapping_sub(1);
            flags.set_zero(result == 0);
            flags.set_subtraction(true);
            flags.set_half_carry((value & 0x0F) == 0);
            result.into()
        },
        Bytes::Two(value) => {
            let result = value.wrapping_sub(1);
            result.into()
        }
    }
}

pub fn add(value1: &Bytes, value2: &Bytes, flags: &mut u8) -> Bytes {
    match (value1, value2) {
        (Bytes::One(value1), Bytes::One(value2)) => {
            let result = value1.wrapping_add(*value2);
            flags.set_zero(result == 0);
            flags.set_subtraction(false);
            flags.set_half_carry((value1 & 0x0F) + (value2 & 0x0F) > 0x0F);
            flags.set_carry((*value1 as u16) + (*value2 as u16) > 0xFF);
            result.into()
        },
        (Bytes::Two(value1), Bytes::Two(value2)) => {
            let result = value1.wrapping_add(*value2);
            flags.set_subtraction(false);
            flags.set_half_carry((value1 & 0x0FFF) + (value2 & 0x0FFF) > 0x0FFF);
            flags.set_carry((*value1 as u32) + (*value2 as u32) > 0xFFFF);
            result.into()
        },
        _ => panic!("Invalid arguments")
    }
}

pub fn adc(value1: &Bytes, value2: &Bytes, flags: &mut u8) -> Bytes {
    match (value1, value2) {
        (Bytes::One(value1), Bytes::One(value2)) => {
            let carry = flags.carry() as u8;
            let result = value1.wrapping_add(*value2).wrapping_add(carry);
            flags.set_zero(result == 0);
            flags.set_subtraction(false);
            flags.set_half_carry((value1 & 0x0F) + (value2 & 0x0F) + carry > 0x0F);
            flags.set_carry((*value1 as u16) + (*value2 as u16) + (carry as u16) > 0xFF);
            result.into()
        },
        (Bytes::Two(value1), Bytes::Two(value2)) => {
            let carry = flags.carry() as u16;
            let result = value1.wrapping_add(*value2).wrapping_add(carry);
            flags.set_subtraction(false);
            flags.set_half_carry((value1 & 0x0FFF) + (value2 & 0x0FFF) + carry > 0x0FFF);
            flags.set_carry((*value1 as u32) + (*value2 as u32) + (carry as u32) > 0xFFFF);
            result.into()
        },
        _ => panic!("Invalid arguments")
    }
}

pub fn sub(value1: &Bytes, value2: &Bytes, flags: &mut u8) -> Bytes {
    match (value1, value2) {
        (Bytes::One(value1), Bytes::One(value2)) => {
            let result = value1.wrapping_sub(*value2);
            flags.set_zero(result == 0);
            flags.set_subtraction(true);
            flags.set_half_carry((value1 & 0x0F) + (value2 & 0x0F) > 0x0F);
            flags.set_carry(*value1 < *value2);
            result.into()
        },
        (Bytes::Two(value1), Bytes::Two(value2)) => {
            let result = value1.wrapping_sub(*value2);
            flags.set_subtraction(true);
            flags.set_half_carry((value1 & 0x0FFF) + (value2 & 0x0FFF) > 0x0FFF);
            flags.set_carry(*value1 < *value2);
            result.into()
        },
        _ => panic!("Invalid arguments")
    }
}

pub fn sbc(value1: &Bytes, value2: &Bytes, flags: &mut u8) -> Bytes {
    match (value1, value2) {
        (Bytes::One(value1), Bytes::One(value2)) => {
            let carry = flags.carry() as u8;
            let result = value1.wrapping_sub(*value2).wrapping_sub(carry);
            flags.set_zero(result == 0);
            flags.set_subtraction(true);
            flags.set_half_carry((value1 & 0x0F) < (value2 & 0x0F) + carry);
            flags.set_carry(*value1 < *value2 + carry);
            result.into()
        },
        (Bytes::Two(value1), Bytes::Two(value2)) => {
            let carry = flags.carry() as u16;
            let result = value1.wrapping_sub(*value2).wrapping_sub(carry);
            flags.set_subtraction(true);
            flags.set_half_carry((value1 & 0x0FFF) < (value2 & 0x0FFF) + carry);
            flags.set_carry(*value1 < *value2 + carry);
            result.into()
        },
        _ => panic!("Invalid arguments")
    }
}

pub fn rlc(value: u8, flags: &mut u8) -> u8 {
    let carry = value & 0x80 != 0;
    let result = (value << 1) | (value >> 7);
    flags.set_zero(false);
    flags.set_subtraction(false);
    flags.set_half_carry(false);
    flags.set_carry(carry);
    result
}

pub fn rrc(value: u8, flags: &mut u8) -> u8 {
    let carry = value & 0x01 != 0;
    let result = (value >> 1) | (value << 7);
    flags.set_zero(false);
    flags.set_subtraction(false);
    flags.set_half_carry(false);
    flags.set_carry(carry);
    result
}

pub fn rl(value: u8, flags: &mut u8) -> u8 {
    let carry = value & 0x80 != 0;
    let result = (value << 1) | (flags.carry() as u8);
    flags.set_zero(false);
    flags.set_subtraction(false);
    flags.set_half_carry(false);
    flags.set_carry(carry);
    result
}

pub fn rr(value: u8, flags: &mut u8) -> u8 {
    let carry = value & 0x01 != 0;
    let result = (value >> 1) | ((flags.carry() as u8) << 7);
    flags.set_zero(false);
    flags.set_subtraction(false);
    flags.set_half_carry(false);
    flags.set_carry(carry);
    result
}

pub fn daa(a: &mut u8, f: &mut u8) {
    if f.subtraction() {
        let mut adjustment: u8 = 0;
        if f.half_carry() {
            adjustment |= 0x06;
        }
        if f.carry() {
            adjustment |= 0x60;
        }
        *a = a.wrapping_sub(adjustment);
    } else {
        let mut adjustment: u8 = 0;
        if f.half_carry() || (*a & 0x0F) > 0x09 {
            adjustment |= 0x06;
        }
        if f.carry() || *a > 0x99 {
            adjustment |= 0x60;
            f.set_carry(true);
        }
        *a = a.wrapping_add(adjustment);
    }
    f.set_zero(*a == 0);
    f.set_half_carry(false);
}

pub fn and(value1: u8, value2: u8, flags: &mut u8) -> u8 {
    let result = value1 & value2;
    flags.set_zero(result == 0);
    flags.set_subtraction(false);
    flags.set_half_carry(true);
    flags.set_carry(false);
    result
}

pub fn xor(value1: u8, value2: u8, flags: &mut u8) -> u8 {
    let result = value1 ^ value2;
    flags.set_zero(result == 0);
    flags.set_subtraction(false);
    flags.set_half_carry(false);
    flags.set_carry(false);
    result
}

pub fn or(value1: u8, value2: u8, flags: &mut u8) -> u8 {
    let result = value1 | value2;
    flags.set_zero(result == 0);
    flags.set_subtraction(false);
    flags.set_half_carry(false);
    flags.set_carry(false);
    result
}

pub fn cp(value1: u8, value2: u8, flags: &mut u8) {
    let result = value1.wrapping_sub(value2);
    flags.set_zero(result == 0);
    flags.set_subtraction(true);
    flags.set_half_carry((value1 & 0x0F) < (value2 & 0x0F));
    flags.set_carry(value1 < value2);
}