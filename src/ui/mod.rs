use std::error::Error as StdError;
use std::str::FromStr;

use winit::window::Window;

pub mod pixels_backend;
pub mod wgpu_shader_backend;

pub type UiError = Box<dyn StdError + 'static>;
pub type UiResult<T> = Result<T, UiError>;

pub trait GraphicsBackend {
    fn frame_mut(&mut self) -> &mut [u8];
    fn present(&mut self) -> UiResult<()>;
    fn resize_surface(&mut self, width: u32, height: u32) -> UiResult<()>;
    fn reload_options(&mut self, _options: GraphicsOptions) -> UiResult<()> {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphicsBackendKind {
    Pixels,
    WgpuShader,
}

impl GraphicsBackendKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pixels => "pixels",
            Self::WgpuShader => "wgpu_shader",
        }
    }
}

impl FromStr for GraphicsBackendKind {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let normalized = value.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "pixels" => Ok(Self::Pixels),
            "wgpu_shader" | "wgpu-shader" | "wgpu" | "wgsl" => Ok(Self::WgpuShader),
            _ => Err(format!(
                "unsupported backend '{value}'. Supported values: pixels, wgpu_shader"
            )),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ShaderOptions {
    pub scanline_strength: f32,
    pub curvature: f32,
}

impl ShaderOptions {
    pub fn clamped(self) -> Self {
        Self {
            scanline_strength: self.scanline_strength.clamp(0.0, 1.0),
            curvature: self.curvature.clamp(0.0, 0.35),
        }
    }
}

impl Default for ShaderOptions {
    fn default() -> Self {
        Self {
            scanline_strength: 0.18,
            curvature: 0.08,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct GraphicsOptions {
    pub shader: ShaderOptions,
}

pub fn create_backend<'win>(
    kind: GraphicsBackendKind,
    width: u32,
    height: u32,
    window: &'win Window,
    options: GraphicsOptions,
) -> UiResult<Box<dyn GraphicsBackend + 'win>> {
    match kind {
        GraphicsBackendKind::Pixels => Ok(Box::new(pixels_backend::PixelsBackend::new(
            width, height, window,
        )?)),
        GraphicsBackendKind::WgpuShader => Ok(Box::new(
            wgpu_shader_backend::WgpuShaderBackend::new(width, height, window, options.shader)?,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_backend_aliases() {
        assert_eq!(
            "pixels".parse::<GraphicsBackendKind>().expect("pixels should parse"),
            GraphicsBackendKind::Pixels
        );
        assert_eq!(
            "wgpu_shader"
                .parse::<GraphicsBackendKind>()
                .expect("wgpu_shader should parse"),
            GraphicsBackendKind::WgpuShader
        );
        assert_eq!(
            "wgpu"
                .parse::<GraphicsBackendKind>()
                .expect("wgpu alias should parse"),
            GraphicsBackendKind::WgpuShader
        );
        assert_eq!(
            "wgsl"
                .parse::<GraphicsBackendKind>()
                .expect("wgsl alias should parse"),
            GraphicsBackendKind::WgpuShader
        );
    }

    #[test]
    fn rejects_unknown_backend() {
        let err = "not_a_backend"
            .parse::<GraphicsBackendKind>()
            .expect_err("unknown backend should fail");
        assert!(err.contains("pixels, wgpu_shader"));
    }

    #[test]
    fn clamps_shader_options_to_supported_ranges() {
        let clamped = ShaderOptions {
            scanline_strength: -5.0,
            curvature: 999.0,
        }
        .clamped();

        assert!((clamped.scanline_strength - 0.0).abs() < f32::EPSILON);
        assert!((clamped.curvature - 0.35).abs() < f32::EPSILON);
    }
}
