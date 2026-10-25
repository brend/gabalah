pub type StereoSample = [f32; 2];

const CPU_CLOCK_HZ: u64 = 4_194_304;
const DEFAULT_OUTPUT_HZ: u32 = 48_000;
const REG_START: u16 = 0xFF10;
const REG_END: u16 = 0xFF3F;
const NR52_ADDR: u16 = 0xFF26;
const WAVE_RAM_START: u16 = 0xFF30;
const UNUSED_START: u16 = 0xFF27;
const UNUSED_END: u16 = 0xFF2F;
const REG_COUNT: usize = (REG_END - REG_START + 1) as usize;

#[derive(Debug)]
pub struct Apu {
    regs: [u8; REG_COUNT],
    powered: bool,
    sample_rate_hz: u32,
    sample_phase: u64,
}

impl Default for Apu {
    fn default() -> Self {
        Self::new()
    }
}

impl Apu {
    pub fn new() -> Self {
        let mut apu = Self {
            regs: [0; REG_COUNT],
            powered: true,
            sample_rate_hz: DEFAULT_OUTPUT_HZ,
            sample_phase: 0,
        };
        apu.apply_boot_defaults();
        apu
    }

    pub fn handles_address(address: u16) -> bool {
        (REG_START..=REG_END).contains(&address)
    }

    pub fn read_register(&self, address: u16) -> u8 {
        debug_assert!(Self::handles_address(address));
        if (UNUSED_START..=UNUSED_END).contains(&address) {
            return 0xFF;
        }
        if address == NR52_ADDR {
            let power_bit = if self.powered { 0x80 } else { 0x00 };
            let ch_status = self.regs[self.index(NR52_ADDR)] & 0x0F;
            return power_bit | 0x70 | ch_status;
        }
        self.regs[self.index(address)]
    }

    pub fn write_register(&mut self, address: u16, value: u8) {
        debug_assert!(Self::handles_address(address));

        if (UNUSED_START..=UNUSED_END).contains(&address) {
            return;
        }
        if address == NR52_ADDR {
            let enable = (value & 0x80) != 0;
            if !enable {
                self.powered = false;
                self.clear_non_wave_registers();
                self.regs[self.index(NR52_ADDR)] = 0x00;
                self.sample_phase = 0;
            } else if !self.powered {
                self.powered = true;
                self.regs[self.index(NR52_ADDR)] = 0x80;
            }
            return;
        }
        if !self.powered && address < WAVE_RAM_START {
            return;
        }
        self.regs[self.index(address)] = value;
    }

    pub fn tick(&mut self, cycles: u32, output: &mut Vec<StereoSample>) {
        self.sample_phase += cycles as u64 * self.sample_rate_hz as u64;
        while self.sample_phase >= CPU_CLOCK_HZ {
            self.sample_phase -= CPU_CLOCK_HZ;
            output.push([0.0, 0.0]);
        }
    }

    fn apply_boot_defaults(&mut self) {
        self.regs[self.index(0xFF10)] = 0x80;
        self.regs[self.index(0xFF11)] = 0xBF;
        self.regs[self.index(0xFF12)] = 0xF3;
        self.regs[self.index(0xFF14)] = 0xBF;
        self.regs[self.index(0xFF16)] = 0x3F;
        self.regs[self.index(0xFF17)] = 0x00;
        self.regs[self.index(0xFF19)] = 0xBF;
        self.regs[self.index(0xFF1A)] = 0x7F;
        self.regs[self.index(0xFF1B)] = 0xFF;
        self.regs[self.index(0xFF1C)] = 0x9F;
        self.regs[self.index(0xFF1E)] = 0xBF;
        self.regs[self.index(0xFF20)] = 0xFF;
        self.regs[self.index(0xFF21)] = 0x00;
        self.regs[self.index(0xFF22)] = 0x00;
        self.regs[self.index(0xFF23)] = 0xBF;
        self.regs[self.index(0xFF24)] = 0x77;
        self.regs[self.index(0xFF25)] = 0xF3;
        self.regs[self.index(0xFF26)] = 0xF1;
    }

    fn clear_non_wave_registers(&mut self) {
        for address in REG_START..WAVE_RAM_START {
            self.regs[self.index(address)] = 0;
        }
    }

    fn index(&self, address: u16) -> usize {
        (address - REG_START) as usize
    }
}
