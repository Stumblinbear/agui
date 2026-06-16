use std::num::NonZeroUsize;

use agui_core::paint::peniko::Color;
use agui_core::paint::scene::Scene;
use agui_render::TextureRenderer;
use agui_render::wgpu;
use vello::kurbo::Affine;
use vello::{AaConfig, AaSupport, RenderParams, Renderer, RendererOptions};

use crate::append_scene_with_transform;

/// A [`TextureRenderer`] that rasterizes a frame with Vello and draws it into a caller-provided
/// texture, holding its own GPU context with no window or surface.
///
/// It rasterizes the whole frame into one scene, so a frame placing a system-owned
/// [`External`](agui_core::paint::compositing::CompositedNode::External) surface, which has no
/// rasterization, has nothing to draw for that surface.
pub struct VelloRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    renderer: Renderer,
}

impl VelloRenderer {
    /// Creates a renderer on a default GPU adapter, or `None` if none is available, so a caller on a
    /// machine without a GPU can skip rather than fail.
    pub fn new() -> Option<Self> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());

        let adapter = pollster::block_on(wgpu::util::initialize_adapter_from_env_or_default(
            &instance, None,
        ))
        .ok()?;

        Self::from_adapter(&adapter)
    }

    /// Creates a renderer on a high-performance adapter restricted to `backends`, or `None` if none is
    /// available. A consumer that requires a specific backend, such as a DirectComposition compositor
    /// requiring `Backends::DX12`, selects it here instead of accepting the default adapter.
    pub fn with_backends(backends: wgpu::Backends) -> Option<Self> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends,
            ..Default::default()
        });

        let adapter = pollster::block_on(wgpu::util::initialize_adapter_from_env_or_default(
            &instance, None,
        ))
        .ok()?;

        Self::from_adapter(&adapter)
    }

    fn from_adapter(adapter: &wgpu::Adapter) -> Option<Self> {
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: None,
            required_features: adapter.features() & wgpu::Features::CLEAR_TEXTURE,
            required_limits: wgpu::Limits::default(),
            experimental_features: wgpu::ExperimentalFeatures::default(),
            memory_hints: wgpu::MemoryHints::default(),
            trace: wgpu::Trace::Off,
        }))
        .ok()?;

        let renderer = Renderer::new(
            &device,
            RendererOptions {
                use_cpu: false,
                antialiasing_support: AaSupport::area_only(),
                num_init_threads: NonZeroUsize::new(1),
                pipeline_cache: None,
            },
        )
        .ok()?;

        Some(Self {
            device,
            queue,
            renderer,
        })
    }
}

impl TextureRenderer for VelloRenderer {
    const NAME: &'static str = "vello";

    fn device(&self) -> &wgpu::Device {
        &self.device
    }

    fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    fn render(
        &mut self,
        scene: &Scene,
        target: &wgpu::TextureView,
        width: u32,
        height: u32,
        scale: f64,
    ) {
        let mut vello_scene = vello::Scene::new();
        append_scene_with_transform(scene, &mut vello_scene, Affine::scale(scale));

        self.renderer
            .render_to_texture(
                &self.device,
                &self.queue,
                &vello_scene,
                target,
                &RenderParams {
                    base_color: Color::TRANSPARENT,
                    width,
                    height,
                    antialiasing_method: AaConfig::Area,
                },
            )
            .expect("render to texture");
    }
}

impl Default for VelloRenderer {
    fn default() -> Self {
        Self::new().expect("a GPU adapter is required")
    }
}
