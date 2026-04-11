#![deny(clippy::all)]
#![forbid(unsafe_code)]

use super::renderer;
use crate::config;
use crate::config::{Controls, DebugDumpSettings};
use crate::cpu::Cpu;
use crate::memory::Addr;
use crate::ui::{self, GraphicsBackendKind, GraphicsOptions};
use log::{debug, error, warn};
use std::fs::{self, File};
use std::io::Write;
use std::time::{Duration, Instant};
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::KeyCode,
    window::{Icon, WindowBuilder},
};
use winit_input_helper::WinitInputHelper;

const WIDTH: u32 = 160;
const HEIGHT: u32 = 144;
const WINDOW_ICON_SIDE: u32 = 64;
const WINDOW_ICON_RGBA: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/icons/window-icon-64.rgba"
));
// ~70,224 cycles per frame at 4.194304 MHz / 59.7275 fps
const CYCLES_PER_FRAME: usize = 70224;
const FRAME_DURATION: Duration = Duration::from_nanos(16_742_706); // 70224 / 4_194_304 s
const INTERRUPT_SERVICE_CYCLES: usize = 20;
const SHADER_NAME_OVERLAY_DURATION: Duration = Duration::from_secs(3);
const FALLBACK_SHADER_NAME: &str = "builtin-crt";

pub fn run_loop(
    cpu: Cpu,
    backend_kind: GraphicsBackendKind,
    backend_options: GraphicsOptions,
    window_scale: f64,
    controls: Controls,
    debug_dump_settings: DebugDumpSettings,
) -> ui::UiResult<()> {
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();
    let mut input = WinitInputHelper::new();

    let window = {
        let size = LogicalSize::new(WIDTH as f64, HEIGHT as f64);
        let scaled_size =
            LogicalSize::new(WIDTH as f64 * window_scale, HEIGHT as f64 * window_scale);
        WindowBuilder::new()
            .with_title("Gabalah")
            .with_inner_size(scaled_size)
            .with_min_inner_size(size)
            .with_window_icon(load_window_icon())
            .build(&event_loop)
            .unwrap()
    };

    let mut graphics = ui::create_backend(backend_kind, WIDTH, HEIGHT, &window, backend_options)?;
    debug!("Using graphics backend '{}'", backend_kind.as_str());

    let mut emulator = Emulator::new(cpu, debug_dump_settings);
    let mut last_frame = Instant::now();
    let mut shader_overlay = ShaderOverlay::default();

    let res = event_loop.run(|event, elwt| {
        elwt.set_control_flow(ControlFlow::WaitUntil(last_frame + FRAME_DURATION));

        if let Event::WindowEvent {
            event: WindowEvent::RedrawRequested,
            ..
        } = event
        {
            let frame = graphics.frame_mut();
            emulator.draw(frame);
            shader_overlay.draw_if_visible(frame);
            emulator.maybe_dump_frame(frame);
            if let Err(err) = graphics.present() {
                log_error("graphics.present", err.as_ref());
                elwt.exit();
                return;
            }
        }

        if input.update(&event) {
            if input.key_pressed(controls.hotkeys.exit) || input.close_requested() {
                elwt.exit();
                return;
            }

            // Joypad: (key, is_action_group, bit)
            // Direction bits: 0=Right, 1=Left, 2=Up, 3=Down
            // Action bits:    0=A,     1=B,    2=Select, 3=Start
            let buttons: [(KeyCode, bool, u8); 8] = [
                (controls.joypad.right, false, 0x01),
                (controls.joypad.left, false, 0x02),
                (controls.joypad.up, false, 0x04),
                (controls.joypad.down, false, 0x08),
                (controls.joypad.a, true, 0x01),
                (controls.joypad.b, true, 0x02),
                (controls.joypad.select, true, 0x04),
                (controls.joypad.start, true, 0x08),
            ];
            let mut any_newly_pressed = false;
            for (key, is_action, bit) in buttons {
                if input.key_pressed(key) {
                    if is_action {
                        emulator.cpu.set_action_button_pressed(bit, true);
                    } else {
                        emulator.cpu.set_direction_button_pressed(bit, true);
                    }
                    any_newly_pressed = true;
                }
                if input.key_released(key) {
                    if is_action {
                        emulator.cpu.set_action_button_pressed(bit, false);
                    } else {
                        emulator.cpu.set_direction_button_pressed(bit, false);
                    }
                }
            }
            if any_newly_pressed {
                emulator.cpu.raise_if(0x10);
            }
            if input.key_pressed(controls.hotkeys.debug_frame_dump) {
                emulator.request_dump();
                window.request_redraw();
            }
            if backend_kind == GraphicsBackendKind::WgpuShader
                && input.key_pressed(controls.hotkeys.next_shader)
            {
                match graphics.cycle_shader_next() {
                    Ok(active_shader_file) => {
                        if let Err(err) =
                            config::save_active_shader_file(active_shader_file.as_deref())
                        {
                            warn!("Failed to persist active shader in config.json: {err}");
                        }
                        shader_overlay.show(active_shader_file);
                        window.request_redraw();
                    }
                    Err(err) => {
                        log_error("graphics.cycle_shader_next", err.as_ref());
                        elwt.exit();
                        return;
                    }
                }
            }
            if backend_kind == GraphicsBackendKind::WgpuShader
                && input.key_pressed(controls.hotkeys.previous_shader)
            {
                match graphics.cycle_shader_prev() {
                    Ok(active_shader_file) => {
                        if let Err(err) =
                            config::save_active_shader_file(active_shader_file.as_deref())
                        {
                            warn!("Failed to persist active shader in config.json: {err}");
                        }
                        shader_overlay.show(active_shader_file);
                        window.request_redraw();
                    }
                    Err(err) => {
                        log_error("graphics.cycle_shader_prev", err.as_ref());
                        elwt.exit();
                        return;
                    }
                }
            }
            if input.key_pressed(controls.hotkeys.reload_graphics_config) {
                match (
                    config::load_graphics_settings(),
                    config::load_debug_dump_settings(),
                ) {
                    (Ok((configured_backend, configured_options)), Ok(configured_debug_dump)) => {
                        if configured_backend != backend_kind {
                            warn!(
                                "config reload ignored backend change: running='{}' configured='{}'",
                                backend_kind.as_str(),
                                configured_backend.as_str()
                            );
                        }
                        emulator.reload_debug_dump_settings(configured_debug_dump);
                        let preferred_active_file =
                            configured_options.shader.active_file.clone();
                        if let Err(err) = graphics.reload_options(configured_options) {
                            log_error("graphics.reload_options", err.as_ref());
                            elwt.exit();
                            return;
                        }
                        match graphics.reload_shader_library(preferred_active_file.as_deref()) {
                            Ok(active_shader_file) => {
                                if let Err(err) = config::save_active_shader_file(active_shader_file.as_deref()) {
                                    warn!("Failed to persist active shader in config.json: {err}");
                                }
                            }
                            Err(err) => {
                                log_error("graphics.reload_shader_library", err.as_ref());
                                elwt.exit();
                                return;
                            }
                        }
                        debug!("Reloaded graphics options from config.json");
                        window.request_redraw();
                    }
                    (Err(err), _) => {
                        log_error("config.load_graphics_settings", err.as_ref());
                    }
                    (_, Err(err)) => {
                        log_error("config.load_debug_dump_settings", err.as_ref());
                    }
                }
            }

            // Resize the window
            if let Some(size) = input.window_resized() {
                if let Err(err) = graphics.resize_surface(size.width, size.height) {
                    log_error("graphics.resize_surface", err.as_ref());
                    elwt.exit();
                    return;
                }
            }

            let mut stepped = false;
            while last_frame.elapsed() >= FRAME_DURATION {
                last_frame += FRAME_DURATION;
                emulator.step_frame();
                stepped = true;
            }
            if stepped {
                window.request_redraw();
            }
        }
    });
    res.map_err(|e| Box::new(e) as ui::UiError)
}

pub fn run_headless(cpu: Cpu, frames: usize) -> Vec<u8> {
    let mut emulator = Emulator::new(cpu, DebugDumpSettings::default());
    for _ in 0..frames {
        emulator.step_frame();
    }
    emulator.cpu.serial_output().to_vec()
}

fn load_window_icon() -> Option<Icon> {
    Icon::from_rgba(
        WINDOW_ICON_RGBA.to_vec(),
        WINDOW_ICON_SIDE,
        WINDOW_ICON_SIDE,
    )
    .ok()
}

fn log_error(method_name: &str, err: &dyn std::error::Error) {
    error!("{method_name}() failed: {err}");
    let mut source = err.source();
    while let Some(cause) = source {
        error!("  Caused by: {cause}");
        source = cause.source();
    }
}

#[derive(Default)]
struct ShaderOverlay {
    text: Option<String>,
    visible_until: Option<Instant>,
}

impl ShaderOverlay {
    fn show(&mut self, active_shader_file: Option<String>) {
        self.text = Some(active_shader_file.unwrap_or_else(|| FALLBACK_SHADER_NAME.to_string()));
        self.visible_until = Some(Instant::now() + SHADER_NAME_OVERLAY_DURATION);
    }

    fn draw_if_visible(&mut self, screen: &mut [u8]) {
        let now = Instant::now();
        let (Some(text), Some(until)) = (self.text.as_deref(), self.visible_until) else {
            return;
        };
        if now > until {
            self.text = None;
            self.visible_until = None;
            return;
        }
        draw_overlay_text(screen, text);
    }
}

fn draw_overlay_text(screen: &mut [u8], text: &str) {
    if screen.len() != (WIDTH * HEIGHT * 4) as usize {
        return;
    }
    let margin_x = 4u32;
    let margin_y = 4u32;
    let char_w = 5u32;
    let char_h = 7u32;
    let spacing = 1u32;
    let max_chars = ((WIDTH - margin_x * 2) / (char_w + spacing)) as usize;
    let text = clip_overlay_text(text, max_chars);
    if text.is_empty() {
        return;
    }

    let text_width = text.chars().count() as u32 * (char_w + spacing) - spacing;
    let bg_w = text_width + 6;
    let bg_h = char_h + 6;
    fill_rect_blend(
        screen,
        margin_x.saturating_sub(2),
        margin_y.saturating_sub(2),
        bg_w,
        bg_h,
        [0, 0, 0],
        180,
    );

    let mut x = margin_x;
    for ch in text.chars() {
        draw_char_5x7(screen, x + 1, margin_y + 1, ch, [0, 0, 0], 190);
        draw_char_5x7(screen, x, margin_y, ch, [244, 252, 244], 255);
        x += char_w + spacing;
    }
}

fn clip_overlay_text(text: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }
    let upper = text.to_ascii_uppercase();
    let upper_len = upper.chars().count();
    if upper_len <= max_chars {
        return upper;
    }
    if max_chars <= 3 {
        return ".".repeat(max_chars);
    }
    let mut clipped: String = upper.chars().take(max_chars - 3).collect();
    clipped.push_str("...");
    clipped
}

fn fill_rect_blend(
    screen: &mut [u8],
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    color: [u8; 3],
    alpha: u8,
) {
    let x_end = (x + width).min(WIDTH);
    let y_end = (y + height).min(HEIGHT);
    for py in y..y_end {
        for px in x..x_end {
            blend_pixel(screen, px, py, color, alpha);
        }
    }
}

fn draw_char_5x7(screen: &mut [u8], x: u32, y: u32, ch: char, color: [u8; 3], alpha: u8) {
    let glyph = glyph_5x7(ch);
    for (row, bits) in glyph.into_iter().enumerate() {
        for col in 0..5u32 {
            if (bits & (1 << (4 - col))) != 0 {
                blend_pixel(screen, x + col, y + row as u32, color, alpha);
            }
        }
    }
}

fn blend_pixel(screen: &mut [u8], x: u32, y: u32, color: [u8; 3], alpha: u8) {
    if x >= WIDTH || y >= HEIGHT {
        return;
    }
    let idx = ((y * WIDTH + x) * 4) as usize;
    for c in 0..3 {
        let dst = screen[idx + c] as u16;
        let src = color[c] as u16;
        let a = alpha as u16;
        let blended = (dst * (255 - a) + src * a) / 255;
        screen[idx + c] = blended as u8;
    }
    screen[idx + 3] = 0xFF;
}

fn glyph_5x7(ch: char) -> [u8; 7] {
    match ch {
        'A' => [0x0E, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        'B' => [0x1E, 0x11, 0x11, 0x1E, 0x11, 0x11, 0x1E],
        'C' => [0x0E, 0x11, 0x10, 0x10, 0x10, 0x11, 0x0E],
        'D' => [0x1E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x1E],
        'E' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x1F],
        'F' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x10],
        'G' => [0x0E, 0x11, 0x10, 0x10, 0x13, 0x11, 0x0F],
        'H' => [0x11, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        'I' => [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x1F],
        'J' => [0x01, 0x01, 0x01, 0x01, 0x11, 0x11, 0x0E],
        'K' => [0x11, 0x12, 0x14, 0x18, 0x14, 0x12, 0x11],
        'L' => [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1F],
        'M' => [0x11, 0x1B, 0x15, 0x15, 0x11, 0x11, 0x11],
        'N' => [0x11, 0x19, 0x15, 0x13, 0x11, 0x11, 0x11],
        'O' => [0x0E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        'P' => [0x1E, 0x11, 0x11, 0x1E, 0x10, 0x10, 0x10],
        'Q' => [0x0E, 0x11, 0x11, 0x11, 0x15, 0x12, 0x0D],
        'R' => [0x1E, 0x11, 0x11, 0x1E, 0x14, 0x12, 0x11],
        'S' => [0x0F, 0x10, 0x10, 0x0E, 0x01, 0x01, 0x1E],
        'T' => [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04],
        'U' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        'V' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x0A, 0x04],
        'W' => [0x11, 0x11, 0x11, 0x15, 0x15, 0x15, 0x0A],
        'X' => [0x11, 0x11, 0x0A, 0x04, 0x0A, 0x11, 0x11],
        'Y' => [0x11, 0x11, 0x0A, 0x04, 0x04, 0x04, 0x04],
        'Z' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x10, 0x1F],
        '0' => [0x0E, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0E],
        '1' => [0x04, 0x0C, 0x14, 0x04, 0x04, 0x04, 0x1F],
        '2' => [0x0E, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1F],
        '3' => [0x1E, 0x01, 0x01, 0x06, 0x01, 0x01, 0x1E],
        '4' => [0x02, 0x06, 0x0A, 0x12, 0x1F, 0x02, 0x02],
        '5' => [0x1F, 0x10, 0x10, 0x1E, 0x01, 0x01, 0x1E],
        '6' => [0x0E, 0x10, 0x10, 0x1E, 0x11, 0x11, 0x0E],
        '7' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08],
        '8' => [0x0E, 0x11, 0x11, 0x0E, 0x11, 0x11, 0x0E],
        '9' => [0x0E, 0x11, 0x11, 0x0F, 0x01, 0x01, 0x0E],
        '.' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x0C, 0x0C],
        '_' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1F],
        '-' => [0x00, 0x00, 0x00, 0x1F, 0x00, 0x00, 0x00],
        '/' => [0x01, 0x02, 0x02, 0x04, 0x08, 0x08, 0x10],
        ' ' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        _ => [0x0E, 0x11, 0x01, 0x02, 0x04, 0x00, 0x04], // '?'
    }
}

struct Emulator {
    cpu: Cpu,
    ppu_line_cycles: usize,
    bg_opaque: Vec<bool>,
    scanline_latches: [renderer::ScanlineRegs; HEIGHT as usize],
    scanline_latched: [bool; HEIGHT as usize],
    dump_next_frame: bool,
    dump_index: usize,
    debug_dump_settings: DebugDumpSettings,
}

impl Emulator {
    fn new(cpu: Cpu, debug_dump_settings: DebugDumpSettings) -> Self {
        Self {
            cpu,
            ppu_line_cycles: 0,
            bg_opaque: vec![false; (WIDTH * HEIGHT) as usize],
            scanline_latches: [renderer::ScanlineRegs::default(); HEIGHT as usize],
            scanline_latched: [false; HEIGHT as usize],
            dump_next_frame: false,
            dump_index: 0,
            debug_dump_settings,
        }
    }

    /// Runs the CPU for approximately one frame's worth of cycles.
    fn step_frame(&mut self) {
        self.step_cycles(CYCLES_PER_FRAME);
    }

    fn step_cycles(&mut self, cycle_budget: usize) {
        let mut cycles_this_step = 0;
        while cycles_this_step < cycle_budget {
            let cycles = self.cpu.step();
            cycles_this_step += cycles;
            self.tick_lcd(cycles);

            if self.cpu.tick_timers(cycles as u32) {
                self.cpu.raise_if(0x04);
            }

            if self.is_interrupt_pending() {
                let interrupt_cycles = self.interrupt();
                cycles_this_step += interrupt_cycles;
                self.tick_lcd(interrupt_cycles);

                if self.cpu.tick_timers(interrupt_cycles as u32) {
                    self.cpu.raise_if(0x04);
                }
            }
        }
    }

    fn tick_lcd(&mut self, cycles: usize) {
        let lcdc = self.cpu.read_byte(Addr(0xFF40));
        if (lcdc & 0x80) == 0 {
            self.ppu_line_cycles = 0;
            self.cpu.set_ly_raw(0);
            self.scanline_latched.fill(false);
            self.update_stat(0, false, false);
            return;
        }

        self.ppu_line_cycles += cycles;
        while self.ppu_line_cycles >= 456 {
            self.ppu_line_cycles -= 456;
            let ly = self.cpu.read_byte(Addr(0xFF44));
            let new_ly = if ly >= 153 { 0 } else { ly + 1 };
            self.cpu.set_ly_raw(new_ly);
            if new_ly == 0 {
                self.scanline_latched.fill(false);
            }
            if new_ly == 144 {
                self.cpu.raise_if(0x01);
            }
        }

        let ly = self.cpu.read_byte(Addr(0xFF44));
        let mode = if ly >= 144 {
            1
        } else if self.ppu_line_cycles < 80 {
            2
        } else if self.ppu_line_cycles < 252 {
            3
        } else {
            0
        };
        let lyc = self.cpu.read_byte(Addr(0xFF45));
        self.update_stat(mode, ly == lyc, true);
        self.maybe_latch_scanline(ly, mode);
    }

    fn update_stat(&mut self, mode: u8, coincidence: bool, allow_interrupt: bool) {
        let old_stat = self.cpu.read_byte(Addr(0xFF41));
        let old_mode = old_stat & 0x03;
        let old_coincidence = (old_stat & 0x04) != 0;
        let mut new_stat = (old_stat & 0x78) | (mode & 0x03);
        if coincidence {
            new_stat |= 0x04;
        }
        self.cpu.set_stat_raw(new_stat);

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
            self.cpu.raise_if(0x02);
        }
    }

    fn is_interrupt_pending(&self) -> bool {
        self.cpu.registers.ime && (self.cpu.get_ie() & self.cpu.get_if()) != 0
    }

    fn interrupt(&mut self) -> usize {
        self.cpu.halted = false;
        self.cpu.registers.ime = false;
        let if_contents = self.cpu.get_if();
        let ie_contents = self.cpu.get_ie();
        let pending = if_contents & ie_contents;
        for bit in 0..5u8 {
            if pending & (1 << bit) != 0 {
                self.cpu.clear_if(1 << bit);
                let vector = 0x0040u16 + (bit as u16) * 8;
                self.call(vector);
                self.cpu.total_cycles += INTERRUPT_SERVICE_CYCLES as u64;
                return INTERRUPT_SERVICE_CYCLES;
            }
        }
        0
    }

    fn call(&mut self, vector: u16) {
        self.cpu.write_word(
            Addr(self.cpu.registers.sp.wrapping_sub(2)),
            self.cpu.registers.pc,
        );
        self.cpu.registers.sp = self.cpu.registers.sp.wrapping_sub(2);
        self.cpu.registers.pc = vector;
    }

    /// Renders the current emulator state into the pixel buffer.
    fn draw(&mut self, screen: &mut [u8]) {
        let mut latches = self.scanline_latches;
        if self.scanline_latched.iter().any(|latched| !latched) {
            let ram = self.cpu.memory_slice();
            let fallback = renderer::ScanlineRegs {
                lcdc: ram[0xFF40],
                scy: ram[0xFF42],
                scx: ram[0xFF43],
                bgp: ram[0xFF47],
                wy: ram[0xFF4A],
                wx: ram[0xFF4B],
            };
            for (line, latched) in self.scanline_latched.iter().enumerate() {
                if !latched {
                    latches[line] = fallback;
                }
            }
        }

        renderer::render_frame_with_scanline_latches(
            self.cpu.memory_slice(),
            screen,
            &mut self.bg_opaque,
            &latches,
        );
    }

    fn maybe_latch_scanline(&mut self, ly: u8, mode: u8) {
        if mode != 3 || ly >= HEIGHT as u8 {
            return;
        }
        let line = ly as usize;
        if self.scanline_latched[line] {
            return;
        }

        let ram = self.cpu.memory_slice();
        self.scanline_latches[line] = renderer::ScanlineRegs {
            lcdc: ram[0xFF40],
            scy: ram[0xFF42],
            scx: ram[0xFF43],
            bgp: ram[0xFF47],
            wy: ram[0xFF4A],
            wx: ram[0xFF4B],
        };
        self.scanline_latched[line] = true;
    }

    fn request_dump(&mut self) {
        if !self.debug_dump_settings.enabled {
            debug!("Debug dump requested, but debug_dump.enabled is false");
            return;
        }
        self.dump_next_frame = true;
    }

    fn reload_debug_dump_settings(&mut self, settings: DebugDumpSettings) {
        self.debug_dump_settings = settings;
        if !self.debug_dump_settings.enabled {
            self.dump_next_frame = false;
        }
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
        let out_dir = self.debug_dump_settings.output_directory.clone();
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

        let ram = self.cpu.memory_slice();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interrupt_services_pending_request_with_20_cycles() {
        let mut cpu = Cpu::new();
        cpu.registers.pc = 0x1234;
        cpu.registers.sp = 0xFFFE;
        cpu.registers.ime = true;
        cpu.write_byte(Addr(0xFFFF), 0x04); // IE: timer
        cpu.raise_if(0x04); // IF: timer pending

        let mut emulator = Emulator::new(cpu, DebugDumpSettings::default());
        let cycles = emulator.interrupt();

        assert_eq!(cycles, INTERRUPT_SERVICE_CYCLES);
        assert_eq!(emulator.cpu.total_cycles, INTERRUPT_SERVICE_CYCLES as u64);
        assert!(!emulator.cpu.registers.ime);
        assert_eq!(emulator.cpu.registers.pc, 0x0050);
        assert_eq!(emulator.cpu.registers.sp, 0xFFFC);
        assert_eq!(emulator.cpu.read_word(Addr(0xFFFC)), 0x1234);
        assert_eq!(emulator.cpu.get_if() & 0x04, 0);
    }

    #[test]
    fn bounded_step_counts_interrupt_cycles_for_timer_and_ppu() {
        let mut cpu = Cpu::new();
        cpu.registers.ime = true;
        cpu.write_byte(Addr(0xFFFF), 0x01); // IE: vblank
        cpu.raise_if(0x01); // IF: vblank pending
        cpu.write_byte(Addr(0xFF07), 0x05); // TAC: enabled, 16-cycle timer

        let mut emulator = Emulator::new(cpu, DebugDumpSettings::default());
        emulator.step_cycles(4);

        assert_eq!(emulator.cpu.total_cycles, 24);
        assert_eq!(emulator.cpu.read_byte(Addr(0xFF05)), 1);
        assert_eq!(emulator.ppu_line_cycles, 24);
    }

    #[test]
    fn maybe_latch_scanline_captures_registers_once_per_line() {
        let mut cpu = Cpu::new();
        cpu.write_byte(Addr(0xFF40), 0xB1);
        cpu.write_byte(Addr(0xFF42), 0x22);
        cpu.write_byte(Addr(0xFF43), 0x11);
        cpu.write_byte(Addr(0xFF47), 0xE4);
        cpu.write_byte(Addr(0xFF4A), 0x05);
        cpu.write_byte(Addr(0xFF4B), 0x10);

        let mut emulator = Emulator::new(cpu, DebugDumpSettings::default());
        emulator.maybe_latch_scanline(12, 3);

        assert!(emulator.scanline_latched[12]);
        let first = emulator.scanline_latches[12];
        assert_eq!(first.lcdc, 0xB1);
        assert_eq!(first.scy, 0x22);
        assert_eq!(first.scx, 0x11);
        assert_eq!(first.bgp, 0xE4);
        assert_eq!(first.wy, 0x05);
        assert_eq!(first.wx, 0x10);

        emulator.cpu.write_byte(Addr(0xFF42), 0x99);
        emulator.cpu.write_byte(Addr(0xFF43), 0x88);
        emulator.maybe_latch_scanline(12, 3);
        let second = emulator.scanline_latches[12];

        assert_eq!(
            first.scy, second.scy,
            "line latch should be stable after first capture"
        );
        assert_eq!(
            first.scx, second.scx,
            "line latch should be stable after first capture"
        );
    }

    #[test]
    fn tick_lcd_clears_scanline_latches_on_frame_wrap() {
        let cpu = Cpu::new();
        let mut emulator = Emulator::new(cpu, DebugDumpSettings::default());

        emulator.scanline_latched.fill(true);
        emulator.cpu.set_ly_raw(153);
        emulator.ppu_line_cycles = 0;
        emulator.tick_lcd(456);

        assert!(
            emulator.scanline_latched.iter().all(|latched| !latched),
            "all scanline latches should reset when LY wraps to 0"
        );
    }

    #[test]
    fn clip_overlay_text_uppercases_and_truncates() {
        let clipped = clip_overlay_text("jelly_tiles.wgsl", 10);
        assert_eq!(clipped, "JELLY_T...");
    }

    #[test]
    fn draw_overlay_text_writes_visible_pixels() {
        let mut frame = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
        draw_overlay_text(&mut frame, "jelly_tiles.wgsl");
        assert!(
            frame
                .chunks_exact(4)
                .any(|px| px[0] != 0 || px[1] != 0 || px[2] != 0),
            "overlay text should write at least one non-black pixel"
        );
    }
}
