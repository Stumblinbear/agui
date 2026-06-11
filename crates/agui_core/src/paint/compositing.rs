use std::{
    cell::{Ref, RefCell, RefMut},
    rc::Rc,
};

use peniko::{BlendMode, Compose, Mix, kurbo::Affine};

use crate::{
    geometry::Offset,
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

/// A node in the retained compositing tree.
///
/// Composing a layer contributes its content to the scene under construction.
pub trait Layer {
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

/// An ordered set of child layers.
struct ChildLayers {
    children: Vec<LayerHandle>,
    dirty: bool,
    cache: Option<Rc<Scene>>,
}

impl ChildLayers {
    fn new() -> Self {
        Self {
            children: Vec::new(),
            dirty: false,
            cache: None,
        }
    }

    fn append(&mut self, child: LayerHandle) {
        self.children.push(child);
    }

    /// Removes every child and the cached composition.
    fn clear(&mut self) {
        self.children.clear();
        self.dirty = true;
        self.cache = None;
    }

    /// Settles the children's dirty state and returns whether any changed.
    fn update_dirty(&mut self) -> bool {
        let mut dirty = false;

        for child in &self.children {
            dirty |= child.borrow_mut().update_dirty();
        }

        self.dirty = dirty;
        self.dirty
    }

    /// Composes the children, reusing the cache when none have changed, and embeds the result.
    fn embed(&mut self, compositor: &mut Compositor) {
        if self.cache.is_none() || self.dirty {
            // A prior composition's embed may still reference the cache, so reuse it only when unaliased.
            if self.cache.as_mut().and_then(Rc::get_mut).is_none() {
                self.cache = Some(Rc::new(Scene::new()));
            }

            let scene = self
                .cache
                .as_mut()
                .and_then(Rc::get_mut)
                .expect("the cache was just made unique");

            scene.reset();

            let mut sub = Compositor { scene };
            for child in &self.children {
                child.borrow_mut().compose(&mut sub);
            }
        }

        if let Some(scene) = &self.cache {
            compositor.embed(Rc::clone(scene));
        }
    }
}

/// Builds a [`Scene`] from a layer tree.
pub struct Compositor<'a> {
    scene: &'a mut Scene,
}

impl Compositor<'_> {
    /// Composes `root` and its descendants into a fresh scene.
    pub fn compose<L: Layer + ?Sized>(root: &LayerHandle<L>) -> Scene {
        let mut scene = Scene::new();
        Self::do_compose(root, &mut scene);
        scene
    }

    /// Composes `root` and its descendants into `scene`, replacing its previous content. Composing
    /// successive frames into one held scene reuses its storage instead of allocating each frame.
    pub fn compose_into<L: Layer + ?Sized>(root: &LayerHandle<L>, scene: &mut Scene) {
        scene.reset();

        Self::do_compose(root, scene);
    }

    fn do_compose<L: Layer + ?Sized>(root: &LayerHandle<L>, scene: &mut Scene) {
        let mut root = root.borrow_mut();

        // Settle every dirty flag before composing; compose reads them to decide cache reuse.
        root.update_dirty();
        root.compose(&mut Compositor { scene });
    }

    /// Splices `sub` into the scene by reference, under the transform in effect.
    pub fn embed(&mut self, sub: Rc<Scene>) {
        self.scene.push(PaintCommand::Embed { scene: sub });
    }

    /// Begins a transform; everything emitted until [`pop_transform`](Compositor::pop_transform) is
    /// placed under it.
    pub fn push_transform(&mut self, transform: Affine) {
        self.scene.push(PaintCommand::PushTransform(transform));
    }

    /// Ends the most recent [`push_transform`](Compositor::push_transform).
    pub fn pop_transform(&mut self) {
        self.scene.push(PaintCommand::PopTransform);
    }

    /// Begins a composited group; everything emitted until [`pop_layer`](Compositor::pop_layer) is
    /// composited as one and blended into the scene.
    pub fn push_layer(&mut self, blend: BlendMode, alpha: f32, clip: PaintShape) {
        self.scene
            .push(PaintCommand::PushLayer { blend, alpha, clip });
    }

    /// Ends the most recent [`push_layer`](Compositor::push_layer).
    pub fn pop_layer(&mut self) {
        self.scene.push(PaintCommand::PopLayer);
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
            dirty: false,

            children: ChildLayers::new(),
        }
    }

    /// Replaces the transform applied to the children.
    pub fn set_transform(&mut self, transform: Affine) {
        self.transform = transform;
        self.dirty = true;
    }

    /// The transform currently applied to the children.
    pub fn transform(&self) -> Affine {
        self.transform
    }

    /// Removes every child, keeping the transform and offset. The parent repaints the children in.
    pub fn clear(&mut self) {
        self.children.clear();
    }
}

impl Layer for TransformLayer {
    fn compose(&mut self, compositor: &mut Compositor) {
        // A changed transform or offset only re-places the children; it doesn't invalidate their
        // cache.
        compositor.push_transform(Affine::translate(self.offset) * self.transform);
        self.children.embed(compositor);
        compositor.pop_transform();

        self.dirty = false;
    }

    fn update_dirty(&mut self) -> bool {
        self.children.update_dirty() || self.dirty
    }
}

impl ContainerLayer for TransformLayer {
    fn append(&mut self, child: LayerHandle) {
        self.children.append(child);
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
            dirty: false,

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
        {
            self.children.embed(compositor);
        }
        compositor.pop_layer();

        if positioned {
            compositor.pop_transform();
        }

        self.dirty = false;
    }

    fn update_dirty(&mut self) -> bool {
        self.children.update_dirty() || self.dirty
    }
}

impl ContainerLayer for OpacityLayer {
    fn append(&mut self, child: LayerHandle) {
        self.children.append(child);
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
            dirty: false,

            children: ChildLayers::new(),
        }
    }

    /// Removes every child from the layer, keeping its position.
    pub fn clear(&mut self) {
        self.children.clear();
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
            self.children.embed(compositor);
        } else {
            compositor.push_transform(Affine::translate(self.offset));
            self.children.embed(compositor);
            compositor.pop_transform();
        }

        self.dirty = false;
    }

    fn update_dirty(&mut self) -> bool {
        self.children.update_dirty() || self.dirty
    }
}

impl ContainerLayer for OffsetLayer {
    fn append(&mut self, child: LayerHandle) {
        self.children.append(child);
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

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use peniko::{
        Brush, Color, Fill,
        kurbo::{Affine, Stroke},
    };

    use crate::{
        geometry::{Rect, Size},
        paint::canvas::Canvas,
    };

    use super::*;

    /// A leaf that records how many times it composed, so a test can prove a cached layer is replayed
    /// rather than recomposed.
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

    /// The transform in effect at the single fill of a composed scene, walking the transform stack of
    /// the flattened result.
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

    fn transform_over(
        child: impl Layer + 'static,
        transform: Affine,
    ) -> LayerHandle<TransformLayer> {
        let mut layer = TransformLayer::new(transform);
        layer.append(LayerHandle::new(child).into());
        LayerHandle::new(layer)
    }

    /// Changing only the transform re-places the child's cached drawing at the new transform; the
    /// child is composed exactly once across the animation.
    #[test]
    fn animating_a_transform_reuses_the_child_cache() {
        let (child, composes, _) = CountingLayer::new();
        let layer = transform_over(child, Affine::translate((10.0, 0.0)));

        let first = Compositor::compose(&layer);
        assert_eq!(composes.get(), 1);
        assert_eq!(fill_transform(&first), Affine::translate((10.0, 0.0)));

        layer
            .borrow_mut()
            .set_transform(Affine::translate((20.0, 0.0)));

        let second = Compositor::compose(&layer);
        assert_eq!(
            composes.get(),
            1,
            "child cache reused — the child did not recompose"
        );
        assert_eq!(fill_transform(&second), Affine::translate((20.0, 0.0)));
    }

    /// Recomposing with nothing dirty touches no child.
    #[test]
    fn a_clean_recompose_touches_no_child() {
        let (child, composes, _) = CountingLayer::new();
        let layer = transform_over(child, Affine::IDENTITY);

        Compositor::compose(&layer);
        Compositor::compose(&layer);

        assert_eq!(composes.get(), 1);
    }

    /// A dirtied child does recompose.
    #[test]
    fn a_dirty_child_recomposes() {
        let (child, composes, dirty) = CountingLayer::new();
        let layer = transform_over(child, Affine::IDENTITY);

        Compositor::compose(&layer);
        assert_eq!(composes.get(), 1);

        dirty.set(true);
        Compositor::compose(&layer);
        assert_eq!(composes.get(), 2, "the dirtied child recomposed");
    }

    // ── caching granularity ────────────────────────────────────────────────────────────────────

    /// A transform change deep in the tree invalidates ancestors' caches but not the leaf's: the leaf
    /// is composed once, while the placement still updates.
    #[test]
    fn a_change_bubbles_to_ancestors_but_not_the_leaf() {
        let (child, composes, _) = CountingLayer::new();
        let inner = transform_over(child, Affine::translate((5.0, 0.0)));

        let mut outer = OffsetLayer::new();
        outer.append(inner.clone().into());
        let outer = LayerHandle::new(outer);

        let first = Compositor::compose(&outer);
        assert_eq!(composes.get(), 1);
        assert_eq!(fill_transform(&first), Affine::translate((5.0, 0.0)));

        inner
            .borrow_mut()
            .set_transform(Affine::translate((7.0, 0.0)));

        let second = Compositor::compose(&outer);
        assert_eq!(
            composes.get(),
            1,
            "leaf cache survived a change to an ancestor transform"
        );
        assert_eq!(fill_transform(&second), Affine::translate((7.0, 0.0)));
    }

    /// Dirtying one child of a container forces the container to re-emit, but a clean sibling's
    /// subtree is not recomposed — only the dirty branch pays.
    #[test]
    fn a_dirty_sibling_does_not_recompose_a_clean_one() {
        let (leaf_a, count_a, _) = CountingLayer::new();
        let (leaf_b, count_b, _) = CountingLayer::new();

        let a = transform_over(leaf_a, Affine::IDENTITY);
        let b = transform_over(leaf_b, Affine::IDENTITY);

        let mut container = OffsetLayer::new();
        container.append(a.clone().into());
        container.append(b.into());
        let container = LayerHandle::new(container);

        Compositor::compose(&container);
        assert_eq!(count_a.get(), 1);
        assert_eq!(count_b.get(), 1);

        a.borrow_mut().set_transform(Affine::translate((5.0, 0.0)));
        Compositor::compose(&container);

        assert_eq!(
            count_a.get(),
            1,
            "A's own leaf cache reused despite A's transform change"
        );
        assert_eq!(count_b.get(), 1, "B's subtree untouched by A's change");
    }

    /// After a change is recomposed, the dirty flags clear, so a following pass with nothing changed
    /// does no work at any level.
    #[test]
    fn a_change_clears_so_a_following_compose_is_idle() {
        let (leaf, count, _) = CountingLayer::new();
        let layer = transform_over(leaf, Affine::IDENTITY);

        Compositor::compose(&layer);
        assert_eq!(count.get(), 1);

        layer
            .borrow_mut()
            .set_transform(Affine::translate((2.0, 0.0)));
        Compositor::compose(&layer);
        assert_eq!(count.get(), 1);

        Compositor::compose(&layer);
        assert_eq!(count.get(), 1, "a third, unchanged pass did no work");
    }

    /// Repositioning a retained container re-places its children without recomposing them.
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
        assert_eq!(fill_transform(&first), Affine::IDENTITY);

        inner.borrow_mut().set_offset(Offset::new(6.0, 0.0));
        let second = Compositor::compose(&outer);
        assert_eq!(
            composes.get(),
            1,
            "the child cache was replayed at the new offset"
        );
        assert_eq!(fill_transform(&second), Affine::translate((6.0, 0.0)));
    }

    /// Re-setting the offset a layer already has leaves it clean.
    #[test]
    fn an_unchanged_offset_marks_nothing_dirty() {
        let layer = LayerHandle::new(OffsetLayer::new());
        Compositor::compose(&layer);

        layer.borrow_mut().set_offset(Offset::new(2.0, 0.0));
        assert!(layer.borrow_mut().update_dirty());
        Compositor::compose(&layer);

        layer.borrow_mut().set_offset(Offset::new(2.0, 0.0));
        assert!(!layer.borrow_mut().update_dirty());
    }

    /// The offset a parent placed a layer at survives the layer clearing and repainting its own
    /// content.
    #[test]
    fn an_offset_survives_clearing_the_children() {
        let layer = LayerHandle::new(OffsetLayer::new());
        layer.borrow_mut().set_offset(Offset::new(3.0, 0.0));

        {
            let mut guard = layer.borrow_mut();
            guard.clear();
            guard.append(LayerHandle::new(PictureLayer::new(solid_fill(Color::BLACK))).into());
        }

        let scene = Compositor::compose(&layer);
        assert_eq!(fill_transform(&scene), Affine::translate((3.0, 0.0)));
    }

    /// Composing successive frames into one held scene replays caches and reflects changes, as a
    /// per-frame driver does.
    #[test]
    fn compose_into_reuses_one_scene_across_frames() {
        let (child, composes, _) = CountingLayer::new();
        let layer = transform_over(child, Affine::translate((10.0, 0.0)));

        let mut scene = Scene::new();
        Compositor::compose_into(&layer, &mut scene);
        assert_eq!(composes.get(), 1);
        assert_eq!(fill_transform(&scene), Affine::translate((10.0, 0.0)));

        layer
            .borrow_mut()
            .set_transform(Affine::translate((20.0, 0.0)));

        Compositor::compose_into(&layer, &mut scene);
        assert_eq!(
            composes.get(),
            1,
            "the leaf cache was replayed, not recomposed"
        );
        assert_eq!(fill_transform(&scene), Affine::translate((20.0, 0.0)));
    }

    /// Recomposing a dirtied tree leaves a still-held previous composition intact.
    #[test]
    fn a_recompose_leaves_a_held_composition_intact() {
        let (child, _, dirty) = CountingLayer::new();
        let layer = transform_over(child, Affine::IDENTITY);

        let first = Compositor::compose(&layer);

        dirty.set(true);
        layer
            .borrow_mut()
            .set_transform(Affine::translate((5.0, 0.0)));
        let second = Compositor::compose(&layer);

        assert_eq!(fill_transform(&first), Affine::IDENTITY);
        assert_eq!(fill_transform(&second), Affine::translate((5.0, 0.0)));
    }

    /// A change bubbles up through several clean container ancestors, rebuilding each cache, while the
    /// leaf below is composed only once.
    #[test]
    fn a_change_bubbles_through_multiple_clean_ancestors() {
        let (leaf, count, _) = CountingLayer::new();
        let transform = transform_over(leaf, Affine::translate((1.0, 0.0)));

        let mut mid = OffsetLayer::new();
        mid.append(transform.clone().into());
        let mut top = OffsetLayer::new();
        top.append(LayerHandle::new(mid).into());
        let top = LayerHandle::new(top);

        Compositor::compose(&top);
        assert_eq!(count.get(), 1);

        transform
            .borrow_mut()
            .set_transform(Affine::translate((9.0, 0.0)));
        let scene = Compositor::compose(&top);

        assert_eq!(fill_transform(&scene), Affine::translate((9.0, 0.0)));
        assert_eq!(
            count.get(),
            1,
            "the leaf survived a change through two clean containers"
        );
    }

    #[test]
    fn flatten_inlines_references_and_keeps_distinct_brushes() {
        let red = Color::from_rgb8(255, 0, 0);
        let blue = Color::from_rgb8(0, 0, 255);

        let mut container = OffsetLayer::new();
        container.append(LayerHandle::new(PictureLayer::new(solid_fill(red))).into());
        container.append(LayerHandle::new(PictureLayer::new(solid_fill(blue))).into());
        let scene = Compositor::compose(&LayerHandle::new(container)).flatten();

        let colors: Vec<Color> = scene
            .commands()
            .iter()
            .filter_map(|command| match command {
                PaintCommand::Fill { brush, .. } => match scene.brush(*brush) {
                    Brush::Solid(color) => Some(*color),
                    other => panic!("expected a solid brush, got {other:?}"),
                },
                _ => None,
            })
            .collect();

        assert_eq!(colors, vec![red, blue]);
    }

    #[test]
    fn a_stroke_survives_flatten() {
        let picture = Canvas::record(|canvas| {
            let style = canvas.stroke_style(Stroke::new(2.0));
            let brush = canvas.brush(Color::BLACK);
            canvas.stroke(style, brush, &Rect::from(Size::new(1.0, 1.0)));
        });

        let layer = transform_over(PictureLayer::new(picture), Affine::translate((3.0, 0.0)));
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

    #[test]
    fn a_clip_group_survives_flatten() {
        let picture = Canvas::record(|canvas| {
            canvas.with_clip(&Rect::from(Size::new(5.0, 5.0)), |canvas| {
                let brush = canvas.brush(Color::BLACK);

                canvas.fill(Fill::NonZero, brush, &Rect::from(Size::new(1.0, 1.0)));
            });
        });

        let layer = transform_over(PictureLayer::new(picture), Affine::translate((4.0, 0.0)));
        let scene = Compositor::compose(&layer).flatten();

        assert!(matches!(
            scene.commands(),
            [
                PaintCommand::PushTransform(_),
                PaintCommand::PushLayer { .. },
                PaintCommand::Fill { .. },
                PaintCommand::PopLayer,
                PaintCommand::PopTransform,
            ]
        ));
    }

    #[test]
    fn an_empty_container_composes_to_nothing() {
        let scene = Compositor::compose(&LayerHandle::new(OffsetLayer::new())).flatten();

        assert!(scene.is_empty());
    }

    #[test]
    fn an_empty_picture_composes_to_nothing() {
        let scene =
            Compositor::compose(&LayerHandle::new(PictureLayer::new(Scene::new()))).flatten();

        assert!(scene.is_empty());
    }
}
