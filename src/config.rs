use crate::ui::{GraphicsBackendKind, GraphicsOptions, ShaderOptions};
use serde::Deserialize;
use std::fs;
use std::io;
use std::path::Path;

const CONFIG_FILE: &str = "config.json";

#[derive(Debug, Deserialize, Default)]
struct AppConfig {
    #[serde(default)]
    graphics_backend: Option<String>,
    #[serde(default)]
    shader: ShaderConfig,
}

#[derive(Debug, Deserialize, Default)]
struct ShaderConfig {
    scanline_strength: Option<f32>,
    curvature: Option<f32>,
}

pub fn load_graphics_settings(
) -> Result<(GraphicsBackendKind, GraphicsOptions), Box<dyn std::error::Error>> {
    let cfg = load_config(CONFIG_FILE)?;

    let backend = match cfg.graphics_backend {
        Some(value) => value.parse::<GraphicsBackendKind>().map_err(|msg| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Invalid graphics_backend in {CONFIG_FILE}: {msg}"),
            )
        })?,
        None => GraphicsBackendKind::Pixels,
    };

    let defaults = ShaderOptions::default();
    let shader = ShaderOptions {
        scanline_strength: cfg
            .shader
            .scanline_strength
            .unwrap_or(defaults.scanline_strength),
        curvature: cfg.shader.curvature.unwrap_or(defaults.curvature),
    }
    .clamped();

    Ok((backend, GraphicsOptions { shader }))
}

fn load_config(path: &str) -> Result<AppConfig, Box<dyn std::error::Error>> {
    if !Path::new(path).exists() {
        return Ok(AppConfig::default());
    }

    let contents = fs::read_to_string(path)?;
    let config = serde_json::from_str::<AppConfig>(&contents).map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to parse {path}: {err}"),
        )
    })?;
    Ok(config)
}
