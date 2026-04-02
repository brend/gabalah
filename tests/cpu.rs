#[cfg(test)]
mod tests {
    use gabalah::memory::Registers;

    fn setup() -> Registers {
        Registers::default()
    }

    #[test]
    fn test_af_read() {
        let mut registers = setup();
        registers.a = 0x42;
        registers.f = 0x43;
        assert_eq!(registers.af(), 0x4243);
    }

    #[test]
    fn test_set_af_masks_low_flag_bits() {
        let mut registers = setup();
        registers.set_af(0x12FF);
        assert_eq!(registers.a, 0x12);
        assert_eq!(registers.f, 0xF0);
    }
}
