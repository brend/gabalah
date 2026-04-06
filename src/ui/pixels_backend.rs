use super::{GraphicsBackend, UiResult};
#[cfg(target_os = "windows")]
use pixels::wgpu::Backends;
use pixels::{Pixels, PixelsBuilder, SurfaceTexture};
use winit::window::Window;

pub struct PixelsBackend<'win> {
    pixels: Pixels<'win>,
}

impl<'win> PixelsBackend<'win> {
    pub fn new(width: u32, height: u32, window: &'win Window) -> UiResult<Self> {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, window);
        #[allow(unused_mut)]
        let mut builder = PixelsBuilder::new(width, height, surface_texture);
        #[cfg(target_os = "windows")]
        {
            // Avoid the DX12 backend on Windows because some drivers trip over
            // swapchain render-target state transitions during presentation.
            builder = builder.wgpu_backend(Backends::VULKAN | Backends::GL);
        }

        let pixels = builder.build()?;
        let adapter_info = pixels.adapter().get_info();
        log::debug!(
            "Initialized pixels with backend={} adapter={}",
            adapter_info.backend.to_str(),
            adapter_info.name
        );

        Ok(Self { pixels })
    }
}

impl GraphicsBackend for PixelsBackend<'_> {
    fn frame_mut(&mut self) -> &mut [u8] {
        self.pixels.frame_mut()
    }

    fn present(&mut self) -> UiResult<()> {
        self.pixels.render()?;
        Ok(())
    }

    fn resize_surface(&mut self, width: u32, height: u32) -> UiResult<()> {
        self.pixels.resize_surface(width, height)?;
        Ok(())
    }
}
