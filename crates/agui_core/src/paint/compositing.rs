use std::{
    cell::{Ref, RefCell, RefMut},
    rc::Rc,
};

use peniko::{BlendMode, Compose, Mix, kurbo::Affine};

use crate::paint::{
    command::{PaintCommand, PaintShape},
    scene::Scene,
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
pub trait Container: Layer {
    /// Adds `child` after the existing children.
    fn append(&mut self, child: LayerHandle);
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
            let mut scene = Scene::new();
            {
                let mut sub = Compositor { scene: &mut scene };

                for child in &self.children {
                    child.borrow_mut().compose(&mut sub);
                }
            }

            self.cache = Some(Rc::new(scene));
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
        Self::compose_into(&mut scene, root);
        scene
    }

    /// Composes `root` and its descendants into `scene`.
    pub fn compose_into<L: Layer + ?Sized>(scene: &mut Scene, root: &LayerHandle<L>) {
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
    dirty: bool,
    children: ChildLayers,
}

impl TransformLayer {
    pub fn new(transform: Affine) -> Self {
        Self {
            transform,
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
}

impl Layer for TransformLayer {
    fn compose(&mut self, compositor: &mut Compositor) {
        // A changed transform only re-places the children; it doesn't invalidate their cache.
        compositor.push_transform(self.transform);
        self.children.embed(compositor);
        compositor.pop_transform();

        self.dirty = false;
    }

    fn update_dirty(&mut self) -> bool {
        self.children.update_dirty() || self.dirty
    }
}

impl Container for TransformLayer {
    fn append(&mut self, child: LayerHandle) {
        self.children.append(child);
    }
}

/// A layer that applies a reduced opacity to its children.
pub struct OpacityLayer {
    alpha: f32,
    clip: PaintShape,
    dirty: bool,
    children: ChildLayers,
}

impl OpacityLayer {
    pub fn new(alpha: f32, clip: PaintShape) -> Self {
        Self {
            alpha,
            clip,
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
        compositor.push_layer(
            BlendMode::new(Mix::Normal, Compose::SrcOver),
            self.alpha,
            self.clip.clone(),
        );
        {
            self.children.embed(compositor);
        }
        compositor.pop_layer();

        self.dirty = false;
    }

    fn update_dirty(&mut self) -> bool {
        self.children.update_dirty() || self.dirty
    }
}

impl Container for OpacityLayer {
    fn append(&mut self, child: LayerHandle) {
        self.children.append(child);
    }
}

/// Groups a sequence of layers with no effect of its own.
pub struct ContainerLayer {
    children: ChildLayers,
}

impl ContainerLayer {
    pub fn new() -> Self {
        Self {
            children: ChildLayers::new(),
        }
    }

    /// Removes every child from the layer.
    pub fn clear(&mut self) {
        self.children.clear();
    }
}

impl Default for ContainerLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Layer for ContainerLayer {
    fn compose(&mut self, compositor: &mut Compositor) {
        self.children.embed(compositor);
    }

    fn update_dirty(&mut self) -> bool {
        self.children.update_dirty()
    }
}

impl Container for ContainerLayer {
    fn append(&mut self, child: LayerHandle) {
        self.children.append(child);
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

        let mut outer = ContainerLayer::new();
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

        let mut container = ContainerLayer::new();
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

    /// A change bubbles up through several clean container ancestors, rebuilding each cache, while the
    /// leaf below is composed only once.
    #[test]
    fn a_change_bubbles_through_multiple_clean_ancestors() {
        let (leaf, count, _) = CountingLayer::new();
        let transform = transform_over(leaf, Affine::translate((1.0, 0.0)));

        let mut mid = ContainerLayer::new();
        mid.append(transform.clone().into());
        let mut top = ContainerLayer::new();
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

        let mut container = ContainerLayer::new();
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
        let scene = Compositor::compose(&LayerHandle::new(ContainerLayer::new())).flatten();

        assert!(scene.is_empty());
    }

    #[test]
    fn an_empty_picture_composes_to_nothing() {
        let scene =
            Compositor::compose(&LayerHandle::new(PictureLayer::new(Scene::new()))).flatten();

        assert!(scene.is_empty());
    }
}
