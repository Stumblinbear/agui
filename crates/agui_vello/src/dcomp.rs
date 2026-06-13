//! A [`WindowRenderer`] that composites an agui frame through DirectComposition (Windows).
//!
//! It walks the [`CompositedFrame`] tree into a DirectComposition visual tree: a
//! [`Surface`](CompositedNode::Surface) becomes a visual with its transform and opacity, with its
//! children nested beneath it, and its [`SurfacePlacement`] handles are filled with a visual-backed
//! [`CompositorVisual`] so a layer drives the visual off the scene; a [`Raster`](CompositedNode::Raster)
//! is rendered by vello into a composition swapchain reused across frames by scene identity, so an
//! animating transform does not re-rasterize its content; an [`External`](CompositedNode::External)
//! becomes a sibling visual holding a placeholder swapchain in lieu of a decoded surface.

#![allow(
    unsafe_op_in_unsafe_fn,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use std::sync::Arc;
use std::{ffi::c_void, mem::ManuallyDrop, num::NonZeroUsize, rc::Rc};

use agui_core::paint::compositing::{
    CompositedFrame, CompositedNode, CompositorVisual, SurfacePlacement,
};
use agui_window::WindowRenderer;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawWindowHandle};
use rustc_hash::FxHashMap;
use vello::{
    AaConfig, AaSupport, RenderParams, Renderer, RendererOptions, kurbo::Affine, peniko::Color,
    wgpu,
};
use windows::Foundation::Numerics::Matrix3x2;
use windows::Win32::{
    Foundation::{CloseHandle, HWND},
    Graphics::{
        Direct3D12::{
            D3D12_COMMAND_LIST_TYPE_DIRECT, D3D12_DESCRIPTOR_HEAP_DESC,
            D3D12_DESCRIPTOR_HEAP_FLAG_NONE, D3D12_DESCRIPTOR_HEAP_TYPE_RTV, D3D12_FENCE_FLAG_NONE,
            D3D12_RESOURCE_BARRIER, D3D12_RESOURCE_BARRIER_0,
            D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES, D3D12_RESOURCE_BARRIER_FLAG_NONE,
            D3D12_RESOURCE_BARRIER_TYPE_TRANSITION, D3D12_RESOURCE_STATE_PRESENT,
            D3D12_RESOURCE_STATE_RENDER_TARGET, D3D12_RESOURCE_STATES,
            D3D12_RESOURCE_TRANSITION_BARRIER, ID3D12CommandAllocator, ID3D12CommandList,
            ID3D12CommandQueue, ID3D12DescriptorHeap, ID3D12Device, ID3D12Fence,
            ID3D12GraphicsCommandList, ID3D12Resource,
        },
        DirectComposition::{
            DCompositionCreateDevice, IDCompositionDevice, IDCompositionTarget, IDCompositionVisual,
        },
        Dxgi::{
            Common::{DXGI_ALPHA_MODE_PREMULTIPLIED, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC},
            CreateDXGIFactory2, DXGI_CREATE_FACTORY_FLAGS, DXGI_PRESENT, DXGI_SCALING_STRETCH,
            DXGI_SWAP_CHAIN_DESC1, DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
            DXGI_USAGE_RENDER_TARGET_OUTPUT, IDXGIDevice, IDXGIFactory2, IDXGISwapChain3,
        },
    },
    System::Threading::{CreateEventW, INFINITE, WaitForSingleObject},
};
use windows::core::Interface;

use crate::append_scene_with_transform;

/// Identifies a retained raster swapchain by the identity of the scene it holds, so an unchanged
/// scene reuses its swapchain rather than re-rasterizing.
type SceneId = *const ();

/// A [`WindowRenderer`] that composites a frame through DirectComposition.
pub struct DcompWindowRenderer {
    inner: Option<Inner>,
}

struct Inner {
    device: wgpu::Device,
    queue: wgpu::Queue,
    raw_device: ID3D12Device,
    raw_queue: ID3D12CommandQueue,
    renderer: Renderer,
    factory: IDXGIFactory2,

    dcomp: IDCompositionDevice,
    _target: IDCompositionTarget,
    root: IDCompositionVisual,

    /// A window-sized texture every rasterized node renders into before being blitted to its
    /// swapchain; reused since nodes are composed one at a time.
    target_view: wgpu::TextureView,
    blitter: PremultBlit,

    width: u32,
    height: u32,

    /// Composition swapchains retained across frames by the identity of the scene they hold.
    swapchains: FxHashMap<SceneId, IDXGISwapChain3>,

    /// The structure of the visual tree currently built, so a present whose frame has the same
    /// structure is skipped: an animated transform moves its visual through its filled handle, off
    /// the scene, with no rebuild.
    structure: Vec<usize>,
}

impl DcompWindowRenderer {
    pub fn new() -> Self {
        Self { inner: None }
    }
}

impl Default for DcompWindowRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowRenderer for DcompWindowRenderer {
    fn attach<W>(&mut self, window: Arc<W>, width: u32, height: u32)
    where
        W: HasWindowHandle + HasDisplayHandle + Send + Sync + 'static,
    {
        let hwnd = hwnd_of(&*window);
        self.inner = Some(unsafe { Inner::new(hwnd, width.max(1), height.max(1)) });
    }

    fn resize(&mut self, width: u32, height: u32) {
        if let Some(inner) = self.inner.as_mut() {
            unsafe { inner.resize(width.max(1), height.max(1)) };
        }
    }

    fn present(&mut self, frame: &CompositedFrame, scale_factor: f64) {
        if let Some(inner) = self.inner.as_mut() {
            unsafe { inner.present(frame, scale_factor) };
        }
    }
}

impl Inner {
    unsafe fn new(hwnd: HWND, width: u32, height: u32) -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::DX12,
            ..Default::default()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        }))
        .expect("a DX12 adapter");
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .expect("a DX12 device");

        let raw_device = device
            .as_hal::<wgpu::hal::api::Dx12>()
            .expect("dx12 device")
            .raw_device()
            .clone();
        let raw_queue = queue
            .as_hal::<wgpu::hal::api::Dx12>()
            .expect("dx12 queue")
            .as_raw()
            .clone();

        let renderer = Renderer::new(
            &device,
            RendererOptions {
                use_cpu: false,
                antialiasing_support: AaSupport::area_only(),
                num_init_threads: NonZeroUsize::new(1),
                pipeline_cache: None,
            },
        )
        .expect("vello renderer");

        let factory: IDXGIFactory2 =
            CreateDXGIFactory2(DXGI_CREATE_FACTORY_FLAGS(0)).expect("dxgi factory");
        let dcomp: IDCompositionDevice =
            DCompositionCreateDevice(None::<&IDXGIDevice>).expect("dcomp device");
        let target = dcomp.CreateTargetForHwnd(hwnd, true).expect("dcomp target");
        let root = dcomp.CreateVisual().expect("root visual");
        target.SetRoot(&root).expect("set root");

        let (target_view, blitter) = make_target(&device, width, height);

        Self {
            device,
            queue,
            raw_device,
            raw_queue,
            renderer,
            factory,
            dcomp,
            _target: target,
            root,
            target_view,
            blitter,
            width,
            height,
            swapchains: FxHashMap::default(),
            structure: Vec::new(),
        }
    }

    unsafe fn resize(&mut self, width: u32, height: u32) {
        if (width, height) == (self.width, self.height) {
            return;
        }
        self.width = width;
        self.height = height;
        let (target_view, blitter) = make_target(&self.device, width, height);
        self.target_view = target_view;
        self.blitter = blitter;
        // Drop every swapchain; they are window-sized and must be rebuilt at the new size. Clearing the
        // structure forces the next present to rebuild the visual tree at the new size.
        self.swapchains.clear();
        self.structure.clear();
    }

    unsafe fn present(&mut self, frame: &CompositedFrame, scale_factor: f64) {
        // When the structure is unchanged the visuals already exist and the off-thread pokes have set
        // their transforms, so commit those without rebuilding.
        let structure = structure_of(frame);
        if structure == self.structure {
            self.dcomp.Commit().expect("commit");

            return;
        }

        // Rebuild the visual tree, reusing a scene's retained swapchain so unchanged content is not
        // re-rasterized. Swapchains not used this frame are dropped at the end.
        self.root.RemoveAllVisuals().expect("clear root");
        let mut retained: FxHashMap<SceneId, IDXGISwapChain3> = FxHashMap::default();

        let root = self.root.clone();
        self.add_nodes(&root, frame, scale_factor, &mut retained);

        self.swapchains = retained;
        self.structure = structure;
        self.dcomp.Commit().expect("commit");
    }

    /// Adds `frame`'s nodes as children of `parent`, in order. Rasterized content is rendered at
    /// physical resolution (scaled by `scale`); a surface's transform is conjugated into physical
    /// space and placed on its visual, which DirectComposition composes beneath `parent`.
    unsafe fn add_nodes(
        &mut self,
        parent: &IDCompositionVisual,
        frame: &CompositedFrame,
        scale: f64,
        retained: &mut FxHashMap<SceneId, IDXGISwapChain3>,
    ) {
        for node in frame.nodes() {
            match node {
                CompositedNode::Raster { scene } => {
                    let id = Rc::as_ptr(scene).cast::<()>();
                    let swapchain = match self.swapchains.remove(&id) {
                        Some(swapchain) => swapchain,
                        None => {
                            let swapchain = make_composition_swapchain(
                                &self.factory,
                                &self.raw_queue,
                                self.width,
                                self.height,
                            );
                            let mut vello_scene = vello::Scene::new();
                            append_scene_with_transform(
                                scene,
                                &mut vello_scene,
                                Affine::scale(scale),
                            );
                            self.render_into_swapchain(&swapchain, &vello_scene);
                            swapchain
                        }
                    };

                    // The content is already at its physical position in the window-sized swapchain.
                    let visual = self.dcomp.CreateVisual().expect("raster visual");
                    visual.SetContent(&swapchain).expect("raster content");
                    parent.AddVisual(&visual, false, None).expect("add raster");

                    retained.insert(id, swapchain);
                }

                CompositedNode::Surface {
                    placement,
                    children,
                } => {
                    let visual = self.dcomp.CreateVisual().expect("surface visual");
                    apply_transform(&visual, physical(placement.transform, scale));
                    parent.AddVisual(&visual, false, None).expect("add surface");

                    fill_handles(&visual, placement, scale);

                    self.add_nodes(&visual, children, scale, retained);
                }

                CompositedNode::External {
                    size, transform: t, ..
                } => {
                    let swapchain = make_composition_swapchain(
                        &self.factory,
                        &self.raw_queue,
                        ((size.width.get() as f64 * scale) as u32).max(1),
                        ((size.height.get() as f64 * scale) as u32).max(1),
                    );
                    clear_swapchain(
                        &self.raw_device,
                        &self.raw_queue,
                        &swapchain,
                        [0.1, 0.7, 0.4, 1.0],
                    );

                    let visual = self.dcomp.CreateVisual().expect("external visual");
                    apply_transform(&visual, physical(*t, scale));
                    visual.SetContent(&swapchain).expect("external content");
                    parent
                        .AddVisual(&visual, false, None)
                        .expect("add external");
                }
            }
        }
    }

    /// Renders `scene` with vello and blits it into `swapchain`'s backbuffer, then presents it.
    unsafe fn render_into_swapchain(&mut self, swapchain: &IDXGISwapChain3, scene: &vello::Scene) {
        self.renderer
            .render_to_texture(
                &self.device,
                &self.queue,
                scene,
                &self.target_view,
                &RenderParams {
                    // Transparent so anything not drawn lets the visuals beneath show through.
                    base_color: Color::TRANSPARENT,
                    width: self.width,
                    height: self.height,
                    antialiasing_method: AaConfig::Area,
                },
            )
            .expect("vello render");

        let index = swapchain.GetCurrentBackBufferIndex();
        let backbuffer: ID3D12Resource = swapchain.GetBuffer(index).expect("backbuffer");
        let wrapped = wrap_backbuffer(&self.device, &backbuffer, self.width, self.height);
        let wrapped_view = wrapped.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        self.blitter
            .blit(&self.device, &mut encoder, &self.target_view, &wrapped_view);
        self.queue.submit([encoder.finish()]);

        // wgpu left the backbuffer in RENDER_TARGET after the blit; present from PRESENT (== COMMON).
        transition_and_present(
            &self.raw_device,
            &self.raw_queue,
            swapchain,
            &backbuffer,
            D3D12_RESOURCE_STATE_RENDER_TARGET,
        );
    }
}

/// A DirectComposition visual a layer drives off the scene. It holds the scale factor so a logical
/// transform poked from a layer is conjugated into the visual's physical space, matching how the
/// visual was placed.
struct DcompVisual {
    visual: IDCompositionVisual,
    scale: f64,
}

impl CompositorVisual for DcompVisual {
    fn set_transform(&self, transform: Affine) {
        unsafe {
            apply_transform(&self.visual, physical(transform, self.scale));
        }
    }

    fn set_opacity(&self, _opacity: f32) {
        // Opacity drive needs IDCompositionVisual3::SetOpacity; not yet wired.
    }
}

/// Fills `placement`'s handles with a visual-backed driver, so a layer moves `visual` off the scene.
unsafe fn fill_handles(visual: &IDCompositionVisual, placement: &SurfacePlacement, scale: f64) {
    if let Some(handle) = &placement.transform_handle {
        handle.surface().fill(Rc::new(DcompVisual {
            visual: visual.clone(),
            scale,
        }));
    }
    if let Some(handle) = &placement.opacity_handle {
        handle.surface().fill(Rc::new(DcompVisual {
            visual: visual.clone(),
            scale,
        }));
    }
}

/// A signature of a frame's visual-tree shape: node kinds in tree order, with each raster's scene
/// identity and each external's surface id. Two frames with equal signatures map to the same visual
/// tree, so only their surface transforms differ, which the filled handles drive.
fn structure_of(frame: &CompositedFrame) -> Vec<usize> {
    fn walk(frame: &CompositedFrame, out: &mut Vec<usize>) {
        for node in frame.nodes() {
            match node {
                CompositedNode::Raster { scene } => {
                    out.push(1);
                    out.push(Rc::as_ptr(scene) as usize);
                }
                CompositedNode::Surface { children, .. } => {
                    out.push(2);
                    walk(children, out);
                    out.push(0);
                }
                CompositedNode::External { surface, .. } => {
                    out.push(3);
                    out.push(surface.0 as usize);
                }
            }
        }
    }

    let mut out = Vec::new();
    walk(frame, &mut out);
    out
}

/// Conjugates a logical-pixel transform into physical-pixel space: a uniform scale leaves the linear
/// part unchanged and scales only the translation.
fn physical(transform: Affine, scale: f64) -> Affine {
    let [a, b, c, d, e, f] = transform.as_coeffs();
    Affine::new([a, b, c, d, e * scale, f * scale])
}

/// Applies `transform` to `visual` as its static 2D transform.
unsafe fn apply_transform(visual: &IDCompositionVisual, transform: Affine) {
    let [a, b, c, d, e, f] = transform.as_coeffs().map(|v| v as f32);
    let matrix = Matrix3x2 {
        M11: a,
        M12: b,
        M21: c,
        M22: d,
        M31: e,
        M32: f,
    };
    visual.SetTransform2(&matrix).expect("set transform");
}

/// Copies a straight-alpha texture into a render target, premultiplying as it goes, so vello's output
/// lands correctly in a premultiplied-alpha composition swapchain.
struct PremultBlit {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
}

impl PremultBlit {
    fn new(device: &wgpu::Device, target: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("premult blit"),
            source: wgpu::ShaderSource::Wgsl(PREMULT_WGSL.into()),
        });

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("premult blit"),
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
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("premult blit"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("premult blit"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());

        Self {
            pipeline,
            layout,
            sampler,
        }
    }

    fn blit(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        src: &wgpu::TextureView,
        dst: &wgpu::TextureView,
    ) {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("premult blit"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(src),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("premult blit"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: dst,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

const PREMULT_WGSL: &str = r"
@vertex
fn vs(@builtin(vertex_index) idx: u32) -> @builtin(position) vec4<f32> {
    var corners = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
    return vec4<f32>(corners[idx], 0.0, 1.0);
}

@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;

@fragment
fn fs(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = pos.xy / vec2<f32>(textureDimensions(tex));
    let c = textureSample(tex, samp, uv);
    return vec4<f32>(c.rgb * c.a, c.a);
}
";

/// Builds the window-sized vello render target and the blitter that copies it into a swapchain.
fn make_target(device: &wgpu::Device, width: u32, height: u32) -> (wgpu::TextureView, PremultBlit) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("vello target"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let blitter = PremultBlit::new(device, wgpu::TextureFormat::Bgra8Unorm);
    (view, blitter)
}

fn hwnd_of(window: &(impl HasWindowHandle + ?Sized)) -> HWND {
    match window.window_handle().unwrap().as_raw() {
        RawWindowHandle::Win32(handle) => HWND(handle.hwnd.get() as *mut c_void),
        other => panic!("expected a Win32 window handle, got {other:?}"),
    }
}

// ── proven COM helpers (from the dcomp_present spike) ──────────────────────────────────────────────

/// Creates a flip-model composition swapchain on the command `queue`.
unsafe fn make_composition_swapchain(
    factory: &IDXGIFactory2,
    queue: &ID3D12CommandQueue,
    width: u32,
    height: u32,
) -> IDXGISwapChain3 {
    let desc = DXGI_SWAP_CHAIN_DESC1 {
        Width: width,
        Height: height,
        Format: DXGI_FORMAT_B8G8R8A8_UNORM,
        Stereo: false.into(),
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
        BufferCount: 2,
        Scaling: DXGI_SCALING_STRETCH,
        SwapEffect: DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
        AlphaMode: DXGI_ALPHA_MODE_PREMULTIPLIED,
        Flags: 0,
    };
    factory
        .CreateSwapChainForComposition(queue, &desc, None)
        .expect("composition swapchain")
        .cast()
        .expect("swapchain3")
}

/// Wraps a swapchain backbuffer as a wgpu texture so wgpu can render into it.
unsafe fn wrap_backbuffer(
    device: &wgpu::Device,
    backbuffer: &ID3D12Resource,
    width: u32,
    height: u32,
) -> wgpu::Texture {
    let size = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let hal_texture = wgpu::hal::dx12::Device::texture_from_raw(
        backbuffer.clone(),
        wgpu::TextureFormat::Bgra8Unorm,
        wgpu::TextureDimension::D2,
        size,
        1,
        1,
    );
    device.create_texture_from_hal::<wgpu::hal::api::Dx12>(
        hal_texture,
        &wgpu::TextureDescriptor {
            label: Some("composition backbuffer"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        },
    )
}

/// Transitions the current backbuffer from `from` to PRESENT and presents.
unsafe fn transition_and_present(
    device: &ID3D12Device,
    queue: &ID3D12CommandQueue,
    swapchain: &IDXGISwapChain3,
    backbuffer: &ID3D12Resource,
    from: D3D12_RESOURCE_STATES,
) {
    let allocator: ID3D12CommandAllocator = device
        .CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT)
        .expect("allocator");
    let list: ID3D12GraphicsCommandList = device
        .CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, &allocator, None)
        .expect("command list");
    list.ResourceBarrier(&[transition(backbuffer, from, D3D12_RESOURCE_STATE_PRESENT)]);
    list.Close().expect("close list");
    execute_and_wait(device, queue, &list);

    swapchain.Present(1, DXGI_PRESENT(0)).ok().expect("present");
}

/// Clears a swapchain's backbuffer to `color` and presents it.
unsafe fn clear_swapchain(
    device: &ID3D12Device,
    queue: &ID3D12CommandQueue,
    swapchain: &IDXGISwapChain3,
    color: [f32; 4],
) {
    let index = swapchain.GetCurrentBackBufferIndex();
    let backbuffer: ID3D12Resource = swapchain.GetBuffer(index).expect("backbuffer");

    let heap: ID3D12DescriptorHeap = device
        .CreateDescriptorHeap(&D3D12_DESCRIPTOR_HEAP_DESC {
            Type: D3D12_DESCRIPTOR_HEAP_TYPE_RTV,
            NumDescriptors: 1,
            Flags: D3D12_DESCRIPTOR_HEAP_FLAG_NONE,
            NodeMask: 0,
        })
        .expect("rtv heap");
    let rtv = heap.GetCPUDescriptorHandleForHeapStart();
    device.CreateRenderTargetView(&backbuffer, None, rtv);

    let allocator: ID3D12CommandAllocator = device
        .CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT)
        .expect("allocator");
    let list: ID3D12GraphicsCommandList = device
        .CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, &allocator, None)
        .expect("command list");
    list.ResourceBarrier(&[transition(
        &backbuffer,
        D3D12_RESOURCE_STATE_PRESENT,
        D3D12_RESOURCE_STATE_RENDER_TARGET,
    )]);
    list.ClearRenderTargetView(rtv, &color, None);
    list.ResourceBarrier(&[transition(
        &backbuffer,
        D3D12_RESOURCE_STATE_RENDER_TARGET,
        D3D12_RESOURCE_STATE_PRESENT,
    )]);
    list.Close().expect("close list");
    execute_and_wait(device, queue, &list);

    swapchain.Present(1, DXGI_PRESENT(0)).ok().expect("present");
}

/// Executes one command list on the queue and blocks until the GPU finishes it.
unsafe fn execute_and_wait(
    device: &ID3D12Device,
    queue: &ID3D12CommandQueue,
    list: &ID3D12GraphicsCommandList,
) {
    let command_list: ID3D12CommandList = list.cast().expect("command list cast");
    queue.ExecuteCommandLists(&[Some(command_list)]);

    let fence: ID3D12Fence = device.CreateFence(0, D3D12_FENCE_FLAG_NONE).expect("fence");
    queue.Signal(&fence, 1).expect("signal");
    if fence.GetCompletedValue() < 1 {
        let event = CreateEventW(None, false, false, None).expect("event");
        fence.SetEventOnCompletion(1, event).expect("set event");
        WaitForSingleObject(event, INFINITE);
        CloseHandle(event).expect("close event");
    }
}

fn transition(
    resource: &ID3D12Resource,
    before: D3D12_RESOURCE_STATES,
    after: D3D12_RESOURCE_STATES,
) -> D3D12_RESOURCE_BARRIER {
    D3D12_RESOURCE_BARRIER {
        Type: D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
        Flags: D3D12_RESOURCE_BARRIER_FLAG_NONE,
        Anonymous: D3D12_RESOURCE_BARRIER_0 {
            Transition: ManuallyDrop::new(D3D12_RESOURCE_TRANSITION_BARRIER {
                pResource: ManuallyDrop::new(Some(resource.clone())),
                Subresource: D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
                StateBefore: before,
                StateAfter: after,
            }),
        },
    }
}
