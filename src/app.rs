#![deny(clippy::all)]
#![forbid(unsafe_code)]

use error_iter::ErrorIter as _;
use std::time::{Duration, Instant};
use crate::cpu::Cpu;
use crate::memory::Addr;
use super::renderer;
use log::{debug, error};
use pixels::{Error, PixelsBuilder, SurfaceTexture};
#[cfg(target_os = "windows")]
use pixels::wgpu::Backends;
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

pub fn run_loop(cpu: Cpu) -> Result<(), Error> {
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

    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        #[allow(unused_mut)]
        let mut builder = PixelsBuilder::new(WIDTH, HEIGHT, surface_texture);
        #[cfg(target_os = "windows")]
        {
            // Avoid the DX12 backend on Windows because some drivers trip over
            // swapchain render-target state transitions during presentation.
            builder = builder.wgpu_backend(Backends::VULKAN | Backends::GL);
        }

        let pixels = builder.build()?;
        let adapter_info = pixels.adapter().get_info();
        debug!(
            "Initialized pixels with backend={} adapter={}",
            adapter_info.backend.to_str(),
            adapter_info.name
        );
        pixels
    };

    let mut emulator = Emulator::new(cpu);
    let mut last_frame = Instant::now();

    let res = event_loop.run(|event, elwt| {
        if let Event::WindowEvent {
            event: WindowEvent::RedrawRequested,
            ..
        } = event
        {
            emulator.draw(pixels.frame_mut());
            if let Err(err) = pixels.render() {
                log_error("pixels.render", err);
                elwt.exit();
                return;
            }
        }

        if input.update(&event) {
            if input.key_pressed(KeyCode::Escape) || input.close_requested() {
                elwt.exit();
                return;
            }

            // Resize the window
            if let Some(size) = input.window_resized() {
                if let Err(err) = pixels.resize_surface(size.width, size.height) {
                    log_error("pixels.resize_surface", err);
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
    res.map_err(|e| Error::UserDefined(Box::new(e)))
}

fn log_error<E: std::error::Error + 'static>(method_name: &str, err: E) {
    error!("{method_name}() failed: {err}");
    for source in err.sources().skip(1) {
        error!("  Caused by: {source}");
    }
}

struct Emulator {
    cpu: Cpu,
}

impl Emulator {
    fn new(cpu: Cpu) -> Self {
        Self { cpu }
    }

    /// Runs the CPU for approximately one frame's worth of cycles.
    fn step_frame(&mut self) {
        let mut cycles_this_frame = 0;
        while cycles_this_frame < CYCLES_PER_FRAME {
            let cycles = self.cpu.step();
            cycles_this_frame += cycles;
            let ly = (cycles_this_frame / 456).min(153) as u8;
            self.cpu.memory.write_byte(Addr(0xFF44), ly);
        }
    }

    /// Renders the current emulator state into the pixel buffer.
    fn draw(&self, screen: &mut [u8]) {
        renderer::render_frame(self.cpu.memory.as_slice(), screen);
    }
}
