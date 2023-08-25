use std::{borrow::Cow, env, mem};

use wgpu::{
    vertex_attr_array, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingType, BlendState, BufferAddress, BufferBindingType,
    BufferSize, ColorTargetState, ColorWrites, CompareFunction, DepthStencilState, FragmentState,
    FrontFace, MultisampleState, PipelineLayoutDescriptor, PresentMode, PrimitiveState,
    RenderPipeline, RenderPipelineDescriptor, SamplerBindingType, ShaderModuleDescriptor,
    ShaderStages, TextureSampleType, TextureViewDimension, VertexBufferLayout, VertexState,
};
pub use wgpu::{
    Adapter, Backends, Device, Instance, Queue, Surface, SurfaceConfiguration, TextureFormat,
};

use crate::{
    errors::{NMSRRenderingError, Result},
    low_level::primitives::vertex::Vertex,
};

use super::scene::{Size, SunInformation};

#[derive(Debug)]
pub struct GraphicsContext {
    pub instance: Instance,
    pub device: Device,
    pub queue: Queue,
    pub surface: Option<Surface>,
    pub surface_config: Result<Option<SurfaceConfiguration>>,
    pub texture_format: TextureFormat,
    pub adapter: Adapter,

    pub pipeline: RenderPipeline,
    pub layouts: GraphicsContextLayouts,
    pub sample_count: u32,
}

#[derive(Debug)]
pub struct GraphicsContextLayouts {
    pub transform_bind_group_layout: BindGroupLayout,
    pub skin_sampler_bind_group_layout: BindGroupLayout,
    pub pipeline_layout: wgpu::PipelineLayout,
    pub sun_bind_group_layout: BindGroupLayout,
}

impl GraphicsContext {
    pub fn get_pipeline(&self) -> &RenderPipeline {
        &self.pipeline
    }
}

pub type ServiceProvider<'a> = dyn FnOnce(&Instance) -> Option<Surface> + 'a;

pub struct GraphicsContextDescriptor<'a> {
    pub backends: Option<Backends>,
    pub surface_provider: Box<ServiceProvider<'a>>,
    pub default_size: (u32, u32),
    pub texture_format: Option<TextureFormat>,
}

impl GraphicsContext {
    pub const DEFAULT_TEXTURE_FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;
    pub const DEPTH_TEXTURE_FORMAT: TextureFormat = TextureFormat::Depth32Float;

    pub async fn new(descriptor: GraphicsContextDescriptor<'_>) -> Result<Self> {
        let backends = wgpu::util::backend_bits_from_env()
            .or(descriptor.backends)
            .ok_or(NMSRRenderingError::NoBackendFound)?;

        let dx12_shader_compiler = wgpu::util::dx12_shader_compiler_from_env().unwrap_or_default();

        let instance = Instance::new(wgpu::InstanceDescriptor {
            backends,
            dx12_shader_compiler,
        });

        let mut surface = (descriptor.surface_provider)(&instance);

        let adapter =
            wgpu::util::initialize_adapter_from_env_or_default(&instance, surface.as_ref())
                .await
                .ok_or(NMSRRenderingError::NoAdapterFound)?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
                    limits: wgpu::Limits::default(),
                },
                None,
            )
            .await?;

        let (default_width, default_height) = descriptor.default_size;

        let mut surface_config = surface
            .as_mut()
            .map(|surface| {
                surface
                    .get_default_config(&adapter, default_width, default_height)
                    .ok_or(NMSRRenderingError::SurfaceNotSupported)
            })
            .transpose();

        if let Some(surface) = &surface {
            if let Ok(Some(surface_config)) = surface_config.as_mut() {
                surface_config.view_formats.push(surface_config.format);
                surface_config.present_mode = PresentMode::AutoNoVsync;
                surface.configure(&device, surface_config);
            }
        }

        let surface_view_format = {
            surface_config
                .as_ref()
                .map(|s| s.as_ref().map(|s| s.format))
        };

        let texture_format = surface_view_format
            .unwrap_or(descriptor.texture_format)
            .unwrap_or(Self::DEFAULT_TEXTURE_FORMAT);

        let adapter =
            wgpu::util::initialize_adapter_from_env_or_default(&instance, surface.as_ref())
                .await
                .ok_or(NMSRRenderingError::WgpuAdapterRequestError)?;

        // Create a bind group layout for storing the transformation matrix in a uniform
        let transform_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(64),
                    },
                    count: None,
                }],
                label: Some("Transform Bind Group Layout"),
            });

        let skin_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Texture Bind Group"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        view_dimension: TextureViewDimension::D2,
                        sample_type: TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let sun_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Sun Bind Group"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: BufferSize::new(mem::size_of::<SunInformation>() as u64),
                },
                count: None,
            }],
        });

        // Create the pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Scene Pipeline Layout"),
            bind_group_layouts: &[
                &transform_bind_group_layout,
                &skin_bind_group_layout,
                &sun_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
        });

        let vertex_buffer_layout = VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x3],
        };

        let sample_count = Self::max_available_sample_count(&adapter, &texture_format);

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[vertex_buffer_layout],
            },
            primitive: PrimitiveState {
                cull_mode: None,
                front_face: FrontFace::Cw,
                ..Default::default()
            },
            depth_stencil: Some(DepthStencilState {
                format: Self::DEPTH_TEXTURE_FORMAT,
                depth_write_enabled: true,
                depth_compare: CompareFunction::LessEqual,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: MultisampleState {
                count: sample_count,
                alpha_to_coverage_enabled: false,
                ..Default::default()
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format: texture_format,
                    blend: Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            multiview: None,
        });

        Ok(GraphicsContext {
            instance,
            device,
            queue,
            surface,
            surface_config,
            texture_format,
            adapter,
            pipeline,
            sample_count,
            layouts: GraphicsContextLayouts {
                pipeline_layout,
                transform_bind_group_layout,
                skin_sampler_bind_group_layout: skin_bind_group_layout,
                sun_bind_group_layout,
            },
        })
    }

    pub fn set_surface_size(&mut self, size: Size) {
        if let Ok(Some(config)) = &mut self.surface_config {
            config.width = size.width;
            config.height = size.height;

            if let Some(surface) = &mut self.surface {
                surface.configure(&self.device, config);
            }
        }
    }

    pub(crate) fn max_available_sample_count(
        adapter: &Adapter,
        texture_format: &TextureFormat,
    ) -> u32 {
        if let Some(count) = env::var("NMSR_SAMPLE_COUNT")
            .ok()
            .and_then(|it| it.parse::<u32>().ok())
        {
            return count;
        }

        let sample_flags = adapter.get_texture_format_features(*texture_format).flags;

        vec![16, 8, 4, 2, 1]
            .iter()
            .find(|&&sample_count| sample_flags.sample_count_supported(sample_count))
            .copied()
            .unwrap_or(1)
    }
}