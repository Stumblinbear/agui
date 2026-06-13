use std::{
    cell::{Ref, RefCell, RefMut},
    rc::Rc,
};

use peniko::{BlendMode, Compose, Mix, kurbo::Affine};

use crate::{
    geometry::{Offset, Size},
    paint::{
        command::{PaintCommand, PaintShape},
        scene::Scene,
    },
};

/// A shared, mutable handle to a layer.
pub struct LayerHandle<L: ?Sized = dyn Layer>(Rc<RefCell<L>>);

impl<L: Layer + 'static> LayerHandle<L> {
    pub fn new(layer: L) -> Self {
        Self(Rc::new(RefCell::new(layer)))
    }
}

impl<L: ?Sized> LayerHandle<L> {
    pub fn borrow(&self) -> Ref<'_, L> {
        self.0.borrow()
    }

    pub fn borrow_mut(&self) -> RefMut<'_, L> {
        self.0.borrow_mut()
    }
}

impl<L: ?Sized> Clone for LayerHandle<L> {
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}

impl<L: ?Sized> PartialEq for LayerHandle<L> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl<L: ?Sized> Eq for LayerHandle<L> {}

impl<L: Layer + 'static> From<LayerHandle<L>> for LayerHandle {
    fn from(val: LayerHandle<L>) -> Self {
        LayerHandle(val.0)
    }
}

/// Identifies a surface the system compositor owns and agui does not rasterize, such as a video
/// frame or an embedded view.
///
/// A driver mints one for a surface it manages and hands it to the widget that places the surface;
/// the [`External`](CompositedNode::External) node of a composed frame reports where that surface
/// belongs so the driver can position the matching system visual.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ExternalSurfaceId(pub u64);

/// A system-composited visual a driver places a surface on.
///
/// A layer drives one of these to move or fade its surface without re-rasterizing, instead of baking
/// the change into a scene. A driver that places the surface on its own visual implements this.
pub trait CompositorVisual {
    /// Replaces the transform applied to the visual.
    fn set_transform(&self, transform: Affine);

    /// Replaces the opacity applied to the visual.
    fn set_opacity(&self, opacity: f32);
}

/// The driver-owned visual a layer's surface was placed on, or detached when none was.
#[derive(Clone)]
pub struct SurfaceHandle(Rc<RefCell<Option<Rc<dyn CompositorVisual>>>>);

impl SurfaceHandle {
    /// Mints a handle with no visual bound yet.
    fn detached() -> Self {
        Self(Rc::new(RefCell::new(None)))
    }

    /// Whether no visual is bound, so a layer must recomposite rather than drive a visual.
    fn is_detached(&self) -> bool {
        self.0.borrow().is_none()
    }

    fn visual(&self) -> Option<Rc<dyn CompositorVisual>> {
        self.0.borrow().clone()
    }

    /// Binds the handle to the visual a driver made for it.
    pub fn fill(&self, visual: Rc<dyn CompositorVisual>) {
        *self.0.borrow_mut() = Some(visual);
    }
}

impl std::fmt::Debug for SurfaceHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SurfaceHandle")
            .field("attached", &!self.is_detached())
            .finish()
    }
}

/// A layer's handle to drive only the transform of its system-composited surface.
#[derive(Clone, Debug)]
pub struct SurfaceTransformHandle(SurfaceHandle);

impl Default for SurfaceTransformHandle {
    fn default() -> Self {
        Self::new()
    }
}

impl SurfaceTransformHandle {
    pub fn new() -> Self {
        Self(SurfaceHandle::detached())
    }

    /// The binding to hand [`Compositor::push_surface`], which a driver fills with a visual.
    pub fn surface(&self) -> SurfaceHandle {
        self.0.clone()
    }

    /// Drives the transform on the bound visual, returning whether one was bound. `false` means no
    /// driver placed the surface, so the caller must recomposite instead.
    pub fn set_transform(&self, transform: Affine) -> bool {
        match self.0.visual() {
            Some(visual) => {
                visual.set_transform(transform);
                true
            }
            None => false,
        }
    }
}

/// A layer's handle to drive only the opacity of its system-composited surface.
#[derive(Clone, Debug)]
pub struct SurfaceOpacityHandle(SurfaceHandle);

impl Default for SurfaceOpacityHandle {
    fn default() -> Self {
        Self::new()
    }
}

impl SurfaceOpacityHandle {
    pub fn new() -> Self {
        Self(SurfaceHandle::detached())
    }

    /// The binding to hand [`Compositor::push_surface`], which a driver fills with a visual.
    pub fn surface(&self) -> SurfaceHandle {
        self.0.clone()
    }

    /// Drives the opacity on the bound visual, returning whether one was bound. `false` means no
    /// driver placed the surface, so the caller must recomposite instead.
    pub fn set_opacity(&self, opacity: f32) -> bool {
        match self.0.visual() {
            Some(visual) => {
                visual.set_opacity(opacity);
                true
            }
            None => false,
        }
    }
}

/// The transform, opacity, and clip a [`Surface`](CompositedNode::Surface) node applies on its own
/// system visual, with the handles a driver fills to drive them off the scene.
///
/// The `transform` and `opacity` are the values in effect now: a driver reads them to position the
/// visual, and a presenter that does not composite applies them when it rasterizes the node. Each
/// handle, once a driver fills it, lets the owning layer change its value on the visual without
/// re-rasterizing; a handle is absent when no layer drives that capability.
#[derive(Clone, Debug)]
pub struct SurfacePlacement {
    /// The transform applied to the visual, relative to its parent.
    pub transform: Affine,

    /// The opacity applied to the visual.
    pub opacity: f32,

    /// The clip applied to the visual, in its own coordinate space, if any.
    pub clip: Option<PaintShape>,

    /// The handle a layer drives the transform through, once a driver fills it.
    pub transform_handle: Option<SurfaceTransformHandle>,

    /// The handle a layer drives the opacity through, once a driver fills it.
    pub opacity_handle: Option<SurfaceOpacityHandle>,
}

/// One node of a [`CompositedFrame`].
///
/// A leaf is rasterized drawing or a placement for a system-owned surface; a [`Surface`](Self::Surface)
/// is an inner node that gives its children their own system visual, so its transform and opacity
/// apply to them through the compositor rather than being baked into a scene.
#[derive(Clone, Debug)]
pub enum CompositedNode {
    /// Rasterized drawing, positioned by the transforms of its ancestor [`Surface`](Self::Surface)
    /// nodes.
    Raster { scene: Rc<Scene> },

    /// A placement for a surface the system compositor owns. A backend positions that surface at
    /// `transform` (relative to its parent), sized to `size`, blended at `opacity`, and clipped
    /// to `clip` if present.
    External {
        surface: ExternalSurfaceId,
        size: Size,
        transform: Affine,
        opacity: f32,
        clip: Option<PaintShape>,
    },

    /// An inner node whose `children` are placed on their own system visual, which `placement`
    /// positions, fades, and clips. The visual composes its children's transforms beneath it, so
    /// moving it moves them as one without re-rasterizing.
    Surface {
        placement: SurfacePlacement,
        children: CompositedFrame,
    },
}

/// The result of composing a layer tree: a back-to-front tree of nodes to present.
#[derive(Clone, Debug, Default)]
pub struct CompositedFrame {
    nodes: Vec<CompositedNode>,
}

impl CompositedFrame {
    pub fn new() -> Self {
        Self::default()
    }

    /// The top-level nodes to present, back-to-front.
    pub fn nodes(&self) -> &[CompositedNode] {
        &self.nodes
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Rasterizes this frame to a single [`Scene`], composing every node's transform and opacity down
    /// the tree. A presenter without a system compositor uses this; it panics on an
    /// [`External`](CompositedNode::External) node, which has no rasterization.
    pub fn rasterize(&self) -> Scene {
        let mut scene = Scene::new();
        self.render_into(&mut scene, Affine::IDENTITY);
        scene.flatten()
    }

    fn render_into(&self, out: &mut Scene, transform: Affine) {
        for node in &self.nodes {
            match node {
                CompositedNode::Raster { scene } => {
                    let shifted = transform != Affine::IDENTITY;

                    if shifted {
                        out.push(PaintCommand::PushTransform(transform));
                    }

                    out.push(PaintCommand::Embed {
                        scene: Rc::clone(scene),
                    });

                    if shifted {
                        out.push(PaintCommand::PopTransform);
                    }
                }

                CompositedNode::Surface {
                    placement,
                    children,
                } => {
                    let inner = transform * placement.transform;
                    let layered = placement.opacity < 1.0 || placement.clip.is_some();

                    if layered {
                        let shifted = inner != Affine::IDENTITY;

                        if shifted {
                            out.push(PaintCommand::PushTransform(inner));
                        }

                        {
                            out.push(PaintCommand::PushLayer {
                                blend: BlendMode::new(Mix::Normal, Compose::SrcOver),
                                alpha: placement.opacity,
                                clip: placement.clip.clone().unwrap_or(PaintShape::Rect(
                                    peniko::kurbo::Rect::new(-1e9, -1e9, 1e9, 1e9),
                                )),
                            });
                            {
                                children.render_into(out, Affine::IDENTITY);
                            }
                            out.push(PaintCommand::PopLayer);
                        }

                        if shifted {
                            out.push(PaintCommand::PopTransform);
                        }
                    } else {
                        children.render_into(out, inner);
                    }
                }

                CompositedNode::External { .. } => {
                    debug_assert!(false, "rasterize cannot place a system-owned surface");
                }
            }
        }
    }
}

/// Builds a [`CompositedFrame`] from a layer tree.
pub struct Compositor {
    nodes: Vec<CompositedNode>,
    run: Scene,
    run_has_content: bool,
}

impl Compositor {
    fn new() -> Self {
        Self {
            nodes: Vec::new(),
            run: Scene::new(),
            run_has_content: false,
        }
    }

    /// Composes `root` and its descendants into the frame to present.
    pub fn compose<L: Layer + ?Sized>(root: &LayerHandle<L>) -> CompositedFrame {
        let mut root = root.borrow_mut();

        // Settle every dirty flag before composing; compose reads them to decide cache reuse.
        root.update_dirty();

        let mut compositor = Compositor::new();
        root.compose(&mut compositor);
        compositor.finish()
    }

    /// Pushes rasterized drawing into the current run, to be drawn over what precedes it.
    pub fn push_raster(&mut self, scene: Rc<Scene>) {
        self.run.push(PaintCommand::Embed { scene });
        self.run_has_content = true;
    }

    /// Pushes a system-owned surface on its own node, flushing the run so drawing after it stacks
    /// above it.
    pub fn push_external(
        &mut self,
        surface: ExternalSurfaceId,
        size: Size,
        transform: Affine,
        opacity: f32,
        clip: Option<PaintShape>,
    ) {
        self.flush_run();

        self.nodes.push(CompositedNode::External {
            surface,
            size,
            transform,
            opacity,
            clip,
        });
    }

    /// Pushes `children` on their own system visual that `placement` positions, fades, and clips,
    /// flushing the run.
    pub fn push_surface(&mut self, placement: SurfacePlacement, children: CompositedFrame) {
        if children.is_empty() {
            return;
        }

        self.flush_run();

        self.nodes.push(CompositedNode::Surface {
            placement,
            children,
        });
    }

    /// Pushes `children` under `transform`: at the identity they pass through unchanged; otherwise the
    /// transform is baked into the run when the subtree is one rasterized node, or applied on a visual
    /// the children inherit.
    pub fn push_transform(&mut self, transform: Affine, children: &CompositedFrame) {
        self.place_group(transform, 1.0, None, children);
    }

    /// Pushes `children` under `offset`, `opacity`, and `clip`: with no effect they pass through;
    /// otherwise the effect is baked into the run when the subtree is one rasterized node, or applied
    /// on a visual the children inherit.
    pub fn push_opacity(
        &mut self,
        offset: Offset,
        opacity: f32,
        clip: Option<PaintShape>,
        children: &CompositedFrame,
    ) {
        self.place_group(Affine::translate(offset), opacity, clip, children);
    }

    /// Concatenates `frame`'s nodes into the build: rasterized nodes merge into the current run, and
    /// every other node flushes the run and stands on its own, so drawing after it stacks above it.
    pub fn splice(&mut self, frame: &CompositedFrame) {
        for node in &frame.nodes {
            match node {
                CompositedNode::Raster { scene } => self.push_raster(Rc::clone(scene)),

                other => {
                    self.flush_run();
                    self.nodes.push(other.clone());
                }
            }
        }
    }

    /// Places `children` under `transform`, `opacity`, and `clip`: passes them through when the effect
    /// is trivial, bakes the effect into the run when the subtree is one rasterized node, and otherwise
    /// gives them a visual a system-composited child inherits.
    fn place_group(
        &mut self,
        transform: Affine,
        opacity: f32,
        clip: Option<PaintShape>,
        children: &CompositedFrame,
    ) {
        if children.is_empty() {
            return;
        }

        let trivial = transform == Affine::IDENTITY && opacity >= 1.0 && clip.is_none();

        if trivial {
            self.splice(children);

            return;
        }

        match children.nodes() {
            [CompositedNode::Raster { scene }] => {
                let layered = opacity < 1.0 || clip.is_some();

                self.run.push(PaintCommand::PushTransform(transform));
                {
                    if layered {
                        self.run.push(PaintCommand::PushLayer {
                            blend: BlendMode::new(Mix::Normal, Compose::SrcOver),
                            alpha: opacity,
                            clip: clip.unwrap_or(PaintShape::Rect(peniko::kurbo::Rect::new(
                                -1e9, -1e9, 1e9, 1e9,
                            ))),
                        });
                    }

                    self.run.push(PaintCommand::Embed {
                        scene: Rc::clone(scene),
                    });

                    if layered {
                        self.run.push(PaintCommand::PopLayer);
                    }
                }
                self.run.push(PaintCommand::PopTransform);

                self.run_has_content = true;
            }

            _ => {
                self.push_surface(
                    SurfacePlacement {
                        transform,
                        opacity,
                        clip,
                        transform_handle: None,
                        opacity_handle: None,
                    },
                    children.clone(),
                );
            }
        }
    }

    /// Flushes the accumulated run, if any, to a [`Raster`](CompositedNode::Raster) node.
    fn flush_run(&mut self) {
        if self.run_has_content {
            let scene = std::mem::replace(&mut self.run, Scene::new());

            self.nodes.push(CompositedNode::Raster {
                scene: Rc::new(scene),
            });

            self.run_has_content = false;
        }
    }

    pub fn finish(mut self) -> CompositedFrame {
        self.flush_run();

        CompositedFrame { nodes: self.nodes }
    }
}

/// A node in the retained compositing tree.
///
/// Composing a layer contributes its content to the frame under construction.
pub trait Layer {
    /// Contributes this layer and its descendants to the frame `compositor` is building.
    fn compose(&mut self, compositor: &mut Compositor);

    /// Settles this layer's dirty state and returns whether it or anything beneath it changed.
    fn update_dirty(&mut self) -> bool;
}

/// A layer that holds an ordered sequence of child layers.
pub trait ContainerLayer: Layer {
    /// Adds `child` after the existing children.
    fn append(&mut self, child: LayerHandle);
}

/// A layer whose content its parent positions when contributing it.
///
/// The offset survives the layer's own repaints, so a retained layer keeps its place until the
/// parent contributes it somewhere else.
pub trait PositionedLayer: Layer {
    /// Repositions this layer's content relative to its parent.
    fn set_offset(&mut self, offset: Offset);
}

/// An ordered set of child layers, caching the frame they compose to.
pub struct ChildLayers {
    children: Vec<LayerHandle>,
    dirty: bool,
    cache: Option<Rc<CompositedFrame>>,
}

impl Default for ChildLayers {
    fn default() -> Self {
        Self::new()
    }
}

impl ChildLayers {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            dirty: true,
            cache: None,
        }
    }

    pub fn append(&mut self, child: LayerHandle) {
        self.children.push(child);
        self.dirty = true;
        self.cache = None;
    }

    /// Removes every child and the cached composition.
    pub fn clear(&mut self) {
        self.children.clear();
        self.dirty = true;
        self.cache = None;
    }

    /// Settles the children's dirty state, folding it into this set's, and returns whether anything
    /// changed.
    pub fn update_dirty(&mut self) -> bool {
        let mut dirty = self.dirty;

        for child in &self.children {
            dirty |= child.borrow_mut().update_dirty();
        }

        self.dirty = dirty;
        self.dirty
    }

    /// Composes the children to their frame, reusing the cached one when nothing changed. The parent
    /// then places that frame under its own transform, so the cache survives a transform change.
    pub fn compose_frame(&mut self) -> Rc<CompositedFrame> {
        if !self.dirty
            && let Some(cache) = &self.cache
        {
            return Rc::clone(cache);
        }

        let mut sub = Compositor::new();

        for child in &self.children {
            child.borrow_mut().compose(&mut sub);
        }

        let frame = Rc::new(sub.finish());
        self.cache = Some(Rc::clone(&frame));
        self.dirty = false;
        frame
    }
}

/// A layer that applies a transform to its children.
pub struct TransformLayer {
    transform: Affine,
    offset: Offset,
    dirty: bool,
    children: ChildLayers,
}

impl TransformLayer {
    pub fn new(transform: Affine) -> Self {
        Self {
            transform,
            offset: Offset::ZERO,
            dirty: true,
            children: ChildLayers::new(),
        }
    }

    /// Replaces the transform applied to the children.
    pub fn set_transform(&mut self, transform: Affine) {
        if self.transform != transform {
            self.transform = transform;
            self.dirty = true;
        }
    }

    /// The transform currently applied to the children.
    pub fn transform(&self) -> Affine {
        self.transform
    }

    /// Removes every child, keeping the transform and offset. The parent repaints the children in.
    pub fn clear(&mut self) {
        self.children.clear();
        self.dirty = true;
    }
}

impl Layer for TransformLayer {
    fn compose(&mut self, compositor: &mut Compositor) {
        compositor.push_transform(
            Affine::translate(self.offset) * self.transform,
            &self.children.compose_frame(),
        );

        self.dirty = false;
    }

    fn update_dirty(&mut self) -> bool {
        self.dirty = self.children.update_dirty() || self.dirty;
        self.dirty
    }
}

impl ContainerLayer for TransformLayer {
    fn append(&mut self, child: LayerHandle) {
        self.children.append(child);
        self.dirty = true;
    }
}

impl PositionedLayer for TransformLayer {
    fn set_offset(&mut self, offset: Offset) {
        if self.offset != offset {
            self.offset = offset;
            self.dirty = true;
        }
    }
}

/// A layer that places its children on a system-composited surface and holds its transform on that
/// surface's visual, so an animation can move it without re-rasterizing the children.
pub struct SurfaceTransformLayer {
    transform: Affine,
    offset: Offset,
    handle: SurfaceTransformHandle,
    dirty: bool,
    children: ChildLayers,
}

impl SurfaceTransformLayer {
    pub fn new(transform: Affine) -> Self {
        Self {
            transform,
            offset: Offset::ZERO,
            handle: SurfaceTransformHandle::new(),
            dirty: true,
            children: ChildLayers::new(),
        }
    }

    /// Drives the transform: pokes the bound visual and returns `true` if a driver placed the surface,
    /// otherwise records that the layer must recomposite and returns `false`.
    pub fn set_transform(&mut self, transform: Affine) -> bool {
        self.transform = transform;

        if self
            .handle
            .set_transform(Affine::translate(self.offset) * transform)
        {
            true
        } else {
            self.dirty = true;
            false
        }
    }

    /// Removes every child, keeping the transform and offset. The parent repaints the children in.
    pub fn clear(&mut self) {
        self.children.clear();
        self.dirty = true;
    }
}

impl Layer for SurfaceTransformLayer {
    fn compose(&mut self, compositor: &mut Compositor) {
        let children = self.children.compose_frame();
        compositor.push_surface(
            SurfacePlacement {
                transform: Affine::translate(self.offset) * self.transform,
                opacity: 1.0,
                clip: None,
                transform_handle: Some(self.handle.clone()),
                opacity_handle: None,
            },
            (*children).clone(),
        );

        self.dirty = false;
    }

    fn update_dirty(&mut self) -> bool {
        self.dirty = self.children.update_dirty() || self.dirty;
        self.dirty
    }
}

impl ContainerLayer for SurfaceTransformLayer {
    fn append(&mut self, child: LayerHandle) {
        self.children.append(child);
        self.dirty = true;
    }
}

impl PositionedLayer for SurfaceTransformLayer {
    fn set_offset(&mut self, offset: Offset) {
        if self.offset != offset {
            self.offset = offset;
            self.dirty = true;
        }
    }
}

/// A layer that applies a reduced opacity to its children.
pub struct OpacityLayer {
    alpha: f32,
    clip: PaintShape,
    offset: Offset,
    dirty: bool,
    children: ChildLayers,
}

impl OpacityLayer {
    pub fn new(alpha: f32, clip: PaintShape) -> Self {
        Self {
            alpha,
            clip,
            offset: Offset::ZERO,
            dirty: true,
            children: ChildLayers::new(),
        }
    }

    /// Replaces the opacity applied to the children.
    pub fn set_alpha(&mut self, alpha: f32) {
        self.alpha = alpha;
        self.dirty = true;
    }
}

impl Layer for OpacityLayer {
    fn compose(&mut self, compositor: &mut Compositor) {
        let children = self.children.compose_frame();
        compositor.push_opacity(self.offset, self.alpha, Some(self.clip.clone()), &children);

        self.dirty = false;
    }

    fn update_dirty(&mut self) -> bool {
        self.dirty = self.children.update_dirty() || self.dirty;
        self.dirty
    }
}

impl ContainerLayer for OpacityLayer {
    fn append(&mut self, child: LayerHandle) {
        self.children.append(child);
        self.dirty = true;
    }
}

impl PositionedLayer for OpacityLayer {
    fn set_offset(&mut self, offset: Offset) {
        if self.offset != offset {
            self.offset = offset;
            self.dirty = true;
        }
    }
}

/// Positions a sequence of child layers as a group, with no other effect of its own.
pub struct OffsetLayer {
    offset: Offset,
    dirty: bool,
    children: ChildLayers,
}

impl OffsetLayer {
    pub fn new() -> Self {
        Self {
            offset: Offset::ZERO,
            dirty: true,
            children: ChildLayers::new(),
        }
    }

    /// Removes every child from the layer, keeping its position.
    pub fn clear(&mut self) {
        self.children.clear();
        self.dirty = true;
    }
}

impl Default for OffsetLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Layer for OffsetLayer {
    fn compose(&mut self, compositor: &mut Compositor) {
        let children = self.children.compose_frame();
        compositor.push_transform(Affine::translate(self.offset), &children);

        self.dirty = false;
    }

    fn update_dirty(&mut self) -> bool {
        self.dirty = self.children.update_dirty() || self.dirty;
        self.dirty
    }
}

impl ContainerLayer for OffsetLayer {
    fn append(&mut self, child: LayerHandle) {
        self.children.append(child);
        self.dirty = true;
    }
}

impl PositionedLayer for OffsetLayer {
    fn set_offset(&mut self, offset: Offset) {
        if self.offset != offset {
            self.offset = offset;
            self.dirty = true;
        }
    }
}

/// A leaf holding finished drawing.
pub struct PictureLayer {
    picture: Rc<Scene>,
}

impl PictureLayer {
    pub fn new(picture: Scene) -> Self {
        Self {
            picture: Rc::new(picture),
        }
    }
}

impl Layer for PictureLayer {
    fn compose(&mut self, compositor: &mut Compositor) {
        compositor.push_raster(Rc::clone(&self.picture));
    }

    fn update_dirty(&mut self) -> bool {
        false
    }
}

/// A leaf placing a surface the system compositor owns, identified by [`ExternalSurfaceId`].
///
/// It draws nothing itself; composing it contributes one [`External`](CompositedNode::External)
/// node, so a backend positions the matching system visual where the layer sits.
pub struct ExternalSurfaceLayer {
    surface: ExternalSurfaceId,
    size: Size,
    offset: Offset,
    dirty: bool,
}

impl ExternalSurfaceLayer {
    pub fn new(surface: ExternalSurfaceId, size: Size) -> Self {
        Self {
            surface,
            size,
            offset: Offset::ZERO,
            dirty: true,
        }
    }

    /// Replaces the surface this layer places.
    pub fn set_surface(&mut self, surface: ExternalSurfaceId) {
        if self.surface != surface {
            self.surface = surface;
            self.dirty = true;
        }
    }

    /// Resizes the placed surface.
    pub fn set_size(&mut self, size: Size) {
        if self.size != size {
            self.size = size;
            self.dirty = true;
        }
    }
}

impl Layer for ExternalSurfaceLayer {
    fn compose(&mut self, compositor: &mut Compositor) {
        compositor.push_external(
            self.surface,
            self.size,
            Affine::translate(self.offset),
            1.0,
            None,
        );

        self.dirty = false;
    }

    fn update_dirty(&mut self) -> bool {
        self.dirty
    }
}

impl PositionedLayer for ExternalSurfaceLayer {
    fn set_offset(&mut self, offset: Offset) {
        if self.offset != offset {
            self.offset = offset;
            self.dirty = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use peniko::{
        Brush, Color, Fill,
        kurbo::{Affine, Stroke},
    };

    use crate::{
        geometry::{Offset, Rect, Size},
        paint::canvas::Canvas,
    };

    use super::*;

    /// A leaf that records how many times it was composed, so a test can prove a cached subtree is
    /// replayed rather than recomposed.
    struct CountingLayer {
        composes: Rc<Cell<usize>>,
        dirty: Rc<Cell<bool>>,
        picture: Rc<Scene>,
    }

    impl CountingLayer {
        fn new() -> (Self, Rc<Cell<usize>>, Rc<Cell<bool>>) {
            let composes = Rc::new(Cell::new(0));
            let dirty = Rc::new(Cell::new(false));

            (
                Self {
                    composes: Rc::clone(&composes),
                    dirty: Rc::clone(&dirty),
                    picture: Rc::new(solid_fill(Color::BLACK)),
                },
                composes,
                dirty,
            )
        }
    }

    impl Layer for CountingLayer {
        fn compose(&mut self, compositor: &mut Compositor) {
            self.composes.set(self.composes.get() + 1);
            self.dirty.set(false);
            compositor.push_raster(Rc::clone(&self.picture));
        }

        fn update_dirty(&mut self) -> bool {
            self.dirty.get()
        }
    }

    fn solid_fill(color: Color) -> Scene {
        Canvas::record(|canvas| {
            let brush = canvas.brush(color);
            canvas.fill(Fill::NonZero, brush, &Rect::from(Size::new(1.0, 1.0)));
        })
    }

    fn picture(color: Color) -> LayerHandle<PictureLayer> {
        LayerHandle::new(PictureLayer::new(solid_fill(color)))
    }

    fn unit_clip() -> PaintShape {
        PaintShape::from_shape(&Rect::from(Size::new(1.0, 1.0)))
    }

    fn external(id: u64) -> LayerHandle<ExternalSurfaceLayer> {
        LayerHandle::new(ExternalSurfaceLayer::new(
            ExternalSurfaceId(id),
            Size::new(16.0, 9.0),
        ))
    }

    /// The single rasterized node of a frame, flattened.
    fn only_raster(frame: &CompositedFrame) -> Scene {
        match frame.nodes() {
            [CompositedNode::Raster { scene }] => scene.flatten(),
            other => panic!("expected one raster node, got {other:?}"),
        }
    }

    /// The transform in effect at the first fill of a scene, walking its transform stack.
    fn fill_transform(scene: &Scene) -> Affine {
        let flat = scene.flatten();

        let mut current = Affine::IDENTITY;
        let mut stack = Vec::new();
        for command in flat.commands() {
            match command {
                PaintCommand::PushTransform(transform) => {
                    stack.push(current);
                    current *= *transform;
                }
                PaintCommand::PopTransform => {
                    current = stack.pop().expect("balanced transform stack");
                }
                PaintCommand::Fill { .. } => return current,
                _ => {}
            }
        }

        panic!("expected a fill, got {:?}", flat.commands());
    }

    fn fill_colors(scene: &Scene) -> Vec<Color> {
        scene
            .flatten()
            .commands()
            .iter()
            .filter_map(|command| match command {
                PaintCommand::Fill { brush, .. } => match scene.brush(*brush) {
                    Brush::Solid(color) => Some(*color),
                    other => panic!("expected a solid brush, got {other:?}"),
                },
                _ => None,
            })
            .collect()
    }

    fn transform_over(child: LayerHandle, transform: Affine) -> LayerHandle<TransformLayer> {
        let mut layer = TransformLayer::new(transform);
        layer.append(child);
        LayerHandle::new(layer)
    }

    /// A picture composes to one rasterized entry holding its drawing.
    #[test]
    fn a_picture_composes_to_one_raster_entry() {
        let frame = Compositor::compose(&picture(Color::BLACK));
        assert_eq!(fill_colors(&only_raster(&frame)), vec![Color::BLACK]);
    }

    /// A transform places the drawing under it.
    #[test]
    fn a_transform_places_drawing_under_it() {
        let layer = transform_over(picture(Color::BLACK).into(), Affine::translate((10.0, 0.0)));
        let frame = Compositor::compose(&layer);
        assert_eq!(
            fill_transform(&only_raster(&frame)),
            Affine::translate((10.0, 0.0))
        );
    }

    /// Nested transforms compose onto the drawing.
    #[test]
    fn nested_transforms_compose() {
        let inner = transform_over(picture(Color::BLACK).into(), Affine::translate((5.0, 0.0)));
        let outer = transform_over(inner.into(), Affine::translate((0.0, 3.0)));
        let frame = Compositor::compose(&outer);
        assert_eq!(
            fill_transform(&only_raster(&frame)),
            Affine::translate((5.0, 3.0))
        );
    }

    /// Adjacent rasterized children accumulate into one entry holding both, in order.
    #[test]
    fn adjacent_rasters_accumulate_into_one_entry() {
        let red = Color::from_rgb8(255, 0, 0);
        let blue = Color::from_rgb8(0, 0, 255);

        let mut container = OffsetLayer::new();
        container.append(picture(red).into());
        container.append(picture(blue).into());

        let frame = Compositor::compose(&LayerHandle::new(container));
        assert_eq!(fill_colors(&only_raster(&frame)), vec![red, blue]);
    }

    // ── caching ──────────────────────────────────────────────────────────────────────────────────

    /// Changing only the transform re-places the cached subtree at the new transform without
    /// recomposing it.
    #[test]
    fn animating_a_transform_reuses_the_child_cache() {
        let (child, composes, _) = CountingLayer::new();
        let layer = transform_over(
            LayerHandle::new(child).into(),
            Affine::translate((10.0, 0.0)),
        );

        let first = Compositor::compose(&layer);
        assert_eq!(composes.get(), 1);
        assert_eq!(
            fill_transform(&only_raster(&first)),
            Affine::translate((10.0, 0.0))
        );

        layer
            .borrow_mut()
            .set_transform(Affine::translate((20.0, 0.0)));

        let second = Compositor::compose(&layer);
        assert_eq!(
            composes.get(),
            1,
            "the cached subtree was replayed, not recomposed"
        );
        assert_eq!(
            fill_transform(&only_raster(&second)),
            Affine::translate((20.0, 0.0))
        );
    }

    /// Recomposing with nothing dirty touches no child.
    #[test]
    fn a_clean_recompose_touches_no_child() {
        let (child, composes, _) = CountingLayer::new();
        let layer = transform_over(LayerHandle::new(child).into(), Affine::IDENTITY);

        Compositor::compose(&layer);
        Compositor::compose(&layer);

        assert_eq!(composes.get(), 1);
    }

    /// A dirtied child does recompose.
    #[test]
    fn a_dirty_child_recomposes() {
        let (child, composes, dirty) = CountingLayer::new();
        let layer = transform_over(LayerHandle::new(child).into(), Affine::IDENTITY);

        Compositor::compose(&layer);
        assert_eq!(composes.get(), 1);

        dirty.set(true);
        Compositor::compose(&layer);
        assert_eq!(composes.get(), 2, "the dirtied child recomposed");
    }

    /// A transform change deep in the tree re-places the cached leaf but does not recompose it.
    #[test]
    fn a_change_bubbles_to_ancestors_but_not_the_leaf() {
        let (child, composes, _) = CountingLayer::new();
        let inner = transform_over(
            LayerHandle::new(child).into(),
            Affine::translate((5.0, 0.0)),
        );

        let mut outer = OffsetLayer::new();
        outer.append(inner.clone().into());
        let outer = LayerHandle::new(outer);

        let first = Compositor::compose(&outer);
        assert_eq!(composes.get(), 1);
        assert_eq!(
            fill_transform(&only_raster(&first)),
            Affine::translate((5.0, 0.0))
        );

        inner
            .borrow_mut()
            .set_transform(Affine::translate((7.0, 0.0)));

        let second = Compositor::compose(&outer);
        assert_eq!(
            composes.get(),
            1,
            "leaf cache survived a change to an ancestor transform"
        );
        assert_eq!(
            fill_transform(&only_raster(&second)),
            Affine::translate((7.0, 0.0))
        );
    }

    /// Repositioning a retained container re-places its cached child without recomposing it.
    #[test]
    fn repositioning_a_container_reuses_the_child_cache() {
        let (child, composes, _) = CountingLayer::new();
        let mut inner = OffsetLayer::new();
        inner.append(LayerHandle::new(child).into());
        let inner = LayerHandle::new(inner);

        let mut outer = OffsetLayer::new();
        outer.append(inner.clone().into());
        let outer = LayerHandle::new(outer);

        let first = Compositor::compose(&outer);
        assert_eq!(composes.get(), 1);
        assert_eq!(fill_transform(&only_raster(&first)), Affine::IDENTITY);

        inner.borrow_mut().set_offset(Offset::new(6.0, 0.0));
        let second = Compositor::compose(&outer);
        assert_eq!(
            composes.get(),
            1,
            "the cached child was replayed at the new offset"
        );
        assert_eq!(
            fill_transform(&only_raster(&second)),
            Affine::translate((6.0, 0.0))
        );
    }

    // ── rasterize and structure ──────────────────────────────────────────────────────────────────

    /// A stroke survives rasterizing the composed frame, under its transform.
    #[test]
    fn a_stroke_survives_rasterize() {
        let stroke = Canvas::record(|canvas| {
            let style = canvas.stroke_style(Stroke::new(2.0));
            let brush = canvas.brush(Color::BLACK);
            canvas.stroke(style, brush, &Rect::from(Size::new(1.0, 1.0)));
        });

        let layer = transform_over(
            LayerHandle::new(PictureLayer::new(stroke)).into(),
            Affine::translate((3.0, 0.0)),
        );
        let scene = Compositor::compose(&layer).rasterize();

        assert!(matches!(
            scene.commands(),
            [
                PaintCommand::PushTransform(t),
                PaintCommand::Stroke { stroke, brush, .. },
                PaintCommand::PopTransform,
            ] if *t == Affine::translate((3.0, 0.0))
                && (scene.stroke(*stroke).width - 2.0).abs() < 1e-9
                && matches!(scene.brush(*brush), Brush::Solid(c) if *c == Color::BLACK)
        ));
    }

    /// An empty container composes to an empty frame.
    #[test]
    fn an_empty_container_composes_to_nothing() {
        let frame = Compositor::compose(&LayerHandle::new(OffsetLayer::new()));
        assert!(frame.is_empty());
    }

    // ── promotion ────────────────────────────────────────────────────────────────────────────────

    /// A static transform over an all-raster subtree bakes into one rasterized node, never a surface.
    #[test]
    fn a_static_transform_over_raster_bakes() {
        let layer = transform_over(picture(Color::BLACK).into(), Affine::translate((10.0, 0.0)));
        let frame = Compositor::compose(&layer);

        assert!(
            matches!(frame.nodes(), [CompositedNode::Raster { .. }]),
            "an all-raster subtree stays a single raster node, got {:?}",
            frame.nodes()
        );
    }

    /// An animated transform always takes a surface, even over an all-raster subtree, so it can move
    /// its visual without recompositing.
    #[test]
    fn an_animated_transform_always_takes_a_surface() {
        let mut animated = SurfaceTransformLayer::new(Affine::translate((10.0, 0.0)));
        animated.append(picture(Color::BLACK).into());

        let frame = Compositor::compose(&LayerHandle::new(animated));

        match frame.nodes() {
            [
                CompositedNode::Surface {
                    placement,
                    children,
                },
            ] => {
                assert_eq!(placement.transform, Affine::translate((10.0, 0.0)));
                assert!(
                    placement.transform_handle.is_some(),
                    "an animated transform carries its handle"
                );
                assert!(matches!(children.nodes(), [CompositedNode::Raster { .. }]));
            }
            other => panic!("expected one surface node, got {other:?}"),
        }
    }

    // ── external surfaces ────────────────────────────────────────────────────────────────────────

    /// A surface placed between two rasterized children stands as its own node, with the raster on
    /// each side accumulated separately around it.
    #[test]
    fn a_surface_splits_the_raster_around_it() {
        let mut container = OffsetLayer::new();
        container.append(picture(Color::BLACK).into());
        container.append(external(7).into());
        container.append(picture(Color::WHITE).into());

        let frame = Compositor::compose(&LayerHandle::new(container));

        match frame.nodes() {
            [
                CompositedNode::Raster { .. },
                CompositedNode::External { surface, size, .. },
                CompositedNode::Raster { .. },
            ] => {
                assert_eq!(*surface, ExternalSurfaceId(7));
                assert_eq!(*size, Size::new(16.0, 9.0));
            }
            other => panic!("expected raster, external, raster; got {other:?}"),
        }
    }

    /// Nested transforms over an external each take a surface, so the external inherits their product
    /// down the visual tree rather than being baked.
    #[test]
    fn nested_transforms_over_an_external_nest_surfaces() {
        let inner = {
            let mut layer = TransformLayer::new(Affine::translate((5.0, 0.0)));
            layer.append(external(1).into());
            LayerHandle::new(layer)
        };

        let mut outer = TransformLayer::new(Affine::translate((0.0, 3.0)));
        outer.append(inner.into());

        let frame = Compositor::compose(&LayerHandle::new(outer));

        match frame.nodes() {
            [
                CompositedNode::Surface {
                    placement: outer,
                    children: mid,
                },
            ] => match mid.nodes() {
                [
                    CompositedNode::Surface {
                        placement: inner,
                        children: leaf,
                    },
                ] => match leaf.nodes() {
                    [CompositedNode::External { transform, .. }] => {
                        let absolute = outer.transform * inner.transform * *transform;
                        assert_eq!(absolute, Affine::translate((5.0, 3.0)));
                    }
                    other => panic!("expected one external node, got {other:?}"),
                },
                other => panic!("expected a nested surface, got {other:?}"),
            },
            other => panic!("expected one surface node, got {other:?}"),
        }
    }

    /// An enclosing opacity puts the external on a faded, clipped visual rather than rasterizing over
    /// it.
    #[test]
    fn an_opacity_over_an_external_takes_a_surface() {
        let mut opacity = OpacityLayer::new(0.5, unit_clip());
        opacity.append(external(2).into());

        let frame = Compositor::compose(&LayerHandle::new(opacity));

        match frame.nodes() {
            [
                CompositedNode::Surface {
                    placement,
                    children,
                },
            ] => {
                assert!((placement.opacity - 0.5).abs() < 1e-9);
                assert!(
                    placement.clip.is_some(),
                    "the opacity's clip reaches the visual"
                );
                assert!(matches!(
                    children.nodes(),
                    [CompositedNode::External { .. }]
                ));
            }
            other => panic!("expected one surface node, got {other:?}"),
        }
    }

    /// Drawing on both sides of an external puts the whole group on one visual, with the over-content
    /// stacked above the external as its own raster node.
    #[test]
    fn raster_resumes_after_an_external() {
        let mut container = TransformLayer::new(Affine::translate((4.0, 0.0)));
        container.append(picture(Color::BLACK).into());
        container.append(external(9).into());
        container.append(picture(Color::WHITE).into());

        let frame = Compositor::compose(&LayerHandle::new(container));

        match frame.nodes() {
            [
                CompositedNode::Surface {
                    placement,
                    children,
                },
            ] => {
                assert_eq!(placement.transform, Affine::translate((4.0, 0.0)));
                assert!(matches!(
                    children.nodes(),
                    [
                        CompositedNode::Raster { .. },
                        CompositedNode::External { .. },
                        CompositedNode::Raster { .. },
                    ]
                ));
            }
            other => panic!("expected one surface node, got {other:?}"),
        }
    }
}
