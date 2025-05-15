mod app;
mod cpu;
mod err;
mod memory;
mod renderer;

use cpu::Cpu;
use error_iter::ErrorIter;
use log::error;
use pixels::Error;
use pixels::Pixels;
use pixels::SurfaceTexture;
use renderer::{read_pixels, HEIGHT, WIDTH};
use std::env;
use std::fs;
use winit::dpi::LogicalSize;
use winit::event::Event;
use winit::event::WindowEvent;
use winit::event_loop::EventLoop;
use winit::keyboard::KeyCode;
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // read the rom from file
    let rom = read_rom()?;
    // create a new CPU and load the rom
    let mut cpu = Cpu::new();
    cpu.load_rom(rom);
    // let mut i = 0;

    // loop {
    //     i += 1;
    //     println!("Step: {}", i);
    //     cpu.step();
    // }
    Ok(app::run_loop(cpu)?)
}

fn read_rom() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <rom file>", args[0]);
        std::process::exit(1);
    }
    let filename = &args[1];
    let rom = fs::read(filename)?;
    Ok(rom)
}

pub fn moin() -> Result<(), Error> {
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();
    let mut input = WinitInputHelper::new();

    let window = {
        let size = LogicalSize::new(WIDTH as f64, HEIGHT as f64);
        let scaled_size = LogicalSize::new(WIDTH as f64 * 3.0, HEIGHT as f64 * 3.0);
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
        Pixels::new(WIDTH, HEIGHT, surface_texture)?
    };

    let ram: Vec<u8> = unimplemented!();
    let mut pixpixs = read_pixels(&ram);

    let res = event_loop.run(|event, elwt| {
        // The one and only event that winit_input_helper doesn't have for us...
        if let Event::WindowEvent {
            event: WindowEvent::RedrawRequested,
            ..
        } = event
        {
            // Draw the pixels
            for i in 0..pixpixs.len() / 4 {
                let color = pixpixs[i];
                pixpixs[i * 4] = color;
                pixpixs[i * 4 + 1] = color;
                pixpixs[i * 4 + 2] = color;
                pixpixs[i * 4 + 3] = 255;
            }

            if let Err(err) = pixels.render() {
                log_error("pixels.render", err);
                elwt.exit();
                return;
            }
        }

        // For everything else, for let winit_input_helper collect events to build its state.
        // It returns `true` when it is time to update our game state and request a redraw.
        if input.update(&event) {
            // Close events
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
            // update here!!
            window.request_redraw();
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
