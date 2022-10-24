use std::borrow::Cow;

use wgpu::{
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, BlendState, ColorTargetState,
    ColorWrites, Device, FragmentState, MultisampleState, PipelineLayoutDescriptor, PrimitiveState,
    RenderPipeline, RenderPipelineDescriptor, SamplerBindingType, ShaderModuleDescriptor,
    ShaderStages, TextureFormat, TextureSampleType, TextureViewDimension, VertexState,
};

const TEXTURE_BINDING: BindGroupLayoutEntry = BindGroupLayoutEntry {
    binding: 0,
    visibility: ShaderStages::VERTEX_FRAGMENT,
    ty: BindingType::Texture {
        sample_type: TextureSampleType::Float { filterable: true },
        view_dimension: TextureViewDimension::D2,
        multisampled: false,
    },
    count: None,
};

const SAMPLER_BINDING: BindGroupLayoutEntry = BindGroupLayoutEntry {
    binding: 1,
    visibility: ShaderStages::VERTEX_FRAGMENT,
    ty: BindingType::Sampler(SamplerBindingType::Filtering),
    count: None,
};

pub const DIRECT_PIPELINE_LAYOUT: BindGroupLayoutDescriptor = BindGroupLayoutDescriptor {
    label: None,
    entries: &[TEXTURE_BINDING, SAMPLER_BINDING],
};

pub struct ScreenPipeline {
    pub pipeline: RenderPipeline,
}

impl ScreenPipeline {
    pub fn new(device: &Device) -> Self {
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("./shader.wgsl"))),
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("agui direct pipeline"),

            layout: Some(&device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&device.create_bind_group_layout(&DIRECT_PIPELINE_LAYOUT)],
                push_constant_ranges: &[],
            })),

            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },

            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format: TextureFormat::Bgra8UnormSrgb,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),

            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
        });

        Self { pipeline }
    }
}
