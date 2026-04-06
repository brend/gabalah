use super::{GraphicsBackend, GraphicsOptions, ShaderColorMode, ShaderOptions, UiResult};
use std::io;
use std::time::Instant;
use winit::window::Window;

const SHADER_SOURCE: &str = include_str!("shaders/crt.wgsl");

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
    pipeline: wgpu::RenderPipeline,
    shader_options: ShaderOptions,
    start_time: Instant,
}

impl<'win> WgpuShaderBackend<'win> {
    pub fn new(
        width: u32,
        height: u32,
        window: &'win Window,
        shader: ShaderOptions,
    ) -> UiResult<Self> {
        let shader_options = shader.clamped();

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
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::Other, "Surface is not supported by adapter")
            })?;
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

        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("gabalah-crt-shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER_SOURCE.into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("gabalah-shader-pipeline-layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let color_target = [Some(wgpu::ColorTargetState {
            format: surface_config.format,
            blend: Some(wgpu::BlendState::REPLACE),
            write_mask: wgpu::ColorWrites::ALL,
        })];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("gabalah-shader-pipeline"),
            layout: Some(&pipeline_layout),
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

        Ok(Self {
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
            pipeline,
            shader_options,
            start_time: Instant::now(),
        })
    }

    fn reconfigure_surface(&self) {
        self.surface.configure(&self.device, &self.surface_config);
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
            pass.set_pipeline(&self.pipeline);
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
        self.shader_options = options.shader.clamped();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::SHADER_SOURCE;

    #[test]
    fn crt_wgsl_shader_parses() {
        let module = naga::front::wgsl::parse_str(SHADER_SOURCE)
            .expect("crt shader should parse as valid WGSL");
        assert!(!module.entry_points.is_empty());
    }
}
