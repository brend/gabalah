use crate::memory::{Bytes, Flags};

pub fn inc(value: &Bytes, flags: Flags) -> (Bytes, Flags) {
    match value {
        Bytes::One(value) => {
            let result = value.wrapping_add(1);
            let flags = Flags {
                zero: result == 0,
                subtraction: false,
                half_carry: (value & 0x0F) + 1 > 0x0F,
                ..flags
            };
            (result.into(), flags)
        },
        Bytes::Two(value) => {
            let result = value.wrapping_add(1);
            (result.into(), flags)
        }
    }
}

pub fn dec(value: &Bytes, flags: Flags) -> (Bytes, Flags) {
    match value {
        Bytes::One(value) => {
            let result = value.wrapping_sub(1);
            let flags = Flags {
                zero: result == 0,
                subtraction: true,
                half_carry: (value & 0x0F) == 0,
                ..flags
            };
            (result.into(), flags)
        },
        Bytes::Two(value) => {
            let result = value.wrapping_sub(1);
            (result.into(), flags)
        }
    }
}

pub fn add(value1: &Bytes, value2: &Bytes, flags: Flags) -> (Bytes, Flags) {
    match (value1, value2) {
        (Bytes::One(value1), Bytes::One(value2)) => {
            let result = value1.wrapping_add(*value2);
            let flags = Flags {
                zero: result == 0,
                subtraction: false,
                half_carry: (value1 & 0x0F) + (value2 & 0x0F) > 0x0F,
                carry: (*value1 as u16) + (*value2 as u16) > 0xFF,
            };
            (result.into(), flags)
        },
        (Bytes::Two(value1), Bytes::Two(value2)) => {
            let result = value1.wrapping_add(*value2);
            let flags = Flags {
                subtraction: false,
                half_carry: (value1 & 0x0FFF) + (value2 & 0x0FFF) > 0x0FFF,
                carry: (*value1 as u32) + (*value2 as u32) > 0xFFFF,
                ..flags
            };
            (result.into(), flags)
        },
        _ => panic!("Invalid arguments")
    }
}

pub fn sub(value1: &Bytes, value2: &Bytes, flags: Flags) -> (Bytes, Flags) {
    match (value1, value2) {
        (Bytes::One(value1), Bytes::One(value2)) => {
            let result = value1.wrapping_sub(*value2);
            let flags = Flags {
                zero: result == 0,
                subtraction: true,
                half_carry: (value1 & 0x0F) + (value2 & 0x0F) > 0x0F,
                carry: *value1 < *value2,
            };
            (result.into(), flags)
        },
        (Bytes::Two(value1), Bytes::Two(value2)) => {
            let result = value1.wrapping_sub(*value2);
            let flags = Flags {
                subtraction: true,
                half_carry: (value1 & 0x0FFF) < (*value2 & 0x0FFF),
                carry: *value1 < *value2,
                ..flags
            };
            (result.into(), flags)
        },
        _ => panic!("Invalid arguments")
    }
}

pub fn rlc(value: u8) -> (u8, Flags) {
    let carry = value & 0x80 != 0;
    let result = (value << 1) | (value >> 7);
    let flags = Flags {
        zero: false,
        subtraction: false,
        half_carry: false,
        carry,
    };
    (result, flags)
}

pub fn rrc(value: u8) -> (u8, Flags) {
    let carry = value & 0x01 != 0;
    let result = (value >> 1) | (value << 7);
    let flags = Flags {
        zero: false,
        subtraction: false,
        half_carry: false,
        carry,
    };
    (result, flags)
}

pub fn rl(value: u8, flags: Flags) -> (u8, Flags) {
    let carry = value & 0x80 != 0;
    let result = (value << 1) | (flags.carry as u8);
    let flags = Flags {
        zero: false,
        subtraction: false,
        half_carry: false,
        carry,
    };
    (result, flags)
}