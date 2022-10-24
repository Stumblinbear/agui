use agui::unit::Size;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindingResource, CommandEncoderDescriptor,
    Extent3d, RenderPass, Texture, TextureDescriptor, TextureDimension, TextureFormat,
    TextureUsages, TextureView, TextureViewDescriptor,
};

use crate::{context::RenderContext, pipelines::screen::DIRECT_PIPELINE_LAYOUT};

pub struct RenderTexture {
    pub size: Size,

    pub texture: Texture,
    pub view: TextureView,

    pub bind_group: BindGroup,
}

impl RenderTexture {
    pub fn new(ctx: &RenderContext, size: Size) -> Self {
        let texture = ctx.handle.device.create_texture(&TextureDescriptor {
            label: None,
            size: Extent3d {
                width: size.width.round() as u32,
                height: size.height.round() as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::RENDER_ATTACHMENT
                | TextureUsages::COPY_SRC
                | TextureUsages::COPY_DST,
        });

        let view = texture.create_view(&TextureViewDescriptor::default());

        let bind_group_layout = ctx
            .handle
            .device
            .create_bind_group_layout(&DIRECT_PIPELINE_LAYOUT);

        let bind_group = ctx.handle.device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&ctx.storage.texture_sampler),
                },
            ],
        });

        Self {
            size,

            texture,
            view,

            bind_group,
        }
    }

    pub fn copy_and_resize(&mut self, ctx: &RenderContext, size: Size) -> Self {
        let new_texture = Self::new(ctx, size);

        let mut encoder = ctx
            .handle
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("agui texture resize encoder"),
            });

        encoder.copy_texture_to_texture(
            self.texture.as_image_copy(),
            new_texture.texture.as_image_copy(),
            Extent3d {
                width: self.size.width.round() as u32,
                height: self.size.height.round() as u32,
                depth_or_array_layers: 1,
            },
        );

        ctx.handle.queue.submit([encoder.finish()]);

        new_texture
    }

    pub fn render<'r>(&'r self, r: &mut RenderPass<'r>) {
        r.set_bind_group(0, &self.bind_group, &[]);

        r.draw(0..6, 0..1);
    }
}
