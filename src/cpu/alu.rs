use crate::memory::Bytes;

trait Flags {
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
        self & 0b1000_0000 != 0
    }

    fn set_zero(&mut self, value: bool) {
        if value {
            *self |= 0b1000_0000;
        } else {
            *self &= 0b0111_1111;
        }
    }

    fn subtraction(&self) -> bool {
        self & 0b0100_0000 != 0
    }

    fn set_subtraction(&mut self, value: bool) {
        if value {
            *self |= 0b0100_0000;
        } else {
            *self &= 0b1011_1111;
        }
    }

    fn half_carry(&self) -> bool {
        self & 0b0010_0000 != 0
    }

    fn set_half_carry(&mut self, value: bool) {
        if value {
            *self |= 0b0010_0000;
        } else {
            *self &= 0b1101_1111;
        }
    }

    fn carry(&self) -> bool {
        self & 0b0001_0000 != 0
    }

    fn set_carry(&mut self, value: bool) {
        if value {
            *self |= 0b0001_0000;
        } else {
            *self &= 0b1110_1111;
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

pub fn sub(value1: &Bytes, value2: &Bytes, flags: &mut u8) -> Bytes {
    match (value1, value2) {
        (Bytes::One(value1), Bytes::One(value2)) => {
            let result = value1.wrapping_sub(*value2);
            flags.set_zero(result == 0);
            flags.set_subtraction(true);
            flags.set_half_carry((value1 & 0x0F) < (value2 & 0x0F));
            flags.set_carry(*value1 < *value2);
            result.into()
        },
        (Bytes::Two(value1), Bytes::Two(value2)) => {
            let result = value1.wrapping_sub(*value2);
            flags.set_subtraction(true);
            flags.set_half_carry((value1 & 0x0FFF) < (value2 & 0x0FFF));
            flags.set_carry(*value1 < *value2);
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