use std::{borrow::Cow, mem};

use wgpu::{
    vertex_attr_array, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType,
    BufferBindingType, ColorTargetState, ColorWrites, Device, FragmentState, MultisampleState,
    PipelineLayoutDescriptor, PrimitiveState, RenderPipeline, RenderPipelineDescriptor,
    SamplerBindingType, ShaderModuleDescriptor, ShaderStages, TextureFormat, TextureSampleType,
    TextureViewDimension, VertexBufferLayout, VertexState, VertexStepMode, BlendState,
};

use crate::render::data::{InstanceData, VertexData};

const INSTANCE_LAYOUT: VertexBufferLayout = VertexBufferLayout {
    array_stride: mem::size_of::<InstanceData>() as u64,
    step_mode: VertexStepMode::Instance,
    attributes: &vertex_attr_array![0 => Float32x2],
};

const VERTEX_LAYOUT: VertexBufferLayout = VertexBufferLayout {
    array_stride: mem::size_of::<VertexData>() as u64,
    step_mode: VertexStepMode::Vertex,
    attributes: &vertex_attr_array![1 => Float32x4],
};

const VIEWPORT_BINDING: BindGroupLayoutEntry = BindGroupLayoutEntry {
    binding: 0,
    visibility: ShaderStages::VERTEX_FRAGMENT,
    ty: BindingType::Buffer {
        ty: BufferBindingType::Uniform,
        has_dynamic_offset: false,
        min_binding_size: None,
    },
    count: None,
};

const OPTIONS_BINDING: BindGroupLayoutEntry = BindGroupLayoutEntry {
    binding: 1,
    visibility: ShaderStages::VERTEX_FRAGMENT,
    ty: BindingType::Buffer {
        ty: BufferBindingType::Uniform,
        has_dynamic_offset: false,
        min_binding_size: None,
    },
    count: None,
};

const INDICES_BINDING: BindGroupLayoutEntry = BindGroupLayoutEntry {
    binding: 2,
    visibility: ShaderStages::VERTEX_FRAGMENT,
    ty: BindingType::Buffer {
        ty: BufferBindingType::Storage { read_only: true },
        has_dynamic_offset: false,
        min_binding_size: None,
    },
    count: None,
};

const POSITIONS_BINDING: BindGroupLayoutEntry = BindGroupLayoutEntry {
    binding: 3,
    visibility: ShaderStages::VERTEX_FRAGMENT,
    ty: BindingType::Buffer {
        ty: BufferBindingType::Storage { read_only: true },
        has_dynamic_offset: false,
        min_binding_size: None,
    },
    count: None,
};

const TEXTURE_BINDING: BindGroupLayoutEntry = BindGroupLayoutEntry {
    binding: 4,
    visibility: ShaderStages::VERTEX_FRAGMENT,
    ty: BindingType::Texture {
        sample_type: TextureSampleType::Float { filterable: true },
        view_dimension: TextureViewDimension::D2,
        multisampled: false,
    },
    count: None,
};

const SAMPLER_BINDING: BindGroupLayoutEntry = BindGroupLayoutEntry {
    binding: 5,
    visibility: ShaderStages::VERTEX_FRAGMENT,
    ty: BindingType::Sampler(SamplerBindingType::Filtering),
    count: None,
};

pub const PAINT_PIPELINE_LAYOUT: BindGroupLayoutDescriptor = BindGroupLayoutDescriptor {
    label: None,
    entries: &[
        VIEWPORT_BINDING,
        OPTIONS_BINDING,
        INDICES_BINDING,
        POSITIONS_BINDING,
        TEXTURE_BINDING,
        SAMPLER_BINDING,
    ],
};

pub fn create(device: &Device) -> RenderPipeline {
    let shader = device.create_shader_module(ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("../shaders/layer.wgsl"))),
    });

    device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("agui paint pipeline"),

        layout: Some(&device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&device.create_bind_group_layout(&PAINT_PIPELINE_LAYOUT)],
            push_constant_ranges: &[],
        })),

        vertex: VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[INSTANCE_LAYOUT, VERTEX_LAYOUT],
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
    })
}
