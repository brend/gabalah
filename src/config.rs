use crate::ui::{GraphicsBackendKind, GraphicsOptions, ShaderColorMode, ShaderOptions};
use serde::Deserialize;
use serde_json::{json, Value};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use winit::keyboard::KeyCode;

const CONFIG_FILE: &str = "config.json";
const DEFAULT_WINDOW_SCALE: f64 = 3.0;
const DEFAULT_SHADER_DIRECTORY: &str = "shaders";
const DEFAULT_DEBUG_DUMP_DIRECTORY: &str = "debug_dumps";

#[derive(Debug, Deserialize, Default)]
struct AppConfig {
    #[serde(default)]
    graphics_backend: Option<String>,
    #[serde(default)]
    window: WindowConfig,
    #[serde(default)]
    controls: ControlsConfig,
    #[serde(default)]
    debug_dump: DebugDumpConfig,
    #[serde(default)]
    shader: ShaderConfig,
}

#[derive(Debug, Deserialize, Default)]
struct WindowConfig {
    scale: Option<f64>,
}

#[derive(Debug, Deserialize, Default)]
struct ControlsConfig {
    #[serde(default)]
    joypad: JoypadConfig,
    #[serde(default)]
    hotkeys: HotkeyConfig,
}

#[derive(Debug, Deserialize, Default)]
struct JoypadConfig {
    up: Option<String>,
    down: Option<String>,
    left: Option<String>,
    right: Option<String>,
    a: Option<String>,
    b: Option<String>,
    select: Option<String>,
    start: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct HotkeyConfig {
    reload_graphics_config: Option<String>,
    previous_shader: Option<String>,
    next_shader: Option<String>,
    debug_frame_dump: Option<String>,
    exit: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JoypadBindings {
    pub up: KeyCode,
    pub down: KeyCode,
    pub left: KeyCode,
    pub right: KeyCode,
    pub a: KeyCode,
    pub b: KeyCode,
    pub select: KeyCode,
    pub start: KeyCode,
}

impl Default for JoypadBindings {
    fn default() -> Self {
        Self {
            up: KeyCode::ArrowUp,
            down: KeyCode::ArrowDown,
            left: KeyCode::ArrowLeft,
            right: KeyCode::ArrowRight,
            a: KeyCode::KeyZ,
            b: KeyCode::KeyX,
            select: KeyCode::ShiftRight,
            start: KeyCode::Enter,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HotkeyBindings {
    pub reload_graphics_config: KeyCode,
    pub previous_shader: KeyCode,
    pub next_shader: KeyCode,
    pub debug_frame_dump: KeyCode,
    pub exit: KeyCode,
}

impl Default for HotkeyBindings {
    fn default() -> Self {
        Self {
            reload_graphics_config: KeyCode::KeyR,
            previous_shader: KeyCode::KeyQ,
            next_shader: KeyCode::KeyE,
            debug_frame_dump: KeyCode::F9,
            exit: KeyCode::Escape,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Controls {
    pub joypad: JoypadBindings,
    pub hotkeys: HotkeyBindings,
}

#[derive(Debug, Deserialize, Default)]
struct ShaderConfig {
    scanline_strength: Option<f32>,
    curvature: Option<f32>,
    color_intensity: Option<f32>,
    mode: Option<String>,
    active_file: Option<String>,
    directory: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct DebugDumpConfig {
    enabled: Option<bool>,
    output_directory: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugDumpSettings {
    pub enabled: bool,
    pub output_directory: PathBuf,
}

impl Default for DebugDumpSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            output_directory: PathBuf::from(DEFAULT_DEBUG_DUMP_DIRECTORY),
        }
    }
}

pub fn load_graphics_settings(
) -> Result<(GraphicsBackendKind, GraphicsOptions), Box<dyn std::error::Error>> {
    load_graphics_settings_from_path(Path::new(CONFIG_FILE))
}

pub fn load_window_scale() -> Result<f64, Box<dyn std::error::Error>> {
    load_window_scale_from_path(Path::new(CONFIG_FILE))
}

pub fn load_controls() -> Result<Controls, Box<dyn std::error::Error>> {
    load_controls_from_path(Path::new(CONFIG_FILE))
}

pub fn load_debug_dump_settings() -> Result<DebugDumpSettings, Box<dyn std::error::Error>> {
    load_debug_dump_settings_from_path(Path::new(CONFIG_FILE))
}

pub fn save_active_shader_file(
    active_file: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    save_active_shader_file_to_path(Path::new(CONFIG_FILE), active_file)
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
    let mode = match cfg.shader.mode.as_deref() {
        Some(value) => value.parse::<ShaderColorMode>().map_err(|msg| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Invalid shader.mode in {config_name}: {msg}"),
            )
        })?,
        None => defaults.mode,
    };
    let shader = ShaderOptions {
        scanline_strength: cfg
            .shader
            .scanline_strength
            .unwrap_or(defaults.scanline_strength),
        curvature: cfg.shader.curvature.unwrap_or(defaults.curvature),
        color_intensity: cfg
            .shader
            .color_intensity
            .unwrap_or(defaults.color_intensity),
        mode,
        active_file: cfg.shader.active_file,
    }
    .clamped();
    let shader_directory = parse_non_empty_path(
        cfg.shader.directory.as_deref(),
        PathBuf::from(DEFAULT_SHADER_DIRECTORY),
        "shader.directory",
        &config_name,
    )?;

    Ok((
        backend,
        GraphicsOptions {
            shader,
            shader_directory,
        },
    ))
}

fn load_window_scale_from_path(path: &Path) -> Result<f64, Box<dyn std::error::Error>> {
    let cfg = load_config(path)?;
    let config_name = path.display().to_string();

    match cfg.window.scale {
        Some(scale) if scale.is_finite() && scale > 0.0 => Ok(scale),
        Some(scale) => Err(Box::new(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "Invalid window.scale in {config_name}: expected a finite number greater than 0, got {scale}"
            ),
        ))),
        None => Ok(DEFAULT_WINDOW_SCALE),
    }
}

fn load_controls_from_path(path: &Path) -> Result<Controls, Box<dyn std::error::Error>> {
    let cfg = load_config(path)?;
    let config_name = path.display().to_string();
    let joypad_defaults = JoypadBindings::default();
    let hotkey_defaults = HotkeyBindings::default();

    Ok(Controls {
        joypad: JoypadBindings {
            up: parse_key_binding(
                cfg.controls.joypad.up.as_deref(),
                joypad_defaults.up,
                "controls.joypad.up",
                &config_name,
            )?,
            down: parse_key_binding(
                cfg.controls.joypad.down.as_deref(),
                joypad_defaults.down,
                "controls.joypad.down",
                &config_name,
            )?,
            left: parse_key_binding(
                cfg.controls.joypad.left.as_deref(),
                joypad_defaults.left,
                "controls.joypad.left",
                &config_name,
            )?,
            right: parse_key_binding(
                cfg.controls.joypad.right.as_deref(),
                joypad_defaults.right,
                "controls.joypad.right",
                &config_name,
            )?,
            a: parse_key_binding(
                cfg.controls.joypad.a.as_deref(),
                joypad_defaults.a,
                "controls.joypad.a",
                &config_name,
            )?,
            b: parse_key_binding(
                cfg.controls.joypad.b.as_deref(),
                joypad_defaults.b,
                "controls.joypad.b",
                &config_name,
            )?,
            select: parse_key_binding(
                cfg.controls.joypad.select.as_deref(),
                joypad_defaults.select,
                "controls.joypad.select",
                &config_name,
            )?,
            start: parse_key_binding(
                cfg.controls.joypad.start.as_deref(),
                joypad_defaults.start,
                "controls.joypad.start",
                &config_name,
            )?,
        },
        hotkeys: HotkeyBindings {
            reload_graphics_config: parse_key_binding(
                cfg.controls.hotkeys.reload_graphics_config.as_deref(),
                hotkey_defaults.reload_graphics_config,
                "controls.hotkeys.reload_graphics_config",
                &config_name,
            )?,
            previous_shader: parse_key_binding(
                cfg.controls.hotkeys.previous_shader.as_deref(),
                hotkey_defaults.previous_shader,
                "controls.hotkeys.previous_shader",
                &config_name,
            )?,
            next_shader: parse_key_binding(
                cfg.controls.hotkeys.next_shader.as_deref(),
                hotkey_defaults.next_shader,
                "controls.hotkeys.next_shader",
                &config_name,
            )?,
            debug_frame_dump: parse_key_binding(
                cfg.controls.hotkeys.debug_frame_dump.as_deref(),
                hotkey_defaults.debug_frame_dump,
                "controls.hotkeys.debug_frame_dump",
                &config_name,
            )?,
            exit: parse_key_binding(
                cfg.controls.hotkeys.exit.as_deref(),
                hotkey_defaults.exit,
                "controls.hotkeys.exit",
                &config_name,
            )?,
        },
    })
}

fn load_debug_dump_settings_from_path(
    path: &Path,
) -> Result<DebugDumpSettings, Box<dyn std::error::Error>> {
    let cfg = load_config(path)?;
    let config_name = path.display().to_string();
    let defaults = DebugDumpSettings::default();

    Ok(DebugDumpSettings {
        enabled: cfg.debug_dump.enabled.unwrap_or(defaults.enabled),
        output_directory: parse_non_empty_path(
            cfg.debug_dump.output_directory.as_deref(),
            defaults.output_directory,
            "debug_dump.output_directory",
            &config_name,
        )?,
    })
}

fn parse_non_empty_path(
    value: Option<&str>,
    default: PathBuf,
    field_name: &str,
    config_name: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    match value {
        Some(raw) if raw.trim().is_empty() => Err(Box::new(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Invalid {field_name} in {config_name}: path must not be empty"),
        ))),
        Some(raw) => Ok(PathBuf::from(raw)),
        None => Ok(default),
    }
}

fn parse_key_binding(
    value: Option<&str>,
    default: KeyCode,
    field_name: &str,
    config_name: &str,
) -> Result<KeyCode, Box<dyn std::error::Error>> {
    match value {
        Some(key_name) => parse_key_code(key_name).map_err(|msg| {
            Box::new(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Invalid {field_name} in {config_name}: {msg}"),
            )) as Box<dyn std::error::Error>
        }),
        None => Ok(default),
    }
}

fn parse_key_code(value: &str) -> Result<KeyCode, String> {
    let normalized = value.trim().to_ascii_lowercase().replace(['-', ' '], "_");
    let key = match normalized.as_str() {
        "a" | "keya" => KeyCode::KeyA,
        "b" | "keyb" => KeyCode::KeyB,
        "c" | "keyc" => KeyCode::KeyC,
        "d" | "keyd" => KeyCode::KeyD,
        "e" | "keye" => KeyCode::KeyE,
        "f" | "keyf" => KeyCode::KeyF,
        "g" | "keyg" => KeyCode::KeyG,
        "h" | "keyh" => KeyCode::KeyH,
        "i" | "keyi" => KeyCode::KeyI,
        "j" | "keyj" => KeyCode::KeyJ,
        "k" | "keyk" => KeyCode::KeyK,
        "l" | "keyl" => KeyCode::KeyL,
        "m" | "keym" => KeyCode::KeyM,
        "n" | "keyn" => KeyCode::KeyN,
        "o" | "keyo" => KeyCode::KeyO,
        "p" | "keyp" => KeyCode::KeyP,
        "q" | "keyq" => KeyCode::KeyQ,
        "r" | "keyr" => KeyCode::KeyR,
        "s" | "keys" => KeyCode::KeyS,
        "t" | "keyt" => KeyCode::KeyT,
        "u" | "keyu" => KeyCode::KeyU,
        "v" | "keyv" => KeyCode::KeyV,
        "w" | "keyw" => KeyCode::KeyW,
        "x" | "keyx" => KeyCode::KeyX,
        "y" | "keyy" => KeyCode::KeyY,
        "z" | "keyz" => KeyCode::KeyZ,
        "0" | "digit0" => KeyCode::Digit0,
        "1" | "digit1" => KeyCode::Digit1,
        "2" | "digit2" => KeyCode::Digit2,
        "3" | "digit3" => KeyCode::Digit3,
        "4" | "digit4" => KeyCode::Digit4,
        "5" | "digit5" => KeyCode::Digit5,
        "6" | "digit6" => KeyCode::Digit6,
        "7" | "digit7" => KeyCode::Digit7,
        "8" | "digit8" => KeyCode::Digit8,
        "9" | "digit9" => KeyCode::Digit9,
        "arrowup" | "up" => KeyCode::ArrowUp,
        "arrowdown" | "down" => KeyCode::ArrowDown,
        "arrowleft" | "left" => KeyCode::ArrowLeft,
        "arrowright" | "right" => KeyCode::ArrowRight,
        "enter" | "return" => KeyCode::Enter,
        "escape" | "esc" => KeyCode::Escape,
        "space" | "spacebar" => KeyCode::Space,
        "tab" => KeyCode::Tab,
        "backspace" => KeyCode::Backspace,
        "shift" | "shiftright" | "right_shift" => KeyCode::ShiftRight,
        "shiftleft" | "left_shift" => KeyCode::ShiftLeft,
        "control" | "ctrl" | "controlleft" | "left_ctrl" => KeyCode::ControlLeft,
        "controlright" | "right_ctrl" => KeyCode::ControlRight,
        "alt" | "altleft" | "left_alt" => KeyCode::AltLeft,
        "altright" | "right_alt" => KeyCode::AltRight,
        "f1" => KeyCode::F1,
        "f2" => KeyCode::F2,
        "f3" => KeyCode::F3,
        "f4" => KeyCode::F4,
        "f5" => KeyCode::F5,
        "f6" => KeyCode::F6,
        "f7" => KeyCode::F7,
        "f8" => KeyCode::F8,
        "f9" => KeyCode::F9,
        "f10" => KeyCode::F10,
        "f11" => KeyCode::F11,
        "f12" => KeyCode::F12,
        _ => {
            return Err(format!(
                "unsupported key '{value}'. Try names like up, z, enter, right_shift, f9"
            ));
        }
    };
    Ok(key)
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

fn save_active_shader_file_to_path(
    path: &Path,
    active_file: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut root = if path.exists() {
        let contents = fs::read_to_string(path)?;
        serde_json::from_str::<Value>(&contents).map_err(|err| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to parse {}: {err}", path.display()),
            )
        })?
    } else {
        json!({})
    };

    let root_obj = root.as_object_mut().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Failed to parse {}: root must be a JSON object",
                path.display()
            ),
        )
    })?;

    let shader_value = root_obj
        .entry("shader".to_string())
        .or_insert_with(|| json!({}));
    let shader_obj = shader_value.as_object_mut().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Failed to parse {}: shader must be a JSON object",
                path.display()
            ),
        )
    })?;

    match active_file {
        Some(file) => {
            shader_obj.insert("active_file".to_string(), Value::String(file.to_string()));
        }
        None => {
            shader_obj.remove("active_file");
        }
    }

    fs::write(path, format!("{}\n", serde_json::to_string_pretty(&root)?))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::process;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static TEMP_PATH_COUNTER: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn missing_config_uses_defaults() {
        let path = unique_temp_path("missing");
        let (backend, options) =
            load_graphics_settings_from_path(&path).expect("missing config should be valid");
        assert_eq!(backend, GraphicsBackendKind::Pixels);
        let window_scale = load_window_scale_from_path(&path)
            .expect("missing config should provide default scale");
        assert!((window_scale - DEFAULT_WINDOW_SCALE).abs() < f64::EPSILON);
        let controls =
            load_controls_from_path(&path).expect("missing config should provide default controls");
        assert_eq!(controls.joypad.up, KeyCode::ArrowUp);
        assert_eq!(controls.joypad.a, KeyCode::KeyZ);
        assert_eq!(controls.hotkeys.exit, KeyCode::Escape);
        assert_eq!(controls.hotkeys.debug_frame_dump, KeyCode::F9);
        let defaults = ShaderOptions::default();
        assert_eq!(options.shader.scanline_strength, defaults.scanline_strength);
        assert_eq!(options.shader.curvature, defaults.curvature);
        assert_eq!(options.shader.color_intensity, defaults.color_intensity);
        assert_eq!(options.shader.mode, defaults.mode);
        assert_eq!(options.shader.active_file, None);
        assert_eq!(
            options.shader_directory,
            PathBuf::from(DEFAULT_SHADER_DIRECTORY)
        );
        let debug_dump = load_debug_dump_settings_from_path(&path)
            .expect("missing config should provide default debug dump settings");
        assert!(debug_dump.enabled);
        assert_eq!(
            debug_dump.output_directory,
            PathBuf::from(DEFAULT_DEBUG_DUMP_DIRECTORY)
        );
    }

    #[test]
    fn parses_wgpu_shader_backend_and_shader_values() {
        let path = write_temp_config(
            r#"{
                "graphics_backend": "wgpu_shader",
                "shader": {
                    "scanline_strength": 0.27,
                    "curvature": 0.11,
                    "color_intensity": 1.2,
                    "mode": "palette_mutation",
                    "active_file": "palette-a.wgsl",
                    "directory": "custom-shaders"
                }
            }"#,
        );

        let (backend, options) = load_graphics_settings_from_path(&path)
            .expect("valid config should parse successfully");
        assert_eq!(backend, GraphicsBackendKind::WgpuShader);
        assert!((options.shader.scanline_strength - 0.27).abs() < f32::EPSILON);
        assert!((options.shader.curvature - 0.11).abs() < f32::EPSILON);
        assert!((options.shader.color_intensity - 1.2).abs() < f32::EPSILON);
        assert_eq!(options.shader.mode, ShaderColorMode::PaletteMutation);
        assert_eq!(
            options.shader.active_file.as_deref(),
            Some("palette-a.wgsl")
        );
        assert_eq!(options.shader_directory, PathBuf::from("custom-shaders"));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn persists_active_shader_file() {
        let path = write_temp_config(
            r#"{
                "graphics_backend": "wgpu_shader",
                "shader": {
                    "scanline_strength": 0.22
                }
            }"#,
        );

        save_active_shader_file_to_path(&path, Some("scanlines.wgsl"))
            .expect("active shader should be written");
        let (_, options) =
            load_graphics_settings_from_path(&path).expect("config should remain readable");
        assert_eq!(
            options.shader.active_file.as_deref(),
            Some("scanlines.wgsl")
        );

        save_active_shader_file_to_path(&path, None).expect("active shader should be cleared");
        let (_, options) =
            load_graphics_settings_from_path(&path).expect("config should remain readable");
        assert_eq!(options.shader.active_file, None);

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
                    "curvature": -5.0,
                    "color_intensity": -3.0
                }
            }"#,
        );

        let (_, options) = load_graphics_settings_from_path(&path)
            .expect("config with out-of-range values should parse");
        assert!((options.shader.scanline_strength - 1.0).abs() < f32::EPSILON);
        assert!((options.shader.curvature - 0.0).abs() < f32::EPSILON);
        assert!((options.shader.color_intensity - 0.0).abs() < f32::EPSILON);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn rejects_invalid_shader_mode() {
        let path = write_temp_config(
            r#"{
                "graphics_backend": "wgpu_shader",
                "shader": {
                    "mode": "definitely_not_supported"
                }
            }"#,
        );

        let err = load_graphics_settings_from_path(&path)
            .expect_err("invalid shader mode should return an error");
        let msg = err.to_string();
        assert!(msg.contains("Invalid shader.mode"));
        assert!(msg.contains("classic, prism, aurora, palette_mutation"));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn parses_window_scale() {
        let path = write_temp_config(
            r#"{
                "window": {
                    "scale": 4.5
                }
            }"#,
        );

        let scale = load_window_scale_from_path(&path).expect("valid window scale should parse");
        assert!((scale - 4.5).abs() < f64::EPSILON);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn rejects_invalid_window_scale() {
        let path = write_temp_config(
            r#"{
                "window": {
                    "scale": 0.0
                }
            }"#,
        );

        let err = load_window_scale_from_path(&path)
            .expect_err("non-positive window scale should return an error");
        let msg = err.to_string();
        assert!(msg.contains("Invalid window.scale"));
        assert!(msg.contains("greater than 0"));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn parses_control_bindings() {
        let path = write_temp_config(
            r#"{
                "controls": {
                    "joypad": {
                        "up": "w",
                        "down": "s",
                        "left": "a",
                        "right": "d",
                        "a": "j",
                        "b": "k",
                        "select": "tab",
                        "start": "enter"
                    },
                    "hotkeys": {
                        "reload_graphics_config": "f5",
                        "previous_shader": "1",
                        "next_shader": "2",
                        "debug_frame_dump": "f8",
                        "exit": "esc"
                    }
                }
            }"#,
        );

        let controls = load_controls_from_path(&path).expect("valid control bindings should parse");
        assert_eq!(controls.joypad.up, KeyCode::KeyW);
        assert_eq!(controls.joypad.down, KeyCode::KeyS);
        assert_eq!(controls.joypad.left, KeyCode::KeyA);
        assert_eq!(controls.joypad.right, KeyCode::KeyD);
        assert_eq!(controls.joypad.a, KeyCode::KeyJ);
        assert_eq!(controls.joypad.b, KeyCode::KeyK);
        assert_eq!(controls.joypad.select, KeyCode::Tab);
        assert_eq!(controls.joypad.start, KeyCode::Enter);
        assert_eq!(controls.hotkeys.reload_graphics_config, KeyCode::F5);
        assert_eq!(controls.hotkeys.previous_shader, KeyCode::Digit1);
        assert_eq!(controls.hotkeys.next_shader, KeyCode::Digit2);
        assert_eq!(controls.hotkeys.debug_frame_dump, KeyCode::F8);
        assert_eq!(controls.hotkeys.exit, KeyCode::Escape);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn rejects_invalid_control_binding() {
        let path = write_temp_config(
            r#"{
                "controls": {
                    "joypad": {
                        "a": "banana"
                    }
                }
            }"#,
        );

        let err = load_controls_from_path(&path)
            .expect_err("unsupported control binding should return an error");
        let msg = err.to_string();
        assert!(msg.contains("Invalid controls.joypad.a"));
        assert!(msg.contains("unsupported key"));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn parses_debug_dump_settings() {
        let path = write_temp_config(
            r#"{
                "debug_dump": {
                    "enabled": false,
                    "output_directory": "captures/session_1"
                }
            }"#,
        );

        let settings = load_debug_dump_settings_from_path(&path)
            .expect("valid debug dump settings should parse");
        assert!(!settings.enabled);
        assert_eq!(
            settings.output_directory,
            PathBuf::from("captures/session_1")
        );

        let _ = fs::remove_file(path);
    }

    #[test]
    fn rejects_empty_shader_directory() {
        let path = write_temp_config(
            r#"{
                "shader": {
                    "directory": "   "
                }
            }"#,
        );

        let err = load_graphics_settings_from_path(&path)
            .expect_err("empty shader directory should return an error");
        let msg = err.to_string();
        assert!(msg.contains("Invalid shader.directory"));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn rejects_empty_debug_dump_directory() {
        let path = write_temp_config(
            r#"{
                "debug_dump": {
                    "output_directory": ""
                }
            }"#,
        );

        let err = load_debug_dump_settings_from_path(&path)
            .expect_err("empty debug dump directory should return an error");
        let msg = err.to_string();
        assert!(msg.contains("Invalid debug_dump.output_directory"));

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
        let counter = TEMP_PATH_COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut path = std::env::temp_dir();
        path.push(format!(
            "gabalah_{label}_{}_{}_{}.json",
            process::id(),
            timestamp,
            counter
        ));
        path
    }
}
