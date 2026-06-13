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
/// the [`External`](CompositedEntry::External) entry of a composed frame reports where that surface
/// belongs so the driver can position the matching system visual.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ExternalSurfaceId(pub u64);

/// One element of a [`CompositedFrame`].
#[derive(Clone, Debug)]
pub enum CompositedEntry {
    /// Rasterized drawing. Its transforms and opacity are recorded within the scene.
    Raster(Rc<Scene>),

    /// A placement for a surface the system compositor owns. A backend positions that surface at
    /// `transform`, sized to `size`, blended at `alpha`, clipped to `clip` if present, rather than
    /// drawing anything here.
    External {
        surface: ExternalSurfaceId,
        size: Size,
        transform: Affine,
        alpha: f32,
        clip: Option<PaintShape>,
    },
}

/// The result of composing a layer tree: an ordered list of entries to present, some rasterized and
/// some placements for system-composited surfaces.
///
/// Entries are in back-to-front order, so a surface placed between two rasterized entries draws over
/// the first and under the second.
#[derive(Clone, Debug, Default)]
pub struct CompositedFrame {
    entries: Vec<CompositedEntry>,
}

impl CompositedFrame {
    pub fn new() -> Self {
        Self::default()
    }

    /// The entries to present, back-to-front.
    pub fn entries(&self) -> &[CompositedEntry] {
        &self.entries
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Resolves this frame to a single [`Scene`], inlining its rasterized entries in order. Surface
    /// placements are omitted, since they are not rasterized. A backend that presents only
    /// rasterized content uses this; one that also places surfaces reads [`entries`](Self::entries).
    pub fn flatten(&self) -> Scene {
        let mut staged = Scene::new();

        for entry in &self.entries {
            if let CompositedEntry::Raster(scene) = entry {
                staged.push(PaintCommand::Embed {
                    scene: Rc::clone(scene),
                });
            }
        }

        staged.flatten()
    }
}

/// A transform or composited group open while a frame is being built.
enum Open {
    Transform(Affine),
    Layer {
        blend: BlendMode,
        alpha: f32,
        clip: PaintShape,
    },
}

/// Builds a [`CompositedFrame`] from a layer tree.
pub struct Compositor {
    entries: Vec<CompositedEntry>,
    current: Scene,
    current_has_content: bool,
    open: Vec<Open>,
}

impl Compositor {
    fn new() -> Self {
        Self {
            entries: Vec::new(),
            current: Scene::new(),
            current_has_content: false,
            open: Vec::new(),
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

    /// Begins a transform: drawing contributed until the matching [`pop_transform`](Self::pop_transform)
    /// is placed under it.
    pub fn push_transform(&mut self, transform: Affine) {
        self.current.push(PaintCommand::PushTransform(transform));
        self.open.push(Open::Transform(transform));
    }

    /// Ends the most recent [`push_transform`](Self::push_transform).
    pub fn pop_transform(&mut self) {
        self.current.push(PaintCommand::PopTransform);
        self.open.pop();
    }

    /// Begins a composited group: drawing contributed until the matching [`pop_layer`](Self::pop_layer)
    /// is clipped to `clip` and blended as one group with `blend` and `alpha`.
    pub fn push_layer(&mut self, blend: BlendMode, alpha: f32, clip: PaintShape) {
        self.current.push(PaintCommand::PushLayer {
            blend,
            alpha,
            clip: clip.clone(),
        });

        self.open.push(Open::Layer { blend, alpha, clip });
    }

    /// Ends the most recent [`push_layer`](Self::push_layer).
    pub fn pop_layer(&mut self) {
        self.current.push(PaintCommand::PopLayer);
        self.open.pop();
    }

    /// Contributes rasterized drawing, placed under the transforms and groups in effect.
    pub fn embed(&mut self, scene: Rc<Scene>) {
        self.current.push(PaintCommand::Embed { scene });
        self.current_has_content = true;
    }

    /// Replays a cached subtree's `frame` into the build under the transforms and groups in effect:
    /// its rasterized entries accumulate into the current run, its surface placements split it and
    /// take the enclosing transform and opacity.
    pub fn splice(&mut self, frame: &CompositedFrame) {
        for entry in &frame.entries {
            match entry {
                CompositedEntry::Raster(scene) => self.embed(Rc::clone(scene)),

                CompositedEntry::External {
                    surface,
                    size,
                    transform,
                    alpha,
                    clip,
                } => {
                    let (enclosing, group_alpha, group_clip) = self.resolved();

                    self.place_external(
                        *surface,
                        *size,
                        enclosing * *transform,
                        group_alpha * *alpha,
                        clip.clone().or(group_clip),
                    );
                }
            }
        }
    }

    /// Places a system-composited surface, at the transform and opacity in effect. It splits the
    /// rasterized content, so what was drawn before it becomes one entry and what follows begins
    /// another.
    pub fn external(&mut self, surface: ExternalSurfaceId, size: Size) {
        let (transform, alpha, clip) = self.resolved();
        self.place_external(surface, size, transform, alpha, clip);
    }

    /// The transform, opacity, and innermost clip the open groups resolve to.
    fn resolved(&self) -> (Affine, f32, Option<PaintShape>) {
        let mut transform = Affine::IDENTITY;
        let mut alpha = 1.0;
        let mut clip = None;

        for op in &self.open {
            match op {
                Open::Transform(t) => transform *= *t,
                Open::Layer {
                    alpha: a, clip: c, ..
                } => {
                    alpha *= *a;
                    clip = Some(c.clone());
                }
            }
        }

        (transform, alpha, clip)
    }

    /// Seals the run before the surface, emits the placement, and reopens the run after it.
    fn place_external(
        &mut self,
        surface: ExternalSurfaceId,
        size: Size,
        transform: Affine,
        alpha: f32,
        clip: Option<PaintShape>,
    ) {
        self.seal();
        {
            self.entries.push(CompositedEntry::External {
                surface,
                size,
                transform,
                alpha,
                clip,
            });
        }
        self.reopen();
    }

    /// Seals the in-progress scene as a rasterized entry, closing the open brackets so the entry
    /// stands alone. An in-progress scene with only bracket scaffolding and no drawing is discarded.
    fn seal(&mut self) {
        if !self.current_has_content {
            self.current.reset();
            return;
        }

        for index in (0..self.open.len()).rev() {
            let close = match self.open[index] {
                Open::Transform(_) => PaintCommand::PopTransform,
                Open::Layer { .. } => PaintCommand::PopLayer,
            };

            self.current.push(close);
        }

        let scene = std::mem::replace(&mut self.current, Scene::new());
        self.entries.push(CompositedEntry::Raster(Rc::new(scene)));
        self.current_has_content = false;
    }

    /// Reopens the bracket stack into the fresh scene, so drawing after a surface keeps its context.
    fn reopen(&mut self) {
        for index in 0..self.open.len() {
            let open = match &self.open[index] {
                Open::Transform(t) => PaintCommand::PushTransform(*t),

                Open::Layer { blend, alpha, clip } => PaintCommand::PushLayer {
                    blend: *blend,
                    alpha: *alpha,
                    clip: clip.clone(),
                },
            };

            self.current.push(open);
        }
    }

    fn finish(mut self) -> CompositedFrame {
        self.seal();

        CompositedFrame {
            entries: self.entries,
        }
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
struct ChildLayers {
    children: Vec<LayerHandle>,
    dirty: bool,
    cache: Option<Rc<CompositedFrame>>,
}

impl ChildLayers {
    fn new() -> Self {
        Self {
            children: Vec::new(),
            dirty: true,
            cache: None,
        }
    }

    fn append(&mut self, child: LayerHandle) {
        self.children.push(child);
        self.dirty = true;
        self.cache = None;
    }

    /// Removes every child and the cached composition.
    fn clear(&mut self) {
        self.children.clear();
        self.dirty = true;
        self.cache = None;
    }

    /// Settles the children's dirty state, folding it into this set's, and returns whether anything
    /// changed.
    fn update_dirty(&mut self) -> bool {
        let mut dirty = self.dirty;

        for child in &self.children {
            dirty |= child.borrow_mut().update_dirty();
        }

        self.dirty = dirty;
        self.dirty
    }

    /// Composes the children into the build, reusing the cached frame when nothing changed.
    fn compose(&mut self, compositor: &mut Compositor) {
        if !self.dirty
            && let Some(cache) = &self.cache
        {
            compositor.splice(cache);
            return;
        }

        let mut sub = Compositor::new();
        for child in &self.children {
            child.borrow_mut().compose(&mut sub);
        }

        let frame = Rc::new(sub.finish());
        compositor.splice(&frame);
        self.cache = Some(frame);
        self.dirty = false;
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
        compositor.push_transform(Affine::translate(self.offset) * self.transform);
        self.children.compose(compositor);
        compositor.pop_transform();

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
        let positioned = self.offset != Offset::ZERO;
        if positioned {
            compositor.push_transform(Affine::translate(self.offset));
        }

        compositor.push_layer(
            BlendMode::new(Mix::Normal, Compose::SrcOver),
            self.alpha,
            self.clip.clone(),
        );
        self.children.compose(compositor);
        compositor.pop_layer();

        if positioned {
            compositor.pop_transform();
        }

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
        if self.offset == Offset::ZERO {
            self.children.compose(compositor);
        } else {
            compositor.push_transform(Affine::translate(self.offset));
            self.children.compose(compositor);
            compositor.pop_transform();
        }

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
        compositor.embed(Rc::clone(&self.picture));
    }

    fn update_dirty(&mut self) -> bool {
        false
    }
}

/// A leaf placing a surface the system compositor owns, identified by [`ExternalSurfaceId`].
///
/// It draws nothing itself; composing it contributes one [`External`](CompositedEntry::External)
/// entry, so a backend positions the matching system visual where the layer sits.
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
        let positioned = self.offset != Offset::ZERO;
        if positioned {
            compositor.push_transform(Affine::translate(self.offset));
        }

        compositor.external(self.surface, self.size);

        if positioned {
            compositor.pop_transform();
        }

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
            compositor.embed(Rc::clone(&self.picture));
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

    /// The single rasterized entry of a frame, flattened.
    fn only_raster(frame: &CompositedFrame) -> Scene {
        match frame.entries() {
            [CompositedEntry::Raster(scene)] => scene.flatten(),
            other => panic!("expected one raster entry, got {other:?}"),
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

    // ── flatten and structure ──────────────────────────────────────────────────────────────────

    /// A stroke survives a flatten of the composed frame.
    #[test]
    fn a_stroke_survives_flatten() {
        let stroke = Canvas::record(|canvas| {
            let style = canvas.stroke_style(Stroke::new(2.0));
            let brush = canvas.brush(Color::BLACK);
            canvas.stroke(style, brush, &Rect::from(Size::new(1.0, 1.0)));
        });

        let layer = transform_over(
            LayerHandle::new(PictureLayer::new(stroke)).into(),
            Affine::translate((3.0, 0.0)),
        );
        let scene = Compositor::compose(&layer).flatten();

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

    // ── external surfaces ────────────────────────────────────────────────────────────────────────

    /// A surface placed between two rasterized children stands as its own entry, with the raster on
    /// each side accumulated separately around it.
    #[test]
    fn a_surface_splits_the_raster_around_it() {
        let mut container = OffsetLayer::new();
        container.append(picture(Color::BLACK).into());
        container.append(external(7).into());
        container.append(picture(Color::WHITE).into());

        let frame = Compositor::compose(&LayerHandle::new(container));

        match frame.entries() {
            [
                CompositedEntry::Raster(_),
                CompositedEntry::External { surface, size, .. },
                CompositedEntry::Raster(_),
            ] => {
                assert_eq!(*surface, ExternalSurfaceId(7));
                assert_eq!(*size, Size::new(16.0, 9.0));
            }
            other => panic!("expected raster, external, raster; got {other:?}"),
        }
    }

    /// A surface under nested transforms is placed at their product, even spliced from a cache.
    #[test]
    fn a_surface_resolves_its_absolute_transform() {
        let inner = {
            let mut layer = TransformLayer::new(Affine::translate((5.0, 0.0)));
            layer.append(external(1).into());
            LayerHandle::new(layer)
        };

        let mut outer = TransformLayer::new(Affine::translate((0.0, 3.0)));
        outer.append(inner.into());

        let frame = Compositor::compose(&LayerHandle::new(outer));

        match frame.entries() {
            [CompositedEntry::External { transform, .. }] => {
                assert_eq!(*transform, Affine::translate((5.0, 3.0)));
            }
            other => panic!("expected one external entry, got {other:?}"),
        }
    }

    /// An enclosing opacity sets the placed surface's alpha and clip rather than rasterizing over it.
    #[test]
    fn an_opacity_sets_the_surface_alpha() {
        let mut opacity = OpacityLayer::new(0.5, unit_clip());
        opacity.append(external(2).into());

        let frame = Compositor::compose(&LayerHandle::new(opacity));

        match frame.entries() {
            [CompositedEntry::External { alpha, clip, .. }] => {
                assert!((*alpha - 0.5).abs() < 1e-9);
                assert!(
                    clip.is_some(),
                    "the opacity's clip is carried to the surface"
                );
            }
            other => panic!("expected one external entry, got {other:?}"),
        }
    }

    /// Drawing on both sides of a surface keeps each raster side under the enclosing transform: the
    /// surface does not absorb the surrounding content.
    #[test]
    fn raster_resumes_after_a_surface() {
        let mut container = TransformLayer::new(Affine::translate((4.0, 0.0)));
        container.append(picture(Color::BLACK).into());
        container.append(external(9).into());
        container.append(picture(Color::WHITE).into());

        let frame = Compositor::compose(&LayerHandle::new(container));

        match frame.entries() {
            [
                CompositedEntry::Raster(before),
                CompositedEntry::External { transform, .. },
                CompositedEntry::Raster(after),
            ] => {
                assert_eq!(*transform, Affine::translate((4.0, 0.0)));
                assert_eq!(fill_transform(before), Affine::translate((4.0, 0.0)));
                assert_eq!(fill_transform(after), Affine::translate((4.0, 0.0)));
            }
            other => panic!("expected raster, external, raster; got {other:?}"),
        }
    }
}
