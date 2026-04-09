use std::error::Error as StdError;
#[cfg(any(not(feature = "frontend-pixels"), not(feature = "frontend-wgpu")))]
use std::io;
use std::path::PathBuf;
use std::str::FromStr;

use winit::window::Window;

#[cfg(feature = "frontend-pixels")]
pub mod pixels_backend;
pub mod terminal_backend;
#[cfg(feature = "frontend-wgpu")]
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
    fn cycle_shader_next(&mut self) -> UiResult<Option<String>> {
        Ok(None)
    }
    fn cycle_shader_prev(&mut self) -> UiResult<Option<String>> {
        Ok(None)
    }
    fn reload_shader_library(
        &mut self,
        _preferred_active_file: Option<&str>,
    ) -> UiResult<Option<String>> {
        Ok(None)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphicsBackendKind {
    Pixels,
    Terminal,
    WgpuShader,
}

impl GraphicsBackendKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pixels => "pixels",
            Self::Terminal => "terminal",
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
            "terminal" | "tty" | "ansi" => Ok(Self::Terminal),
            "wgpu_shader" | "wgpu-shader" | "wgpu" | "wgsl" => Ok(Self::WgpuShader),
            _ => Err(format!(
                "unsupported backend '{value}'. Supported values: pixels, terminal, wgpu_shader"
            )),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ShaderOptions {
    pub scanline_strength: f32,
    pub curvature: f32,
    pub color_intensity: f32,
    pub mode: ShaderColorMode,
    pub active_file: Option<String>,
}

impl ShaderOptions {
    pub fn clamped(&self) -> Self {
        Self {
            scanline_strength: self.scanline_strength.clamp(0.0, 1.0),
            curvature: self.curvature.clamp(0.0, 0.35),
            color_intensity: self.color_intensity.clamp(0.0, 1.5),
            mode: self.mode,
            active_file: self.active_file.clone(),
        }
    }
}

impl Default for ShaderOptions {
    fn default() -> Self {
        Self {
            scanline_strength: 0.18,
            curvature: 0.08,
            color_intensity: 0.65,
            mode: ShaderColorMode::Classic,
            active_file: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShaderColorMode {
    #[default]
    Classic,
    Prism,
    Aurora,
    PaletteMutation,
}

impl FromStr for ShaderColorMode {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let normalized = value.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "classic" | "crt" | "default" => Ok(Self::Classic),
            "prism" | "chromashift" | "chroma_shift" | "chroma-shift" => Ok(Self::Prism),
            "aurora" | "acid" | "vapor" => Ok(Self::Aurora),
            "palette_mutation" | "palette-mutation" | "mutation" | "mutant" => {
                Ok(Self::PaletteMutation)
            }
            _ => Err(format!(
                "unsupported shader mode '{value}'. Supported values: classic, prism, aurora, palette_mutation"
            )),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct GraphicsOptions {
    pub shader: ShaderOptions,
    pub shader_directory: PathBuf,
}

pub fn create_backend<'win>(
    kind: GraphicsBackendKind,
    width: u32,
    height: u32,
    window: &'win Window,
    options: GraphicsOptions,
) -> UiResult<Box<dyn GraphicsBackend + 'win>> {
    match kind {
        GraphicsBackendKind::Pixels => {
            #[cfg(feature = "frontend-pixels")]
            {
                Ok(Box::new(pixels_backend::PixelsBackend::new(
                    width, height, window,
                )?))
            }
            #[cfg(not(feature = "frontend-pixels"))]
            {
                Err(feature_disabled_error("pixels backend", "frontend-pixels"))
            }
        }
        GraphicsBackendKind::Terminal => Ok(Box::new(terminal_backend::TerminalBackend::new(
            width, height,
        )?)),
        GraphicsBackendKind::WgpuShader => {
            #[cfg(feature = "frontend-wgpu")]
            {
                Ok(Box::new(wgpu_shader_backend::WgpuShaderBackend::new(
                    width, height, window, options,
                )?))
            }
            #[cfg(not(feature = "frontend-wgpu"))]
            {
                let _ = (window, options);
                Err(feature_disabled_error(
                    "wgpu_shader backend",
                    "frontend-wgpu",
                ))
            }
        }
    }
}

#[cfg(any(not(feature = "frontend-pixels"), not(feature = "frontend-wgpu")))]
fn feature_disabled_error(backend_name: &str, feature: &str) -> UiError {
    Box::new(io::Error::new(
        io::ErrorKind::Unsupported,
        format!("{backend_name} is disabled in this build; rebuild with `--features {feature}`"),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_backend_aliases() {
        assert_eq!(
            "pixels"
                .parse::<GraphicsBackendKind>()
                .expect("pixels should parse"),
            GraphicsBackendKind::Pixels
        );
        assert_eq!(
            "terminal"
                .parse::<GraphicsBackendKind>()
                .expect("terminal should parse"),
            GraphicsBackendKind::Terminal
        );
        assert_eq!(
            "tty"
                .parse::<GraphicsBackendKind>()
                .expect("tty alias should parse"),
            GraphicsBackendKind::Terminal
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
        assert!(err.contains("pixels, terminal, wgpu_shader"));
    }

    #[test]
    fn clamps_shader_options_to_supported_ranges() {
        let clamped = ShaderOptions {
            scanline_strength: -5.0,
            curvature: 999.0,
            color_intensity: 9.0,
            mode: ShaderColorMode::Prism,
            active_file: Some("foo.wgsl".to_string()),
        }
        .clamped();

        assert!((clamped.scanline_strength - 0.0).abs() < f32::EPSILON);
        assert!((clamped.curvature - 0.35).abs() < f32::EPSILON);
        assert!((clamped.color_intensity - 1.5).abs() < f32::EPSILON);
        assert_eq!(clamped.mode, ShaderColorMode::Prism);
        assert_eq!(clamped.active_file.as_deref(), Some("foo.wgsl"));
    }

    #[test]
    fn parses_shader_color_mode_aliases() {
        assert_eq!(
            "classic"
                .parse::<ShaderColorMode>()
                .expect("classic should parse"),
            ShaderColorMode::Classic
        );
        assert_eq!(
            "chroma-shift"
                .parse::<ShaderColorMode>()
                .expect("chroma-shift should parse"),
            ShaderColorMode::Prism
        );
        assert_eq!(
            "acid"
                .parse::<ShaderColorMode>()
                .expect("acid alias should parse"),
            ShaderColorMode::Aurora
        );
        assert_eq!(
            "palette-mutation"
                .parse::<ShaderColorMode>()
                .expect("palette-mutation should parse"),
            ShaderColorMode::PaletteMutation
        );
    }

    #[test]
    fn rejects_unknown_shader_color_mode() {
        let err = "not-a-mode"
            .parse::<ShaderColorMode>()
            .expect_err("unknown shader mode should fail");
        assert!(err.contains("classic, prism, aurora, palette_mutation"));
    }
}
