#[cfg(test)]
mod tests {
    use gabalah::memory::{Addr, Ram, Registers};

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

    // --- Joypad ---

    fn joypad_ram() -> Ram {
        Ram::new()
    }

    // Selects a button group by writing to 0xFF00.
    // Bit 5 clear = action group; bit 4 clear = direction group.
    fn select_group(ram: &mut Ram, action: bool, direction: bool) {
        let mut val = 0x30u8; // both groups deselected
        if action {
            val &= !0x20;
        }
        if direction {
            val &= !0x10;
        }
        ram.write_byte(Addr(0xFF00), val);
    }

    #[test]
    fn joypad_no_buttons_pressed_returns_all_high() {
        let mut ram = joypad_ram();
        select_group(&mut ram, true, false);
        let result = ram.read_byte(Addr(0xFF00));
        assert_eq!(
            result & 0x0F,
            0x0F,
            "all action bits should be high when nothing pressed"
        );
    }

    #[test]
    fn joypad_action_a_pressed_bit0_low() {
        let mut ram = joypad_ram();
        ram.action_buttons = 0x01; // A pressed
        select_group(&mut ram, true, false);
        let result = ram.read_byte(Addr(0xFF00));
        assert_eq!(result & 0x01, 0, "A (bit 0) should be low when pressed");
        assert_eq!(result & 0x0E, 0x0E, "other action bits should remain high");
    }

    #[test]
    fn joypad_action_start_pressed_bit3_low() {
        let mut ram = joypad_ram();
        ram.action_buttons = 0x08; // Start pressed
        select_group(&mut ram, true, false);
        let result = ram.read_byte(Addr(0xFF00));
        assert_eq!(result & 0x08, 0, "Start (bit 3) should be low when pressed");
        assert_eq!(result & 0x07, 0x07, "other action bits should remain high");
    }

    #[test]
    fn joypad_direction_right_pressed_bit0_low() {
        let mut ram = joypad_ram();
        ram.direction_buttons = 0x01; // Right pressed
        select_group(&mut ram, false, true);
        let result = ram.read_byte(Addr(0xFF00));
        assert_eq!(result & 0x01, 0, "Right (bit 0) should be low when pressed");
        assert_eq!(
            result & 0x0E,
            0x0E,
            "other direction bits should remain high"
        );
    }

    #[test]
    fn joypad_direction_not_visible_when_action_group_selected() {
        let mut ram = joypad_ram();
        ram.direction_buttons = 0x0F; // all directions pressed
        select_group(&mut ram, true, false); // only action group selected
        let result = ram.read_byte(Addr(0xFF00));
        assert_eq!(
            result & 0x0F,
            0x0F,
            "direction buttons must not bleed into action group read"
        );
    }

    #[test]
    fn joypad_action_not_visible_when_direction_group_selected() {
        let mut ram = joypad_ram();
        ram.action_buttons = 0x0F; // all actions pressed
        select_group(&mut ram, false, true); // only direction group selected
        let result = ram.read_byte(Addr(0xFF00));
        assert_eq!(
            result & 0x0F,
            0x0F,
            "action buttons must not bleed into direction group read"
        );
    }

    #[test]
    fn joypad_both_groups_selected_results_are_anded() {
        let mut ram = joypad_ram();
        ram.action_buttons = 0x01; // A pressed (bit 0 of action)
        ram.direction_buttons = 0x02; // Left pressed (bit 1 of direction)
        select_group(&mut ram, true, true);
        let result = ram.read_byte(Addr(0xFF00));
        // bit 0: A pressed → low; bit 1: Left pressed → low; rest high
        assert_eq!(result & 0x01, 0, "bit 0 low: A pressed in action group");
        assert_eq!(
            result & 0x02,
            0,
            "bit 1 low: Left pressed in direction group"
        );
        assert_eq!(result & 0x0C, 0x0C, "bits 2-3 high: nothing pressed there");
    }

    #[test]
    fn joypad_write_only_stores_select_bits() {
        let mut ram = joypad_ram();
        ram.action_buttons = 0x05;
        // Write with extra bits set — only bits 4-5 should be stored
        ram.write_byte(Addr(0xFF00), 0xFF);
        // With 0xFF written, bits 4 and 5 are set → neither group selected
        let result = ram.read_byte(Addr(0xFF00));
        assert_eq!(
            result & 0x0F,
            0x0F,
            "no group selected: all bits high regardless of pressed buttons"
        );
        assert_eq!(result & 0x30, 0x30, "select bits reflected back");
    }

    #[test]
    fn joypad_upper_bits_always_set() {
        let mut ram = joypad_ram();
        select_group(&mut ram, true, true);
        ram.action_buttons = 0x0F;
        ram.direction_buttons = 0x0F;
        let result = ram.read_byte(Addr(0xFF00));
        assert_eq!(result & 0xC0, 0xC0, "bits 6-7 must always read as 1");
    }

    // --- Timer ---

    #[test]
    fn div_increments_every_256_cycles() {
        let mut ram = Ram::new();
        let initial = ram.read_byte(Addr(0xFF04));
        ram.tick(256);
        assert_eq!(ram.read_byte(Addr(0xFF04)), initial.wrapping_add(1));
        ram.tick(256);
        assert_eq!(ram.read_byte(Addr(0xFF04)), initial.wrapping_add(2));
    }

    #[test]
    fn div_write_resets_to_zero() {
        let mut ram = Ram::new();
        ram.tick(512); // DIV = 2
        ram.write_byte(Addr(0xFF04), 0xFF); // any write resets
        assert_eq!(ram.read_byte(Addr(0xFF04)), 0);
    }

    #[test]
    fn tima_stays_zero_when_timer_disabled() {
        let mut ram = Ram::new();
        ram.write_byte(Addr(0xFF07), 0x00); // TAC: timer disabled
        let overflow = ram.tick(100_000);
        assert!(!overflow);
        assert_eq!(ram.read_byte(Addr(0xFF05)), 0);
    }

    #[test]
    fn tima_increments_at_1024_cycle_rate() {
        let mut ram = Ram::new();
        ram.write_byte(Addr(0xFF07), 0x04); // TAC: enabled, clock select 00 (1024 cycles)
        let overflow = ram.tick(1024);
        assert!(!overflow);
        assert_eq!(ram.read_byte(Addr(0xFF05)), 1);
    }

    #[test]
    fn tima_overflow_reloads_from_tma_and_returns_true() {
        let mut ram = Ram::new();
        ram.write_byte(Addr(0xFF05), 0xFF); // TIMA at max
        ram.write_byte(Addr(0xFF06), 0x42); // TMA reload value
        ram.write_byte(Addr(0xFF07), 0x04); // TAC: enabled, 1024-cycle rate
        let overflow = ram.tick(1024);
        assert!(overflow);
        assert_eq!(ram.read_byte(Addr(0xFF05)), 0x42);
    }

    #[test]
    fn tima_no_overflow_returns_false() {
        let mut ram = Ram::new();
        ram.write_byte(Addr(0xFF07), 0x04); // TAC: enabled, 1024-cycle rate
        let overflow = ram.tick(512); // not enough to increment
        assert!(!overflow);
    }

    // --- OAM DMA ---

    #[test]
    fn dma_copies_160_bytes_to_oam() {
        let mut ram = Ram::new();
        // Write a recognisable pattern starting at 0xC000
        for i in 0..160u8 {
            ram.write_byte(Addr(0xC000 + i as u16), i);
        }
        ram.write_byte(Addr(0xFF46), 0xC0); // trigger DMA from 0xC000
        for i in 0..160u8 {
            assert_eq!(ram.read_byte(Addr(0xFE00 + i as u16)), i, "OAM byte {i}");
        }
    }

    #[test]
    fn dma_from_oam_page_is_stable() {
        let mut ram = Ram::new();
        for i in 0..160u8 {
            ram.write_byte(Addr(0xFE00 + i as u16), i ^ 0x5A);
        }

        ram.write_byte(Addr(0xFF46), 0xFE); // trigger DMA from 0xFE00 (OAM page)

        for i in 0..160u8 {
            assert_eq!(
                ram.read_byte(Addr(0xFE00 + i as u16)),
                i ^ 0x5A,
                "OAM byte {i}"
            );
        }
    }

    // --- LCD IO semantics ---

    #[test]
    fn ly_write_resets_to_zero() {
        let mut ram = Ram::new();
        ram.write_byte(Addr(0xFF44), 0x77);
        assert_eq!(ram.read_byte(Addr(0xFF44)), 0);
    }

    #[test]
    fn stat_write_preserves_mode_and_coincidence_bits() {
        let mut ram = Ram::new();
        ram.set_stat_raw(0x87); // mode=3, coincidence=1
        ram.write_byte(Addr(0xFF41), 0x00); // clear writable bits
        let stat = ram.read_byte(Addr(0xFF41));
        assert_eq!(stat & 0x80, 0x80, "STAT bit 7 should stay set");
        assert_eq!(
            stat & 0x07,
            0x07,
            "mode/coincidence bits should be preserved"
        );
    }

    // --- Memory map behavior ---

    #[test]
    fn writes_to_rom_are_ignored() {
        let mut ram = Ram::new();
        ram.load_rom(vec![0u8; 32 * 1024]);
        let before = ram.read_byte(Addr(0x1234));
        ram.write_byte(Addr(0x1234), before.wrapping_add(1));
        assert_eq!(ram.read_byte(Addr(0x1234)), before);
    }

    #[test]
    fn echo_ram_reads_and_writes_map_to_work_ram() {
        let mut ram = Ram::new();
        ram.write_byte(Addr(0xC123), 0x42);
        assert_eq!(ram.read_byte(Addr(0xE123)), 0x42);
        ram.write_byte(Addr(0xE123), 0x99);
        assert_eq!(ram.read_byte(Addr(0xC123)), 0x99);
    }

    #[test]
    fn unusable_memory_reads_ff_and_ignores_writes() {
        let mut ram = Ram::new();
        ram.write_byte(Addr(0xFEA0), 0x12);
        assert_eq!(ram.read_byte(Addr(0xFEA0)), 0xFF);
    }
}
