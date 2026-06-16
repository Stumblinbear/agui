//! A [`WindowRenderer`] that composites an agui frame through `DirectComposition` (Windows).
//!
//! It walks the [`CompositedFrame`] tree into a `DirectComposition` visual tree: a
//! [`Surface`](CompositedNode::Surface) becomes a visual with its transform and opacity, with its
//! children nested beneath it, and its [`SurfacePlacement`] handles are filled with a visual-backed
//! [`CompositorVisual`] so a layer drives the visual off the scene; a [`Raster`](CompositedNode::Raster)
//! is rasterized by the wrapped [`TextureRenderer`] into a composition swapchain reused across frames by
//! scene identity, so an animating transform does not re-rasterize its content; an
//! [`External`](CompositedNode::External) becomes a sibling visual holding a placeholder swapchain in
//! lieu of a decoded surface.
//!
//! The compositor owns no rasterizer of its own. It hands each raster node to `R`, which renders into a
//! storage target the compositor then premultiplies into the node's swapchain, so any backend that
//! implements [`TextureRenderer`] on a `DirectX 12` device gains `DirectComposition` presentation.

use std::sync::Arc;
use std::{ffi::c_void, mem::ManuallyDrop, rc::Rc};

use agui_core::paint::command::PaintShape;
use agui_core::paint::compositing::{
    CompositedFrame, CompositedNode, CompositorVisual, SurfacePlacement,
};
use agui_core::paint::peniko::kurbo::Affine;
use agui_core::paint::scene::Scene;
use agui_window::WindowRenderer;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawWindowHandle};
use rustc_hash::FxHashMap;
use windows::Foundation::Numerics::Matrix3x2;
use windows::Win32::{
    Foundation::{CloseHandle, HWND},
    Graphics::{
        Direct2D::Common::D2D_RECT_F,
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
            DCompositionCreateDevice2, IDCompositionDesktopDevice, IDCompositionRectangleClip,
            IDCompositionTarget, IDCompositionVisual, IDCompositionVisual3,
        },
        Dxgi::{
            Common::{DXGI_ALPHA_MODE_PREMULTIPLIED, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC},
            CreateDXGIFactory2, DXGI_CREATE_FACTORY_FLAGS, DXGI_PRESENT, DXGI_SCALING_STRETCH,
            DXGI_SWAP_CHAIN_DESC1, DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
            DXGI_USAGE_RENDER_TARGET_OUTPUT, IDXGIFactory2, IDXGISwapChain3,
        },
    },
    System::Threading::{CreateEventW, INFINITE, WaitForSingleObject},
};
use windows::core::{IUnknown, Interface};

use crate::TextureRenderer;

/// Identifies a retained raster swapchain by the identity of the scene it holds, so an unchanged
/// scene reuses its swapchain rather than re-rasterizing.
type SceneId = *const ();

/// A [`WindowRenderer`] that composites a frame through `DirectComposition`, rasterizing each raster
/// node with the wrapped renderer `R`, whose device must be `DirectX 12`.
pub struct Dcomp<R> {
    renderer: R,
    inner: Option<Inner>,
}

struct Inner {
    device: wgpu::Device,
    queue: wgpu::Queue,
    raw_device: ID3D12Device,
    raw_queue: ID3D12CommandQueue,
    factory: IDXGIFactory2,

    dcomp: IDCompositionDesktopDevice,
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

impl<R> Dcomp<R> {
    pub fn new(renderer: R) -> Self {
        Self {
            renderer,
            inner: None,
        }
    }
}

impl<R: TextureRenderer> WindowRenderer for Dcomp<R> {
    fn attach<W>(&mut self, window: Arc<W>, width: u32, height: u32)
    where
        W: HasWindowHandle + HasDisplayHandle + Send + Sync + 'static,
    {
        let hwnd = hwnd_of(&*window);
        self.inner = Some(Inner::new(
            &self.renderer,
            hwnd,
            width.max(1),
            height.max(1),
        ));
    }

    fn resize(&mut self, width: u32, height: u32) {
        if let Some(inner) = self.inner.as_mut() {
            inner.resize(width.max(1), height.max(1));
        }
    }

    fn present(&mut self, frame: &CompositedFrame, scale_factor: f64) {
        if let Some(inner) = self.inner.as_mut() {
            inner.present(&mut self.renderer, frame, scale_factor);
        }
    }
}

impl Inner {
    #[allow(clippy::undocumented_unsafe_blocks)]
    fn new<R: TextureRenderer>(renderer: &R, hwnd: HWND, width: u32, height: u32) -> Self {
        let device = renderer.device().clone();
        let queue = renderer.queue().clone();

        let raw_device = unsafe {
            device
                .as_hal::<wgpu::hal::api::Dx12>()
                .expect("Dcomp requires a DX12-backed TextureRenderer")
                .raw_device()
                .clone()
        };
        let raw_queue = unsafe {
            queue
                .as_hal::<wgpu::hal::api::Dx12>()
                .expect("Dcomp requires a DX12-backed TextureRenderer")
                .as_raw()
                .clone()
        };

        let factory: IDXGIFactory2 =
            unsafe { CreateDXGIFactory2(DXGI_CREATE_FACTORY_FLAGS(0)).expect("dxgi factory") };
        let dcomp: IDCompositionDesktopDevice =
            unsafe { DCompositionCreateDevice2(None::<&IUnknown>).expect("dcomp device") };
        let target = unsafe { dcomp.CreateTargetForHwnd(hwnd, true).expect("dcomp target") };
        let root: IDCompositionVisual = unsafe {
            dcomp
                .CreateVisual()
                .expect("root visual")
                .cast()
                .expect("root visual cast")
        };

        unsafe { target.SetRoot(&root).expect("set root") };

        let (target_view, blitter) = make_target(&device, width, height);

        Self {
            device,
            queue,
            raw_device,
            raw_queue,
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

    fn resize(&mut self, width: u32, height: u32) {
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

    #[allow(clippy::undocumented_unsafe_blocks)]
    fn present<R: TextureRenderer>(
        &mut self,
        renderer: &mut R,
        frame: &CompositedFrame,
        scale_factor: f64,
    ) {
        // When the structure is unchanged the visuals already exist and the off-thread pokes have set
        // their transforms, so commit those without rebuilding.
        let structure = structure_of(frame);

        if structure == self.structure {
            unsafe { self.dcomp.Commit().expect("commit") };

            return;
        }

        // Rebuild the visual tree, reusing a scene's retained swapchain so unchanged content is not
        // re-rasterized. Swapchains not used this frame are dropped at the end.
        unsafe { self.root.RemoveAllVisuals().expect("clear root") };

        let mut retained: FxHashMap<SceneId, IDXGISwapChain3> = FxHashMap::default();

        let root = self.root.clone();

        self.add_nodes(renderer, &root, frame, scale_factor, &mut retained);

        self.swapchains = retained;
        self.structure = structure;

        unsafe { self.dcomp.Commit().expect("commit") };
    }

    /// Adds `frame`'s nodes as children of `parent`, in order. Rasterized content is rendered at
    /// physical resolution (scaled by `scale`); a surface's transform is conjugated into physical
    /// space and placed on its visual, which `DirectComposition` composes beneath `parent`.
    #[allow(clippy::undocumented_unsafe_blocks)]
    fn add_nodes<R: TextureRenderer>(
        &mut self,
        renderer: &mut R,
        parent: &IDCompositionVisual,
        frame: &CompositedFrame,
        scale: f64,
        retained: &mut FxHashMap<SceneId, IDXGISwapChain3>,
    ) {
        for node in frame.nodes() {
            match node {
                CompositedNode::Raster { scene } => {
                    let id = Rc::as_ptr(scene).cast::<()>();

                    let swapchain = if let Some(swapchain) = self.swapchains.remove(&id) {
                        swapchain
                    } else {
                        let swapchain = make_composition_swapchain(
                            &self.factory,
                            &self.raw_queue,
                            self.width,
                            self.height,
                        );
                        self.render_into_swapchain(renderer, &swapchain, scene, scale);
                        swapchain
                    };

                    // The content is already at its physical position in the window-sized swapchain.
                    let visual: IDCompositionVisual = unsafe {
                        self.dcomp
                            .CreateVisual()
                            .expect("raster visual")
                            .cast()
                            .expect("raster visual cast")
                    };

                    unsafe { visual.SetContent(&swapchain).expect("raster content") };
                    unsafe { parent.AddVisual(&visual, false, None).expect("add raster") };

                    retained.insert(id, swapchain);
                }

                CompositedNode::Surface {
                    placement,
                    children,
                } => {
                    let visual: IDCompositionVisual = unsafe {
                        self.dcomp
                            .CreateVisual()
                            .expect("surface visual")
                            .cast()
                            .expect("surface visual cast")
                    };

                    apply_placement(&self.dcomp, &visual, placement, scale);

                    unsafe { parent.AddVisual(&visual, false, None).expect("add surface") };

                    fill_handles(&visual, placement, scale);

                    self.add_nodes(renderer, &visual, children, scale, retained);
                }

                CompositedNode::External {
                    size, transform: t, ..
                } => {
                    let swapchain = make_composition_swapchain(
                        &self.factory,
                        &self.raw_queue,
                        as_u32(f64::from(size.width.get()) * scale).max(1),
                        as_u32(f64::from(size.height.get()) * scale).max(1),
                    );

                    clear_swapchain(
                        &self.raw_device,
                        &self.raw_queue,
                        &swapchain,
                        [0.1, 0.7, 0.4, 1.0],
                    );

                    let visual: IDCompositionVisual = unsafe {
                        self.dcomp
                            .CreateVisual()
                            .expect("external visual")
                            .cast()
                            .expect("external visual cast")
                    };

                    apply_transform(&visual, physical(*t, scale));

                    unsafe { visual.SetContent(&swapchain).expect("external content") };

                    unsafe {
                        parent
                            .AddVisual(&visual, false, None)
                            .expect("add external");
                    };
                }
            }
        }
    }

    /// Rasterizes one raster node's `scene` with the renderer into the shared target, blits it into
    /// `swapchain`'s backbuffer premultiplied, then presents it.
    #[allow(clippy::undocumented_unsafe_blocks)]
    fn render_into_swapchain<R: TextureRenderer>(
        &mut self,
        renderer: &mut R,
        swapchain: &IDXGISwapChain3,
        scene: &Rc<Scene>,
        scale: f64,
    ) {
        renderer.render(scene, &self.target_view, self.width, self.height, scale);

        let index = unsafe { swapchain.GetCurrentBackBufferIndex() };
        let backbuffer: ID3D12Resource = unsafe { swapchain.GetBuffer(index).expect("backbuffer") };
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

/// A `DirectComposition` visual a layer drives off the scene. It holds the scale factor so a logical
/// transform poked from a layer is conjugated into the visual's physical space, matching how the
/// visual was placed.
struct DcompVisual {
    visual: IDCompositionVisual,
    scale: f64,
}

impl CompositorVisual for DcompVisual {
    fn set_transform(&self, transform: Affine) {
        apply_transform(&self.visual, physical(transform, self.scale));
    }

    #[allow(clippy::undocumented_unsafe_blocks)]
    fn set_opacity(&self, opacity: f32) {
        unsafe {
            let visual: IDCompositionVisual3 = self.visual.cast().expect("a v3 visual for opacity");
            visual.SetOpacity2(opacity).expect("set opacity");
        }
    }
}

/// Fills `placement`'s handles with a visual-backed driver, so a layer moves `visual` off the scene.
fn fill_handles(visual: &IDCompositionVisual, placement: &SurfacePlacement, scale: f64) {
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
                    #[allow(clippy::cast_possible_truncation)]
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
    let coeffs = transform.as_coeffs();

    Affine::new([
        coeffs[0],
        coeffs[1],
        coeffs[2],
        coeffs[3],
        coeffs[4] * scale,
        coeffs[5] * scale,
    ])
}

/// Applies `transform` to `visual` as its static 2D transform.
#[allow(clippy::undocumented_unsafe_blocks)]
fn apply_transform(visual: &IDCompositionVisual, transform: Affine) {
    let coeffs = transform.as_coeffs().map(as_f32);

    let matrix = Matrix3x2 {
        M11: coeffs[0],
        M12: coeffs[1],
        M21: coeffs[2],
        M22: coeffs[3],
        M31: coeffs[4],
        M32: coeffs[5],
    };

    unsafe {
        visual
            .SetTransform2(std::ptr::from_ref(&matrix))
            .expect("set transform");
    }
}

/// Applies a surface placement to its visual: transform, opacity, and clip. A rect, rounded-rect, or
/// circle clip maps onto a visual; an arbitrary path cannot, so the compositor bakes those when the
/// subtree is rasterizable and one over a system surface reaches here and is left unclipped.
#[allow(clippy::undocumented_unsafe_blocks)]
fn apply_placement(
    dcomp: &IDCompositionDesktopDevice,
    visual: &IDCompositionVisual,
    placement: &SurfacePlacement,
    scale: f64,
) {
    apply_transform(visual, physical(placement.transform, scale));

    if placement.opacity < 1.0 {
        let visual: IDCompositionVisual3 = visual.cast().expect("a v3 visual for opacity");
        unsafe { visual.SetOpacity2(placement.opacity).expect("set opacity") };
    }

    match &placement.clip {
        Some(PaintShape::Rect(rect)) => {
            let clip = D2D_RECT_F {
                left: as_f32(rect.x0 * scale),
                top: as_f32(rect.y0 * scale),
                right: as_f32(rect.x1 * scale),
                bottom: as_f32(rect.y1 * scale),
            };

            unsafe {
                visual
                    .SetClip2(std::ptr::from_ref(&clip))
                    .expect("set clip");
            }
        }

        Some(PaintShape::RoundedRect(rounded)) => {
            let rect = rounded.rect();
            let radii = rounded.radii();
            let clip = rounded_clip(
                dcomp,
                [rect.x0, rect.y0, rect.x1, rect.y1],
                [
                    radii.top_left,
                    radii.top_right,
                    radii.bottom_right,
                    radii.bottom_left,
                ],
                scale,
            );

            unsafe { visual.SetClip(&clip).expect("set clip") };
        }

        Some(PaintShape::Circle(circle)) => {
            let (c, r) = (circle.center, circle.radius);

            let clip = rounded_clip(
                dcomp,
                [c.x - r, c.y - r, c.x + r, c.y + r],
                [r, r, r, r],
                scale,
            );

            unsafe { visual.SetClip(&clip).expect("set clip") };
        }

        Some(PaintShape::Path(_)) | None => {}
    }
}

/// Builds a rounded-rectangle clip from `edges` (left, top, right, bottom) and per-corner `radii`
/// (top-left, top-right, bottom-right, bottom-left), scaled to physical pixels.
#[allow(clippy::undocumented_unsafe_blocks)]
fn rounded_clip(
    dcomp: &IDCompositionDesktopDevice,
    edges: [f64; 4],
    radii: [f64; 4],
    scale: f64,
) -> IDCompositionRectangleClip {
    let [left, top, right, bottom] = edges.map(|edge| as_f32(edge * scale));
    let [tl, tr, br, bl] = radii.map(|radius| as_f32(radius * scale));

    unsafe {
        let clip = dcomp.CreateRectangleClip().expect("rectangle clip");
        clip.SetLeft2(left).expect("left");
        clip.SetTop2(top).expect("top");
        clip.SetRight2(right).expect("right");
        clip.SetBottom2(bottom).expect("bottom");
        clip.SetTopLeftRadiusX2(tl).expect("tl x");
        clip.SetTopLeftRadiusY2(tl).expect("tl y");
        clip.SetTopRightRadiusX2(tr).expect("tr x");
        clip.SetTopRightRadiusY2(tr).expect("tr y");
        clip.SetBottomRightRadiusX2(br).expect("br x");
        clip.SetBottomRightRadiusY2(br).expect("br y");
        clip.SetBottomLeftRadiusX2(bl).expect("bl x");
        clip.SetBottomLeftRadiusY2(bl).expect("bl y");
        clip
    }
}

/// Copies a straight-alpha texture into a render target, premultiplying as it goes, so the renderer's
/// output lands correctly in a premultiplied-alpha composition swapchain.
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

/// Builds the window-sized render target and the blitter that copies it into a swapchain.
fn make_target(device: &wgpu::Device, width: u32, height: u32) -> (wgpu::TextureView, PremultBlit) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("dcomp target"),
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

/// Narrows a logical-pixel coordinate to the `f32` the Windows graphics APIs expect.
#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
fn as_f32(value: f64) -> f32 {
    value as f32
}

/// Narrows a physical-pixel extent to `u32`.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn as_u32(value: f64) -> u32 {
    value as u32
}

fn hwnd_of(window: &(impl HasWindowHandle + ?Sized)) -> HWND {
    match window.window_handle().unwrap().as_raw() {
        RawWindowHandle::Win32(handle) => HWND(handle.hwnd.get() as *mut c_void),
        other => panic!("expected a Win32 window handle, got {other:?}"),
    }
}

/// Creates a flip-model composition swapchain on the command `queue`.
#[allow(clippy::undocumented_unsafe_blocks)]
fn make_composition_swapchain(
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

    unsafe {
        factory
            .CreateSwapChainForComposition(queue, std::ptr::from_ref(&desc), None)
            .expect("composition swapchain")
            .cast()
            .expect("swapchain3")
    }
}

/// Wraps a swapchain backbuffer as a wgpu texture so wgpu can render into it.
#[allow(clippy::undocumented_unsafe_blocks)]
fn wrap_backbuffer(
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

    let hal_texture = unsafe {
        wgpu::hal::dx12::Device::texture_from_raw(
            backbuffer.clone(),
            wgpu::TextureFormat::Bgra8Unorm,
            wgpu::TextureDimension::D2,
            size,
            1,
            1,
        )
    };

    unsafe {
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
}

/// Transitions the current backbuffer from `from` to PRESENT and presents.
#[allow(clippy::undocumented_unsafe_blocks)]
fn transition_and_present(
    device: &ID3D12Device,
    queue: &ID3D12CommandQueue,
    swapchain: &IDXGISwapChain3,
    backbuffer: &ID3D12Resource,
    from: D3D12_RESOURCE_STATES,
) {
    unsafe {
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
}

/// Clears a swapchain's backbuffer to `color` and presents it.
#[allow(clippy::undocumented_unsafe_blocks)]
fn clear_swapchain(
    device: &ID3D12Device,
    queue: &ID3D12CommandQueue,
    swapchain: &IDXGISwapChain3,
    color: [f32; 4],
) {
    unsafe {
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
}

/// Executes one command list on the queue and blocks until the GPU finishes it.
#[allow(clippy::undocumented_unsafe_blocks)]
fn execute_and_wait(
    device: &ID3D12Device,
    queue: &ID3D12CommandQueue,
    list: &ID3D12GraphicsCommandList,
) {
    unsafe {
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
