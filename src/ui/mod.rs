use std::error::Error as StdError;

use winit::window::Window;

pub mod pixels_backend;

pub type UiError = Box<dyn StdError + 'static>;
pub type UiResult<T> = Result<T, UiError>;

pub trait GraphicsBackend {
    fn frame_mut(&mut self) -> &mut [u8];
    fn present(&mut self) -> UiResult<()>;
    fn resize_surface(&mut self, width: u32, height: u32) -> UiResult<()>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphicsBackendKind {
    Pixels,
}

pub fn create_backend<'win>(
    kind: GraphicsBackendKind,
    width: u32,
    height: u32,
    window: &'win Window,
) -> UiResult<Box<dyn GraphicsBackend + 'win>> {
    match kind {
        GraphicsBackendKind::Pixels => Ok(Box::new(pixels_backend::PixelsBackend::new(
            width, height, window,
        )?)),
    }
}
