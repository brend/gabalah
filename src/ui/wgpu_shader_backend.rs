use super::{GraphicsBackend, GraphicsOptions, ShaderColorMode, ShaderOptions, UiResult};
use log::{debug, warn};
use naga::{AddressSpace, ImageClass, ScalarKind, ShaderStage, TypeInner};
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Instant;
#[cfg(target_os = "windows")]
use wgpu::Backends;
use winit::window::Window;

const BUILTIN_SHADER_SOURCE: &str = include_str!("shaders/crt.wgsl");
const BUILTIN_SHADER_LABEL: &str = "builtin-crt";
const REQUIRED_UNIFORM_FIELDS: usize = 8;

#[derive(Debug, Clone, Copy)]
struct ShaderUniforms {
    time_seconds: f32,
    scanline_strength: f32,
    curvature: f32,
    color_intensity: f32,
    color_mode: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
}

impl ShaderUniforms {
    fn to_bytes(self) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        bytes[0..4].copy_from_slice(&self.time_seconds.to_ne_bytes());
        bytes[4..8].copy_from_slice(&self.scanline_strength.to_ne_bytes());
        bytes[8..12].copy_from_slice(&self.curvature.to_ne_bytes());
        bytes[12..16].copy_from_slice(&self.color_intensity.to_ne_bytes());
        bytes[16..20].copy_from_slice(&self.color_mode.to_ne_bytes());
        bytes[20..24].copy_from_slice(&self._pad0.to_ne_bytes());
        bytes[24..28].copy_from_slice(&self._pad1.to_ne_bytes());
        bytes[28..32].copy_from_slice(&self._pad2.to_ne_bytes());
        bytes
    }
}

#[derive(Debug)]
struct ShaderSource {
    file_name: String,
    source: String,
}

struct ShaderProgram {
    file_name: Option<String>,
    pipeline: wgpu::RenderPipeline,
}

impl ShaderColorMode {
    const fn as_uniform_value(self) -> f32 {
        match self {
            Self::Classic => 0.0,
            Self::Prism => 1.0,
            Self::Aurora => 2.0,
            Self::PaletteMutation => 3.0,
        }
    }
}

pub struct WgpuShaderBackend<'win> {
    width: u32,
    height: u32,
    frame: Vec<u8>,
    surface: wgpu::Surface<'win>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    frame_texture: wgpu::Texture,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    pipeline_layout: wgpu::PipelineLayout,
    shader_programs: Vec<ShaderProgram>,
    active_shader_index: usize,
    shader_options: ShaderOptions,
    shader_directory: PathBuf,
    start_time: Instant,
}

impl<'win> WgpuShaderBackend<'win> {
    pub fn new(
        width: u32,
        height: u32,
        window: &'win Window,
        options: GraphicsOptions,
    ) -> UiResult<Self> {
        let shader_options = options.shader.clamped();

        #[cfg(target_os = "windows")]
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            // Keep the custom shader backend aligned with the pixels backend:
            // some Windows DX12 drivers fail swapchain render-target transitions.
            backends: Backends::VULKAN | Backends::GL,
            ..Default::default()
        });
        #[cfg(not(target_os = "windows"))]
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window)?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No suitable GPU adapter found"))?;

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("gabalah-wgpu-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
            },
            None,
        ))?;

        let mut surface_config = surface
            .get_default_config(
                &adapter,
                window.inner_size().width.max(1),
                window.inner_size().height.max(1),
            )
            .ok_or_else(|| io::Error::other("Surface is not supported by adapter"))?;
        let capabilities = surface.get_capabilities(&adapter);
        if let Some(srgb_format) = capabilities
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
        {
            surface_config.format = srgb_format;
        }
        if capabilities
            .present_modes
            .contains(&wgpu::PresentMode::Fifo)
        {
            surface_config.present_mode = wgpu::PresentMode::Fifo;
        }
        surface.configure(&device, &surface_config);

        let frame_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("gabalah-frame-texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let frame_view = frame_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let frame_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("gabalah-frame-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let uniforms = ShaderUniforms {
            time_seconds: 0.0,
            scanline_strength: shader_options.scanline_strength,
            curvature: shader_options.curvature,
            color_intensity: shader_options.color_intensity,
            color_mode: shader_options.mode.as_uniform_value(),
            _pad0: 0.0,
            _pad1: 0.0,
            _pad2: 0.0,
        };
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gabalah-shader-uniforms"),
            size: 32,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&uniform_buffer, 0, &uniforms.to_bytes());

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("gabalah-shader-bind-group-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("gabalah-shader-bind-group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&frame_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&frame_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("gabalah-shader-pipeline-layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let mut backend = Self {
            width,
            height,
            frame: vec![0; (width * height * 4) as usize],
            surface,
            device,
            queue,
            surface_config,
            frame_texture,
            uniform_buffer,
            bind_group,
            pipeline_layout,
            shader_programs: Vec::new(),
            active_shader_index: 0,
            shader_options,
            shader_directory: options.shader_directory,
            start_time: Instant::now(),
        };

        let preferred_active_file = backend.shader_options.active_file.clone();
        backend.reload_shader_library(preferred_active_file.as_deref())?;
        Ok(backend)
    }

    fn reconfigure_surface(&self) {
        self.surface.configure(&self.device, &self.surface_config);
    }

    fn active_shader_file(&self) -> Option<&str> {
        self.shader_programs
            .get(self.active_shader_index)
            .and_then(|program| program.file_name.as_deref())
    }

    fn active_pipeline(&self) -> &wgpu::RenderPipeline {
        &self.shader_programs[self.active_shader_index].pipeline
    }

    fn cycle_shader(&mut self, step: isize) -> Option<String> {
        if self.shader_programs.is_empty() {
            return None;
        }
        let len = self.shader_programs.len() as isize;
        let current = self.active_shader_index as isize;
        self.active_shader_index = (current + step).rem_euclid(len) as usize;

        let active = self.active_shader_file().map(str::to_string);
        self.shader_options.active_file = active.clone();
        active
    }

    fn set_active_shader_file(&mut self, file_name: &str) -> bool {
        if let Some(index) = self
            .shader_programs
            .iter()
            .position(|program| program.file_name.as_deref() == Some(file_name))
        {
            self.active_shader_index = index;
            self.shader_options.active_file = Some(file_name.to_string());
            return true;
        }
        false
    }

    fn compile_shader_program(
        &self,
        source: &str,
        label: &str,
        file_name: Option<String>,
    ) -> UiResult<ShaderProgram> {
        validate_shader_contract(source).map_err(|err| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("shader '{label}' violates required contract: {err}"),
            )
        })?;

        self.device.push_error_scope(wgpu::ErrorFilter::Validation);

        let shader_module = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });
        let color_target = [Some(wgpu::ColorTargetState {
            format: self.surface_config.format,
            blend: Some(wgpu::BlendState::REPLACE),
            write_mask: wgpu::ColorWrites::ALL,
        })];
        let pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("gabalah-shader-pipeline"),
                layout: Some(&self.pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader_module,
                    entry_point: "vs_main",
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader_module,
                    entry_point: "fs_main",
                    targets: &color_target,
                }),
                multiview: None,
            });

        if let Some(err) = pollster::block_on(self.device.pop_error_scope()) {
            return Err(Box::new(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("failed to compile shader '{label}': {err}"),
            )));
        }

        Ok(ShaderProgram {
            file_name,
            pipeline,
        })
    }

    fn reload_shader_programs(
        &mut self,
        preferred_active_file: Option<&str>,
    ) -> UiResult<Option<String>> {
        let previous_active = self.active_shader_file().map(str::to_string);

        let shader_sources = load_runtime_shaders(&self.shader_directory)?;
        let mut shader_programs = Vec::new();

        for shader_source in shader_sources {
            match self.compile_shader_program(
                &shader_source.source,
                &shader_source.file_name,
                Some(shader_source.file_name.clone()),
            ) {
                Ok(program) => shader_programs.push(program),
                Err(err) => warn!("Skipping shader '{}': {err}", shader_source.file_name),
            }
        }

        if shader_programs.is_empty() {
            warn!(
                "No valid runtime shaders found in '{}'; using built-in fallback",
                self.shader_directory.display()
            );
            shader_programs.push(self.compile_shader_program(
                BUILTIN_SHADER_SOURCE,
                BUILTIN_SHADER_LABEL,
                None,
            )?);
        }

        let desired_active = preferred_active_file.or(previous_active.as_deref());
        let mut active_index = 0;
        if let Some(desired_active) = desired_active {
            if let Some(index) = shader_programs
                .iter()
                .position(|program| program.file_name.as_deref() == Some(desired_active))
            {
                active_index = index;
            } else {
                warn!(
                    "Requested active shader '{}' not available; falling back to first shader",
                    desired_active
                );
            }
        }

        self.shader_programs = shader_programs;
        self.active_shader_index = active_index;
        let active = self.active_shader_file().map(str::to_string);
        self.shader_options.active_file = active.clone();

        if let Some(active_name) = active.as_deref() {
            debug!("Active shader: {active_name}");
        } else {
            debug!("Active shader: built-in fallback");
        }

        Ok(active)
    }
}

impl GraphicsBackend for WgpuShaderBackend<'_> {
    fn frame_mut(&mut self) -> &mut [u8] {
        &mut self.frame
    }

    fn present(&mut self) -> UiResult<()> {
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.frame_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &self.frame,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * self.width),
                rows_per_image: Some(self.height),
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );

        let uniforms = ShaderUniforms {
            time_seconds: self.start_time.elapsed().as_secs_f32(),
            scanline_strength: self.shader_options.scanline_strength,
            curvature: self.shader_options.curvature,
            color_intensity: self.shader_options.color_intensity,
            color_mode: self.shader_options.mode.as_uniform_value(),
            _pad0: 0.0,
            _pad1: 0.0,
            _pad2: 0.0,
        };
        self.queue
            .write_buffer(&self.uniform_buffer, 0, &uniforms.to_bytes());

        let output = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.reconfigure_surface();
                self.surface.get_current_texture()?
            }
            Err(err) => return Err(Box::new(err)),
        };

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("gabalah-shader-render-encoder"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("gabalah-shader-render-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(self.active_pipeline());
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        self.queue.submit(Some(encoder.finish()));
        output.present();
        Ok(())
    }

    fn resize_surface(&mut self, width: u32, height: u32) -> UiResult<()> {
        self.surface_config.width = width.max(1);
        self.surface_config.height = height.max(1);
        self.reconfigure_surface();
        Ok(())
    }

    fn reload_options(&mut self, options: GraphicsOptions) -> UiResult<()> {
        self.shader_directory = options.shader_directory;
        self.shader_options = options.shader.clamped();
        if let Some(ref active_file) = self.shader_options.active_file.clone() {
            if !self.set_active_shader_file(active_file) {
                warn!(
                    "Configured active shader '{}' not present in loaded shader list",
                    active_file
                );
            }
        }
        Ok(())
    }

    fn cycle_shader_next(&mut self) -> UiResult<Option<String>> {
        Ok(self.cycle_shader(1))
    }

    fn cycle_shader_prev(&mut self) -> UiResult<Option<String>> {
        Ok(self.cycle_shader(-1))
    }

    fn reload_shader_library(
        &mut self,
        preferred_active_file: Option<&str>,
    ) -> UiResult<Option<String>> {
        self.reload_shader_programs(preferred_active_file)
    }
}

fn load_runtime_shaders(
    shader_dir: &Path,
) -> Result<Vec<ShaderSource>, Box<dyn std::error::Error>> {
    let mut shaders = Vec::new();
    for shader_path in discover_shader_files(shader_dir)? {
        let source = fs::read_to_string(&shader_path)?;
        let file_name = shader_path
            .file_name()
            .and_then(OsStr::to_str)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Shader path '{}' is not valid UTF-8", shader_path.display()),
                )
            })?
            .to_string();
        shaders.push(ShaderSource { file_name, source });
    }
    Ok(shaders)
}

fn discover_shader_files(shader_dir: &Path) -> io::Result<Vec<PathBuf>> {
    if !shader_dir.exists() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    for entry in fs::read_dir(shader_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(ext) = path.extension().and_then(OsStr::to_str) else {
            continue;
        };
        if ext.eq_ignore_ascii_case("wgsl") {
            files.push(path);
        }
    }

    files.sort_by(|left, right| {
        left.file_name()
            .unwrap_or_default()
            .cmp(right.file_name().unwrap_or_default())
    });
    Ok(files)
}

fn validate_shader_contract(source: &str) -> Result<(), String> {
    let module =
        naga::front::wgsl::parse_str(source).map_err(|err| format!("invalid WGSL: {err}"))?;

    let mut has_vs_main = false;
    let mut has_fs_main = false;
    for entry in &module.entry_points {
        if entry.stage == ShaderStage::Vertex && entry.name == "vs_main" {
            has_vs_main = true;
        }
        if entry.stage == ShaderStage::Fragment && entry.name == "fs_main" {
            has_fs_main = true;
        }
    }
    if !has_vs_main || !has_fs_main {
        return Err(
            "expected entry points `vs_main` (vertex) and `fs_main` (fragment)".to_string(),
        );
    }

    let mut texture_ok = false;
    let mut sampler_ok = false;
    let mut uniform_ok = false;

    for (_, global) in module.global_variables.iter() {
        let Some(ref binding) = global.binding else {
            continue;
        };
        if binding.group != 0 {
            continue;
        }

        let ty = &module.types[global.ty].inner;
        match binding.binding {
            0 => {
                if matches!(
                    ty,
                    TypeInner::Image {
                        dim: naga::ImageDimension::D2,
                        arrayed: false,
                        class: ImageClass::Sampled {
                            kind: ScalarKind::Float,
                            multi: false,
                        }
                    }
                ) {
                    texture_ok = true;
                }
            }
            1 => {
                if matches!(ty, TypeInner::Sampler { comparison: false }) {
                    sampler_ok = true;
                }
            }
            2 => {
                if global.space == AddressSpace::Uniform
                    && uniform_struct_matches(&module, global.ty)
                {
                    uniform_ok = true;
                }
            }
            _ => {}
        }
    }

    if !texture_ok {
        return Err("missing binding @group(0) @binding(0) texture_2d<f32>".to_string());
    }
    if !sampler_ok {
        return Err("missing binding @group(0) @binding(1) sampler".to_string());
    }
    if !uniform_ok {
        return Err(
            "missing binding @group(0) @binding(2) uniform struct with 8 f32 fields".to_string(),
        );
    }

    Ok(())
}

fn uniform_struct_matches(module: &naga::Module, ty_handle: naga::Handle<naga::Type>) -> bool {
    let TypeInner::Struct { members, .. } = &module.types[ty_handle].inner else {
        return false;
    };
    if members.len() != REQUIRED_UNIFORM_FIELDS {
        return false;
    }
    for member in members {
        if !is_f32_scalar(module, member.ty) {
            return false;
        }
    }
    true
}

fn is_f32_scalar(module: &naga::Module, ty_handle: naga::Handle<naga::Type>) -> bool {
    matches!(
        module.types[ty_handle].inner,
        TypeInner::Scalar(naga::Scalar {
            kind: ScalarKind::Float,
            width: 4,
        })
    )
}

#[cfg(test)]
mod tests {
    use super::{discover_shader_files, validate_shader_contract, BUILTIN_SHADER_SOURCE};
    use std::fs;
    use std::path::PathBuf;
    use std::process;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn crt_wgsl_shader_parses() {
        let module = naga::front::wgsl::parse_str(BUILTIN_SHADER_SOURCE)
            .expect("crt shader should parse as valid WGSL");
        assert!(!module.entry_points.is_empty());
    }

    #[test]
    fn validates_builtin_shader_contract() {
        validate_shader_contract(BUILTIN_SHADER_SOURCE)
            .expect("builtin shader should satisfy runtime contract");
    }

    #[test]
    fn rejects_shader_missing_entry_points() {
        let broken = BUILTIN_SHADER_SOURCE.replace("fn vs_main", "fn vs_main_missing");
        let err = validate_shader_contract(&broken)
            .expect_err("shader without required entry points should fail");
        assert!(err.contains("vs_main"));
    }

    #[test]
    fn rejects_uniform_shape_mismatch() {
        let broken = BUILTIN_SHADER_SOURCE.replace("_pad2: f32,", "_pad2: vec2<f32>,");
        let err = validate_shader_contract(&broken)
            .expect_err("shader with wrong uniform shape should fail");
        assert!(err.contains("uniform"));
    }

    #[test]
    fn discovers_wgsl_files_in_sorted_order() {
        let dir = unique_temp_dir("shader_scan");
        fs::create_dir_all(&dir).expect("temp dir should be created");
        fs::write(dir.join("b.wgsl"), "// b").expect("write should succeed");
        fs::write(dir.join("a.wgsl"), "// a").expect("write should succeed");
        fs::write(dir.join("ignore.txt"), "ignore").expect("write should succeed");

        let files = discover_shader_files(&dir).expect("shader discovery should succeed");
        let names: Vec<String> = files
            .iter()
            .map(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .expect("file name should be utf-8")
                    .to_string()
            })
            .collect();
        assert_eq!(names, vec!["a.wgsl", "b.wgsl"]);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn returns_empty_when_shader_directory_is_missing() {
        let dir = unique_temp_dir("shader_missing");
        let files = discover_shader_files(&dir).expect("missing shader dir should be allowed");
        assert!(files.is_empty());
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be after unix epoch")
            .as_nanos();
        let mut path = std::env::temp_dir();
        path.push(format!("gabalah_{label}_{}_{}", process::id(), timestamp));
        path
    }
}
