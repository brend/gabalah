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
}