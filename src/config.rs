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
    load_graphics_settings_from_path(Path::new(CONFIG_FILE))
}

fn load_graphics_settings_from_path(
    path: &Path,
) -> Result<(GraphicsBackendKind, GraphicsOptions), Box<dyn std::error::Error>> {
    let cfg = load_config(path)?;
    let config_name = path.display().to_string();

    let backend = match cfg.graphics_backend {
        Some(value) => value.parse::<GraphicsBackendKind>().map_err(|msg| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Invalid graphics_backend in {config_name}: {msg}"),
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

fn load_config(path: &Path) -> Result<AppConfig, Box<dyn std::error::Error>> {
    if !path.exists() {
        return Ok(AppConfig::default());
    }

    let contents = fs::read_to_string(path)?;
    let config = serde_json::from_str::<AppConfig>(&contents).map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to parse {}: {err}", path.display()),
        )
    })?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::process;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn missing_config_uses_defaults() {
        let path = unique_temp_path("missing");
        let (backend, options) =
            load_graphics_settings_from_path(&path).expect("missing config should be valid");
        assert_eq!(backend, GraphicsBackendKind::Pixels);
        let defaults = ShaderOptions::default();
        assert_eq!(options.shader.scanline_strength, defaults.scanline_strength);
        assert_eq!(options.shader.curvature, defaults.curvature);
    }

    #[test]
    fn parses_wgpu_shader_backend_and_shader_values() {
        let path = write_temp_config(
            r#"{
                "graphics_backend": "wgpu_shader",
                "shader": {
                    "scanline_strength": 0.27,
                    "curvature": 0.11
                }
            }"#,
        );

        let (backend, options) = load_graphics_settings_from_path(&path)
            .expect("valid config should parse successfully");
        assert_eq!(backend, GraphicsBackendKind::WgpuShader);
        assert!((options.shader.scanline_strength - 0.27).abs() < f32::EPSILON);
        assert!((options.shader.curvature - 0.11).abs() < f32::EPSILON);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn rejects_invalid_backend_value() {
        let path = write_temp_config(
            r#"{
                "graphics_backend": "broken_backend"
            }"#,
        );

        let err = load_graphics_settings_from_path(&path)
            .expect_err("invalid backend should return an error");
        let msg = err.to_string();
        assert!(msg.contains("Invalid graphics_backend"));
        assert!(msg.contains("pixels, wgpu_shader"));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn clamps_shader_values_from_config() {
        let path = write_temp_config(
            r#"{
                "graphics_backend": "wgpu_shader",
                "shader": {
                    "scanline_strength": 9.0,
                    "curvature": -5.0
                }
            }"#,
        );

        let (_, options) =
            load_graphics_settings_from_path(&path).expect("config with out-of-range values should parse");
        assert!((options.shader.scanline_strength - 1.0).abs() < f32::EPSILON);
        assert!((options.shader.curvature - 0.0).abs() < f32::EPSILON);

        let _ = fs::remove_file(path);
    }

    fn write_temp_config(contents: &str) -> PathBuf {
        let path = unique_temp_path("config");
        fs::write(&path, contents).expect("temp config write should succeed");
        path
    }

    fn unique_temp_path(label: &str) -> PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be after unix epoch")
            .as_nanos();
        let mut path = std::env::temp_dir();
        path.push(format!(
            "gabalah_{label}_{}_{}.json",
            process::id(),
            timestamp
        ));
        path
    }
}
