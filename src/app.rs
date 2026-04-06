#![deny(clippy::all)]
#![forbid(unsafe_code)]

use super::renderer;
use crate::cpu::Cpu;
use crate::memory::Addr;
use crate::ui::{self, GraphicsBackendKind};
use log::{debug, error};
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    keyboard::KeyCode,
    window::WindowBuilder,
};
use winit_input_helper::WinitInputHelper;

const WIDTH: u32 = 160;
const HEIGHT: u32 = 144;
const SCALE: f64 = 3.0;

// ~70,224 cycles per frame at 4.194304 MHz / 59.7275 fps
const CYCLES_PER_FRAME: usize = 70224;
const FRAME_DURATION: Duration = Duration::from_nanos(16_742_706); // 70224 / 4_194_304 s

pub fn run_loop(cpu: Cpu) -> ui::UiResult<()> {
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();
    let mut input = WinitInputHelper::new();

    let window = {
        let size = LogicalSize::new(WIDTH as f64, HEIGHT as f64);
        let scaled_size = LogicalSize::new(WIDTH as f64 * SCALE, HEIGHT as f64 * SCALE);
        WindowBuilder::new()
            .with_title("Gabalah")
            .with_inner_size(scaled_size)
            .with_min_inner_size(size)
            .build(&event_loop)
            .unwrap()
    };

    let mut graphics = ui::create_backend(GraphicsBackendKind::Pixels, WIDTH, HEIGHT, &window)?;

    let mut emulator = Emulator::new(cpu);
    let mut last_frame = Instant::now();

    let res = event_loop.run(|event, elwt| {
        if let Event::WindowEvent {
            event: WindowEvent::RedrawRequested,
            ..
        } = event
        {
            emulator.draw(graphics.frame_mut());
            emulator.maybe_dump_frame(graphics.frame_mut());
            if let Err(err) = graphics.present() {
                log_error("graphics.present", err.as_ref());
                elwt.exit();
                return;
            }
        }

        if input.update(&event) {
            if input.key_pressed(KeyCode::Escape) || input.close_requested() {
                elwt.exit();
                return;
            }

            // Joypad: (key, is_action_group, bit)
            // Direction bits: 0=Right, 1=Left, 2=Up, 3=Down
            // Action bits:    0=A,     1=B,    2=Select, 3=Start
            let buttons: [(KeyCode, bool, u8); 8] = [
                (KeyCode::ArrowRight, false, 0x01),
                (KeyCode::ArrowLeft, false, 0x02),
                (KeyCode::ArrowUp, false, 0x04),
                (KeyCode::ArrowDown, false, 0x08),
                (KeyCode::KeyZ, true, 0x01),
                (KeyCode::KeyX, true, 0x02),
                (KeyCode::ShiftRight, true, 0x04),
                (KeyCode::Enter, true, 0x08),
            ];
            let mut any_newly_pressed = false;
            for (key, is_action, bit) in buttons {
                if input.key_pressed(key) {
                    if is_action {
                        emulator.cpu.memory.action_buttons |= bit;
                    } else {
                        emulator.cpu.memory.direction_buttons |= bit;
                    }
                    any_newly_pressed = true;
                }
                if input.key_released(key) {
                    if is_action {
                        emulator.cpu.memory.action_buttons &= !bit;
                    } else {
                        emulator.cpu.memory.direction_buttons &= !bit;
                    }
                }
            }
            if any_newly_pressed {
                emulator.cpu.set_if(emulator.cpu.get_if() | 0x10);
            }
            if input.key_pressed(KeyCode::F9) {
                emulator.request_dump();
                window.request_redraw();
            }

            // Resize the window
            if let Some(size) = input.window_resized() {
                if let Err(err) = graphics.resize_surface(size.width, size.height) {
                    log_error("graphics.resize_surface", err.as_ref());
                    elwt.exit();
                    return;
                }
            }

            if last_frame.elapsed() >= FRAME_DURATION {
                last_frame += FRAME_DURATION;
                emulator.step_frame();
                window.request_redraw();
            }
        }
    });
    res.map_err(|e| Box::new(e) as ui::UiError)
}

pub fn run_headless(cpu: Cpu, frames: usize) -> Vec<u8> {
    let mut emulator = Emulator::new(cpu);
    for _ in 0..frames {
        emulator.step_frame();
    }
    emulator.cpu.memory.serial_output.clone()
}

fn log_error(method_name: &str, err: &dyn std::error::Error) {
    error!("{method_name}() failed: {err}");
    let mut source = err.source();
    while let Some(cause) = source {
        error!("  Caused by: {cause}");
        source = cause.source();
    }
}

struct Emulator {
    cpu: Cpu,
    ppu_line_cycles: usize,
    dump_next_frame: bool,
    dump_index: usize,
}

impl Emulator {
    fn new(cpu: Cpu) -> Self {
        Self {
            cpu,
            ppu_line_cycles: 0,
            dump_next_frame: false,
            dump_index: 0,
        }
    }

    /// Runs the CPU for approximately one frame's worth of cycles.
    fn step_frame(&mut self) {
        let mut cycles_this_frame = 0;
        while cycles_this_frame < CYCLES_PER_FRAME {
            let cycles = self.cpu.step();
            cycles_this_frame += cycles;
            self.tick_lcd(cycles);

            if self.cpu.memory.tick(cycles as u32) {
                self.cpu.set_if(self.cpu.get_if() | 0x04);
            }

            if self.is_interrupt_pending() {
                self.interrupt()
            }
        }
    }

    fn tick_lcd(&mut self, cycles: usize) {
        let lcdc = self.cpu.memory.read_byte(Addr(0xFF40));
        if (lcdc & 0x80) == 0 {
            self.ppu_line_cycles = 0;
            self.cpu.memory.set_ly_raw(0);
            self.update_stat(0, false, false);
            return;
        }

        self.ppu_line_cycles += cycles;
        while self.ppu_line_cycles >= 456 {
            self.ppu_line_cycles -= 456;
            let ly = self.cpu.memory.read_byte(Addr(0xFF44));
            let new_ly = if ly >= 153 { 0 } else { ly + 1 };
            self.cpu.memory.set_ly_raw(new_ly);
            if new_ly == 144 {
                self.cpu.set_if(self.cpu.get_if() | 0x01);
            }
        }

        let ly = self.cpu.memory.read_byte(Addr(0xFF44));
        let mode = if ly >= 144 {
            1
        } else if self.ppu_line_cycles < 80 {
            2
        } else if self.ppu_line_cycles < 252 {
            3
        } else {
            0
        };
        let lyc = self.cpu.memory.read_byte(Addr(0xFF45));
        // Keep STAT mode/coincidence bits updated for software polling, but
        // don't assert STAT IRQ yet: current timing granularity is not precise
        // enough and can over-interrupt some games.
        self.update_stat(mode, ly == lyc, false);
    }

    fn update_stat(&mut self, mode: u8, coincidence: bool, allow_interrupt: bool) {
        let old_stat = self.cpu.memory.read_byte(Addr(0xFF41));
        let old_mode = old_stat & 0x03;
        let old_coincidence = (old_stat & 0x04) != 0;
        let mut new_stat = (old_stat & 0x78) | (mode & 0x03);
        if coincidence {
            new_stat |= 0x04;
        }
        self.cpu.memory.set_stat_raw(new_stat);

        if !allow_interrupt {
            return;
        }

        let mode_changed = mode != old_mode;
        let mode_irq = match mode {
            0 => (new_stat & 0x08) != 0,
            1 => (new_stat & 0x10) != 0,
            2 => (new_stat & 0x20) != 0,
            _ => false,
        };
        let lyc_irq = coincidence && !old_coincidence && (new_stat & 0x40) != 0;
        if (mode_changed && mode_irq) || lyc_irq {
            self.cpu.set_if(self.cpu.get_if() | 0x02);
        }
    }

    fn is_interrupt_pending(&self) -> bool {
        self.cpu.registers.ime && (self.cpu.get_ie() & self.cpu.get_if()) != 0
    }

    fn interrupt(&mut self) {
        self.cpu.registers.ime = false;
        let if_contents = self.cpu.get_if();
        let ie_contents = self.cpu.get_ie();
        let pending = if_contents & ie_contents;
        for bit in 0..5u8 {
            if pending & (1 << bit) != 0 {
                self.cpu.set_if(if_contents & !(1 << bit));
                let vector = 0x0040u16 + (bit as u16) * 8;
                self.call(vector);
                return;
            }
        }
    }

    fn call(&mut self, vector: u16) {
        // Emulation remark: this should cost 20 cycles
        self.cpu.memory.write_word(
            Addr(self.cpu.registers.sp.wrapping_sub(2)),
            self.cpu.registers.pc,
        );
        self.cpu.registers.sp = self.cpu.registers.sp.wrapping_sub(2);
        self.cpu.registers.pc = vector;
    }

    /// Renders the current emulator state into the pixel buffer.
    fn draw(&self, screen: &mut [u8]) {
        renderer::render_frame(self.cpu.memory.as_slice(), screen);
    }

    fn request_dump(&mut self) {
        self.dump_next_frame = true;
    }

    fn maybe_dump_frame(&mut self, screen: &mut [u8]) {
        if !self.dump_next_frame {
            return;
        }
        self.dump_next_frame = false;
        if let Err(err) = self.dump_frame_artifacts(screen) {
            error!("debug dump failed: {err}");
        }
    }

    fn dump_frame_artifacts(&mut self, screen: &[u8]) -> std::io::Result<()> {
        let out_dir = PathBuf::from("debug_dumps");
        fs::create_dir_all(&out_dir)?;

        let idx = self.dump_index;
        self.dump_index += 1;
        let stem = format!("frame_{idx:04}");
        let ppm_path = out_dir.join(format!("{stem}.ppm"));
        let txt_path = out_dir.join(format!("{stem}.txt"));
        let vram_path = out_dir.join(format!("{stem}_vram.bin"));
        let oam_path = out_dir.join(format!("{stem}_oam.bin"));

        let mut ppm = File::create(&ppm_path)?;
        write!(ppm, "P6\n{} {}\n255\n", WIDTH, HEIGHT)?;
        for px in screen.chunks_exact(4) {
            ppm.write_all(&px[..3])?;
        }

        let ram = self.cpu.memory.as_slice();
        fs::write(&vram_path, &ram[0x8000..0xA000])?;
        fs::write(&oam_path, &ram[0xFE00..0xFEA0])?;

        let mut txt = File::create(&txt_path)?;
        writeln!(txt, "total_cycles={}", self.cpu.total_cycles)?;
        writeln!(txt, "FF40_LCDC={:02X}", ram[0xFF40])?;
        writeln!(txt, "FF41_STAT={:02X}", ram[0xFF41])?;
        writeln!(txt, "FF42_SCY={:02X}", ram[0xFF42])?;
        writeln!(txt, "FF43_SCX={:02X}", ram[0xFF43])?;
        writeln!(txt, "FF44_LY={:02X}", ram[0xFF44])?;
        writeln!(txt, "FF45_LYC={:02X}", ram[0xFF45])?;
        writeln!(txt, "FF47_BGP={:02X}", ram[0xFF47])?;
        writeln!(txt, "FF48_OBP0={:02X}", ram[0xFF48])?;
        writeln!(txt, "FF49_OBP1={:02X}", ram[0xFF49])?;
        writeln!(txt, "FF4A_WY={:02X}", ram[0xFF4A])?;
        writeln!(txt, "FF4B_WX={:02X}", ram[0xFF4B])?;
        writeln!(txt, "FF0F_IF={:02X}", ram[0xFF0F])?;
        writeln!(txt, "FFFF_IE={:02X}", ram[0xFFFF])?;
        debug!(
            "Wrote debug dump: {}, {}, {}, {}",
            ppm_path.display(),
            txt_path.display(),
            vram_path.display(),
            oam_path.display()
        );
        Ok(())
    }
}
