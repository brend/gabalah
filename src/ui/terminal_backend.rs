use super::{GraphicsBackend, UiResult};
use std::fmt::Write as _;
use std::io::{stdout, Stdout, Write};
use std::time::{Duration, Instant};

const TERMINAL_COLS: usize = 80;
const TERMINAL_ROWS: usize = 24;
const TARGET_FPS: u64 = 15;

pub struct TerminalBackend {
    width: u32,
    height: u32,
    frame: Vec<u8>,
    stdout: Stdout,
    frame_interval: Duration,
    last_present: Option<Instant>,
}

impl TerminalBackend {
    pub fn new(width: u32, height: u32) -> UiResult<Self> {
        let mut stdout = stdout();
        // Enter alternate screen and hide cursor so terminal output does not
        // spam the main shell buffer.
        stdout.write_all(b"\x1b[?1049h\x1b[?25l\x1b[2J\x1b[H")?;
        stdout.flush()?;

        Ok(Self {
            width,
            height,
            frame: vec![0; (width * height * 4) as usize],
            stdout,
            frame_interval: Duration::from_millis(1000 / TARGET_FPS.max(1)),
            last_present: None,
        })
    }

    fn sample_rgb(&self, x: usize, y: usize) -> (u8, u8, u8) {
        let idx = (y * self.width as usize + x) * 4;
        (self.frame[idx], self.frame[idx + 1], self.frame[idx + 2])
    }
}

impl GraphicsBackend for TerminalBackend {
    fn frame_mut(&mut self) -> &mut [u8] {
        &mut self.frame
    }

    fn present(&mut self) -> UiResult<()> {
        let now = Instant::now();
        if let Some(last_present) = self.last_present {
            if now.duration_since(last_present) < self.frame_interval {
                return Ok(());
            }
        }
        self.last_present = Some(now);

        let source_width = self.width as usize;
        let source_height = self.height as usize;

        let mut output = String::with_capacity(TERMINAL_COLS * TERMINAL_ROWS * 28);
        output.push_str("\x1b[H");

        for row in 0..TERMINAL_ROWS {
            let src_y = row * source_height / TERMINAL_ROWS;
            for col in 0..TERMINAL_COLS {
                let src_x = col * source_width / TERMINAL_COLS;
                let (r, g, b) = self.sample_rgb(src_x, src_y);
                let _ = write!(output, "\x1b[48;2;{r};{g};{b}m ");
            }
            output.push_str("\x1b[0m");
            if row + 1 < TERMINAL_ROWS {
                output.push('\n');
            }
        }
        output.push_str("\x1b[0m");

        self.stdout.write_all(output.as_bytes())?;
        self.stdout.flush()?;
        Ok(())
    }

    fn resize_surface(&mut self, _width: u32, _height: u32) -> UiResult<()> {
        // The terminal backend scales into a fixed 80x24 target grid.
        Ok(())
    }
}

impl Drop for TerminalBackend {
    fn drop(&mut self) {
        let _ = self.stdout.write_all(b"\x1b[0m\x1b[?25h\x1b[?1049l");
        let _ = self.stdout.flush();
    }
}
