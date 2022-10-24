use agui::unit::Size;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferBindingType, Extent3d,
    RenderPass, SamplerBindingType, ShaderStages, TextureDescriptor, TextureFormat,
    TextureSampleType, TextureUsages, TextureView, TextureViewDescriptor, TextureViewDimension,
};

use crate::context::RenderContext;

pub struct RenderTexture {
    pub view: TextureView,

    pub pos_buffer: Buffer,

    pub count: u32,
    pub bind_group: BindGroup,
}

impl RenderTexture {
    pub fn new(ctx: &mut RenderContext, size: Size) -> Self {
        let texture = ctx.handle.device.create_texture(&TextureDescriptor {
            label: None,
            size: Extent3d {
                width: size.width.round() as u32,
                height: size.height.round() as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
        });

        let texture_view = texture.create_view(&TextureViewDescriptor::default());

        let index_buffer = ctx.handle.device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            contents: bytemuck::cast_slice(&[0_u32, 1, 3, 1, 2, 3]),
        });

        let position_buffer = ctx.handle.device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            contents: bytemuck::cast_slice(&[0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0]),
        });

        let bind_group_layout =
            ctx.handle
                .device
                .create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: None,
                    entries: &[
                        BindGroupLayoutEntry {
                            binding: 0,
                            visibility: ShaderStages::VERTEX_FRAGMENT,
                            ty: BindingType::Buffer {
                                ty: BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        BindGroupLayoutEntry {
                            binding: 1,
                            visibility: ShaderStages::VERTEX_FRAGMENT,
                            ty: BindingType::Buffer {
                                ty: BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        BindGroupLayoutEntry {
                            binding: 2,
                            visibility: ShaderStages::VERTEX_FRAGMENT,
                            ty: BindingType::Buffer {
                                ty: BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        BindGroupLayoutEntry {
                            binding: 3,
                            visibility: ShaderStages::VERTEX_FRAGMENT,
                            ty: BindingType::Texture {
                                sample_type: TextureSampleType::Float { filterable: true },
                                view_dimension: TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        BindGroupLayoutEntry {
                            binding: 4,
                            visibility: ShaderStages::VERTEX_FRAGMENT,
                            ty: BindingType::Sampler(SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                });

        let bind_group = ctx.handle.device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: ctx.storage.render_size.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: index_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: position_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(&texture_view),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::Sampler(&ctx.storage.texture_sampler),
                },
            ],
        });

        Self {
            view: texture_view,

            pos_buffer: ctx.handle.device.create_buffer_init(&BufferInitDescriptor {
                label: None,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                contents: bytemuck::cast_slice(&[0.0, 0.0]),
            }),

            count: 0,
            bind_group,
        }
    }

    pub fn render<'r>(&'r self, r: &mut RenderPass<'r>) {
        r.set_bind_group(0, &self.bind_group, &[]);

        r.set_vertex_buffer(0, self.pos_buffer.slice(..));

        r.draw(0..6, 0..1);
    }
}
