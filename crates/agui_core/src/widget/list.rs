use std::marker::PhantomData;

use crate::{
    context::{Dispatch, UpdateCtx},
    element::{Element, MultiChildElement, RoutingId, RoutingPath, node::ElementNode},
    render_object::{
        MultiChildRenderObject, RenderChildren, box_layout::RenderBox, node::RenderNode,
    },
    widget::Widget,
};

/// A statically-typed, nestable sequence of child widgets that a multi-child widget accepts. It is a
/// widget (one child), an [`Option`] of a sequence (that one or absent), a [`Vec`] of one widget type
/// (a keyed dynamic run), or a tuple of any of those. Nested sequences flatten: the leaves of the
/// whole structure become one ordered run of children.
///
/// Building a sequence produces two parallel structures of the same shape: the [child elements](ElementSequence),
/// held by a [`ChildrenElement`], and their [render objects](RenderChildren), held by the widget's
/// render object. Each tuple and `Vec` level pushes a routing id per child, so a leaf is addressed by
/// the path of those ids; a widget and an `Option` add none.
pub trait WidgetSequence {
    /// The child elements produced by [`build`](Self::build).
    type Elements: ElementSequence<Renders = Self::Renders>;

    /// The child render objects produced by [`build`](Self::build).
    type Renders: RenderChildren;

    /// Builds every child, pushing a routing id per child at each tuple and `Vec` level.
    fn build(self, ctx: &mut UpdateCtx) -> (Self::Elements, Self::Renders);

    /// Reconciles `elements` and `renders` against this sequence in place.
    fn rebuild(
        self,
        elements: &mut Self::Elements,
        renders: &mut Self::Renders,
        ctx: &mut UpdateCtx,
    );
}

/// The child elements of a [`WidgetSequence`], paired with their [render objects](Self::Renders) and
/// routed by the same path of ids the sequence pushed at build.
pub trait ElementSequence {
    /// The render objects paired with these elements.
    type Renders: RenderChildren;

    /// Routes `action` along `path` to the addressed child, threading its render object. A dispatch
    /// to an absent sub-sequence is dropped.
    fn dispatch(&mut self, renders: &mut Self::Renders, path: &RoutingPath, action: Dispatch);
}

/// The [`Element`] of a widget whose children are a [`WidgetSequence`], holding the child elements and
/// threading the widget's render `R` to reach each child's render object on dispatch.
pub struct ChildrenElement<L: WidgetSequence, R: ?Sized> {
    children: L::Elements,
    _render: PhantomData<fn() -> R>,
}

impl<L: WidgetSequence, R> ChildrenElement<L, R> {
    /// Builds the element and every child's render object from `children`.
    pub fn new(children: L, ctx: &mut UpdateCtx) -> (Self, L::Renders) {
        let (elements, renders) = children.build(ctx);

        (
            Self {
                children: elements,
                _render: PhantomData,
            },
            renders,
        )
    }

    /// Reconciles the child elements and `render`'s child render objects against `children`.
    pub fn update(&mut self, children: L, render: &mut R, ctx: &mut UpdateCtx)
    where
        R: MultiChildRenderObject<Children = L::Renders>,
    {
        children.rebuild(&mut self.children, render.children_mut(), ctx);
    }
}

impl<L: WidgetSequence, R> Element for ChildrenElement<L, R>
where
    R: MultiChildRenderObject<Children = L::Renders>,
{
    type Render = R;

    fn dispatch(&mut self, render: &mut R, path: &RoutingPath, action: Dispatch) {
        self.children.dispatch(render.children_mut(), path, action);
    }
}

impl<W> WidgetSequence for W
where
    W: Widget + 'static,
    W::Render: RenderBox,
{
    type Elements = ElementNode<W::Element>;
    type Renders = RenderNode<W::Render>;

    fn build(self, ctx: &mut UpdateCtx) -> (ElementNode<W::Element>, RenderNode<W::Render>) {
        let (element, render) = self.create(ctx);
        (ElementNode::new(element), RenderNode::new(render))
    }

    fn rebuild(
        self,
        element: &mut ElementNode<W::Element>,
        render: &mut RenderNode<W::Render>,
        ctx: &mut UpdateCtx,
    ) {
        self.update(&mut element.element, &mut render.object, ctx);
    }
}

impl<E> ElementSequence for ElementNode<E>
where
    E: Element,
    E::Render: RenderBox + Sized,
{
    type Renders = RenderNode<E::Render>;

    fn dispatch(
        &mut self,
        render: &mut RenderNode<E::Render>,
        path: &RoutingPath,
        action: Dispatch,
    ) {
        self.element.dispatch(&mut render.object, path, action);
    }
}

impl<C: WidgetSequence> WidgetSequence for Option<C> {
    type Elements = Option<C::Elements>;
    type Renders = Option<C::Renders>;

    fn build(self, ctx: &mut UpdateCtx) -> (Option<C::Elements>, Option<C::Renders>) {
        match self {
            Some(sequence) => {
                let (elements, renders) = sequence.build(ctx);
                (Some(elements), Some(renders))
            }
            None => (None, None),
        }
    }

    fn rebuild(
        self,
        elements: &mut Option<C::Elements>,
        renders: &mut Option<C::Renders>,
        ctx: &mut UpdateCtx,
    ) {
        if let Some(sequence) = self {
            if let (Some(elements), Some(renders)) = (elements.as_mut(), renders.as_mut()) {
                sequence.rebuild(elements, renders, ctx);
            } else {
                let (e, mut r) = sequence.build(ctx);
                r.for_each_mut(&mut |child| ctx.mount(&mut child.object));
                *elements = Some(e);
                *renders = Some(r);
            }
        } else {
            *elements = None;
            *renders = None;
        }
    }
}

impl<E: ElementSequence> ElementSequence for Option<E> {
    type Renders = Option<E::Renders>;

    fn dispatch(&mut self, renders: &mut Option<E::Renders>, path: &RoutingPath, action: Dispatch) {
        if let (Some(elements), Some(renders)) = (self.as_mut(), renders.as_mut()) {
            elements.dispatch(renders, path, action);
        }
    }
}

impl<W> WidgetSequence for Vec<W>
where
    W: Widget + 'static,
    W::Render: RenderBox,
{
    type Elements = MultiChildElement<W::Element>;
    type Renders = Vec<RenderNode<W::Render>>;

    fn build(self, ctx: &mut UpdateCtx) -> (Self::Elements, Self::Renders) {
        MultiChildElement::build(self, ctx)
    }

    fn rebuild(
        self,
        elements: &mut Self::Elements,
        renders: &mut Self::Renders,
        ctx: &mut UpdateCtx,
    ) {
        let old_renders = std::mem::take(renders);
        *renders = elements.reconcile(self, old_renders, ctx);
    }
}

impl<E> ElementSequence for MultiChildElement<E>
where
    E: Element,
    E::Render: RenderBox + Sized,
{
    type Renders = Vec<RenderNode<E::Render>>;

    fn dispatch(
        &mut self,
        renders: &mut Vec<RenderNode<E::Render>>,
        path: &RoutingPath,
        action: Dispatch,
    ) {
        MultiChildElement::dispatch(self, renders, path, action);
    }
}

macro_rules! impl_sequence_tuple {
    ($($T:ident => $i:tt),+) => {
        impl<P, $($T,)+> WidgetSequence for ($($T,)+)
        where
            $($T: WidgetSequence, $T::Renders: RenderChildren<ParentData = P>,)+
        {
            type Elements = ($($T::Elements,)+);
            type Renders = ($($T::Renders,)+);

            #[allow(non_snake_case)]
            fn build(self, ctx: &mut UpdateCtx) -> (Self::Elements, Self::Renders) {
                let ($($T,)+) = self;
                $(
                    let $T = ctx.with_routing_id(RoutingId::new(u32::try_from($i).unwrap()), |ctx| $T.build(ctx));
                )+
                (($($T.0,)+), ($($T.1,)+))
            }

            #[allow(non_snake_case)]
            fn rebuild(
                self,
                elements: &mut Self::Elements,
                renders: &mut Self::Renders,
                ctx: &mut UpdateCtx,
            ) {
                let ($($T,)+) = self;
                $(
                    ctx.with_routing_id(RoutingId::new(u32::try_from($i).unwrap()), |ctx| {
                        $T.rebuild(&mut elements.$i, &mut renders.$i, ctx);
                    });
                )+
            }
        }

        impl<P, $($T,)+> ElementSequence for ($($T,)+)
        where
            $($T: ElementSequence, $T::Renders: RenderChildren<ParentData = P>,)+
        {
            type Renders = ($($T::Renders,)+);

            fn dispatch(
                &mut self,
                renders: &mut Self::Renders,
                path: &RoutingPath,
                action: Dispatch,
            ) {
                let Some((head, rest)) = path.decode() else {
                    unreachable!("tuple sequence addresses one of its slots");
                };

                match head.get() as usize {
                    $($i => self.$i.dispatch(&mut renders.$i, rest, action),)+
                    _ => unreachable!("routing id addresses no tuple slot"),
                }
            }
        }
    };
}

impl_sequence_tuple!(A => 0);
impl_sequence_tuple!(A => 0, B => 1);
impl_sequence_tuple!(A => 0, B => 1, C => 2);
impl_sequence_tuple!(A => 0, B => 1, C => 2, D => 3);
impl_sequence_tuple!(A => 0, B => 1, C => 2, D => 3, E => 4);
impl_sequence_tuple!(A => 0, B => 1, C => 2, D => 3, E => 4, F => 5);
impl_sequence_tuple!(A => 0, B => 1, C => 2, D => 3, E => 4, F => 5, G => 6);
impl_sequence_tuple!(A => 0, B => 1, C => 2, D => 3, E => 4, F => 5, G => 6, H => 7);
impl_sequence_tuple!(A => 0, B => 1, C => 2, D => 3, E => 4, F => 5, G => 6, H => 7, I => 8);
impl_sequence_tuple!(A => 0, B => 1, C => 2, D => 3, E => 4, F => 5, G => 6, H => 7, I => 8, J => 9);

#[cfg(test)]
mod tests {
    use std::{any::Any, cell::Cell, marker::PhantomData, rc::Rc};

    use typed_floats::{Positive, PositiveFinite};

    use crate::{
        context::{Dispatch, LayoutCtx, MessageCtx, MountCtx, PaintCtx, UpdateCtx},
        element::{Element, LeafElement, RoutingId, RoutingPath},
        geometry::{Offset, Size},
        input::hit_test::{HitTest, HitTestResult},
        render_object::{
            MultiChildRenderObject, RenderChildren, RenderObject,
            box_layout::{BoxConstraints, RenderBox},
        },
        test_harness::TestCtx,
        text::TextBaseline,
        widget::Widget,
    };

    use super::{ChildrenElement, WidgetSequence};

    /// A leaf widget distinguished by its type parameter, recording messages it receives.
    struct Probe<T> {
        id: u32,
        mounts: Rc<Cell<usize>>,
        sink: Rc<Cell<Option<(u32, u32)>>>,
        _type: PhantomData<fn() -> T>,
    }

    struct ProbeElement {
        id: u32,
        sink: Rc<Cell<Option<(u32, u32)>>>,
    }

    impl Element for ProbeElement {
        type Render = ();

        fn dispatch(&mut self, (): &mut (), path: &RoutingPath, action: Dispatch) {
            assert!(path.is_empty(), "probe is a leaf");
            if let Dispatch::Message(ctx) = action {
                self.sink.set(Some((self.id, ctx.consume::<u32>())));
            }
        }
    }

    impl<T: 'static> Widget for Probe<T> {
        type Element = ProbeElement;
        type Render = ();

        fn create(self, _: &mut UpdateCtx) -> (Self::Element, Self::Render) {
            self.mounts.set(self.mounts.get() + 1);
            (
                ProbeElement {
                    id: self.id,
                    sink: self.sink,
                },
                (),
            )
        }

        fn update(self, element: &mut Self::Element, (): &mut Self::Render, _: &mut UpdateCtx) {
            element.id = self.id;
        }
    }

    struct Container<S> {
        children: S,
    }

    impl<S: RenderChildren> MultiChildRenderObject for Container<S> {
        type Children = S;

        fn children_mut(&mut self) -> &mut S {
            &mut self.children
        }
    }

    fn probe<T: 'static>(
        id: u32,
        mounts: &Rc<Cell<usize>>,
        sink: &Rc<Cell<Option<(u32, u32)>>>,
    ) -> Probe<T> {
        Probe {
            id,
            mounts: Rc::clone(mounts),
            sink: Rc::clone(sink),
            _type: PhantomData,
        }
    }

    fn build<L: WidgetSequence>(children: L) -> L::Renders {
        TestCtx::new().run(|ctx| children.build(ctx).1)
    }

    #[test]
    fn heterogeneous_tuple_counts_each_child() {
        let mounts = Rc::new(Cell::new(0));
        let sink = Rc::new(Cell::new(None));

        let renders = build((
            probe::<u8>(10, &mounts, &sink),
            probe::<u16>(20, &mounts, &sink),
            probe::<u32>(30, &mounts, &sink),
        ));

        assert_eq!(mounts.get(), 3);
        assert_eq!(renders.len(), 3);
    }

    #[test]
    fn vec_in_tuple_flattens_in_order() {
        let mounts = Rc::new(Cell::new(0));
        let sink = Rc::new(Cell::new(None));

        // (single, vec of three, single) -> five flattened children.
        let renders = build((
            probe::<u8>(1, &mounts, &sink),
            vec![
                probe::<u16>(2, &mounts, &sink),
                probe::<u16>(3, &mounts, &sink),
                probe::<u16>(4, &mounts, &sink),
            ],
            probe::<u32>(5, &mounts, &sink),
        ));

        assert_eq!(mounts.get(), 5);
        assert_eq!(renders.len(), 5, "vec flattened in place");

        // Collect the flattened order through for_each.
        let mut order = Vec::new();
        renders.for_each(&mut |_| order.push(()));
        assert_eq!(order.len(), 5);
    }

    #[test]
    fn nested_vec_in_tuple_in_option_in_tuple() {
        let mounts = Rc::new(Cell::new(0));
        let sink = Rc::new(Cell::new(None));

        // (a, Some((b, vec![c, d]))) -> 1 + (1 + 2) = 4 children.
        let renders = build((
            probe::<u8>(1, &mounts, &sink),
            Some((
                probe::<u16>(2, &mounts, &sink),
                vec![
                    probe::<u32>(3, &mounts, &sink),
                    probe::<u32>(4, &mounts, &sink),
                ],
            )),
        ));

        assert_eq!(mounts.get(), 4);
        assert_eq!(renders.len(), 4);
    }

    #[test]
    fn absent_option_contributes_no_children() {
        let mounts = Rc::new(Cell::new(0));
        let sink = Rc::new(Cell::new(None));

        let renders = build((
            probe::<u8>(1, &mounts, &sink),
            Option::<Vec<Probe<u16>>>::None,
            probe::<u32>(2, &mounts, &sink),
        ));

        assert_eq!(mounts.get(), 2, "the absent option built nothing");
        assert_eq!(renders.len(), 2, "absent option flattens to nothing");
    }

    #[test]
    fn dispatch_routes_a_structural_path_to_a_nested_child() {
        let mounts = Rc::new(Cell::new(0));
        let sink = Rc::new(Cell::new(None));

        // (a, vec![b, c]) — c is tuple slot 1, vec index 1, so path [1, 1].
        let children = (
            probe::<u8>(1, &mounts, &sink),
            vec![
                probe::<u16>(2, &mounts, &sink),
                probe::<u16>(3, &mounts, &sink),
            ],
        );

        let (mut element, renders) = TestCtx::new().run(|ctx| ChildrenElement::new(children, ctx));
        let mut container = Container { children: renders };

        let mut msg = MessageCtx::new(Box::new(77_u32) as Box<dyn Any>);
        element.dispatch(
            &mut container,
            RoutingPath::new(&RoutingId::encode_path([
                RoutingId::new(1),
                RoutingId::new(1),
            ])),
            Dispatch::Message(&mut msg),
        );

        assert_eq!(sink.get(), Some((3, 77)), "the nested child received it");
    }

    #[test]
    fn rebuild_grows_a_nested_vec_without_remounting_survivors() {
        let mounts = Rc::new(Cell::new(0));
        let sink = Rc::new(Cell::new(None));

        let children = (
            probe::<u8>(1, &mounts, &sink),
            vec![probe::<u16>(2, &mounts, &sink)],
        );

        let (mut element, renders) = TestCtx::new().run(|ctx| ChildrenElement::new(children, ctx));
        let mut container = Container { children: renders };
        assert_eq!(mounts.get(), 2);

        let next = (
            probe::<u8>(1, &mounts, &sink),
            vec![
                probe::<u16>(2, &mounts, &sink),
                probe::<u16>(3, &mounts, &sink),
            ],
        );
        TestCtx::new().run(|ctx| element.update(next, &mut container, ctx));
        assert_eq!(
            mounts.get(),
            3,
            "the leading widget and vec child reused, one appended"
        );
        assert_eq!(container.children.len(), 3);
    }

    /// A render object that records its mounts and is otherwise layout-inert.
    struct MountSpy {
        mounts: Rc<Cell<usize>>,
    }

    impl RenderObject for MountSpy {
        fn mount(&mut self, _: &mut MountCtx) {
            self.mounts.set(self.mounts.get() + 1);
        }

        fn unmount(&mut self, _: &mut MountCtx) {}

        fn update_compositing_bits(&mut self) -> bool {
            false
        }
    }

    impl RenderBox for MountSpy {
        fn min_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
            ().min_intrinsic_width(height)
        }

        fn max_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
            ().max_intrinsic_width(height)
        }

        fn min_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
            ().min_intrinsic_height(width)
        }

        fn max_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
            ().max_intrinsic_height(width)
        }

        fn measure(&self, constraints: BoxConstraints) -> Size {
            constraints.smallest()
        }

        fn layout(&mut self, _: &mut LayoutCtx, constraints: BoxConstraints) -> Size {
            constraints.smallest()
        }

        fn measure_baseline(
            &self,
            _: BoxConstraints,
            _: TextBaseline,
        ) -> Option<PositiveFinite<f32>> {
            None
        }

        fn distance_to_baseline(&mut self, _: TextBaseline) -> Option<PositiveFinite<f32>> {
            None
        }

        fn hit_test(&self, _: &mut HitTestResult, _: Offset) -> HitTest {
            HitTest::Pass
        }

        fn paint(&mut self, _: &mut PaintCtx, _: Offset) {}
    }

    /// A leaf widget whose render object is a [`MountSpy`].
    struct SpyWidget {
        mounts: Rc<Cell<usize>>,
    }

    impl Widget for SpyWidget {
        type Element = LeafElement<MountSpy>;
        type Render = MountSpy;

        fn create(self, _: &mut UpdateCtx) -> (Self::Element, Self::Render) {
            (
                LeafElement::new(),
                MountSpy {
                    mounts: self.mounts,
                },
            )
        }

        fn update(self, _: &mut Self::Element, _: &mut Self::Render, _: &mut UpdateCtx) {}
    }

    #[test]
    fn option_toggled_in_mounts_the_grafted_subtree() {
        let mounts = Rc::new(Cell::new(0));
        let sink = Rc::new(Cell::new(None));
        let spy_mounts = Rc::new(Cell::new(0));

        let children = (probe::<u8>(1, &mounts, &sink), Option::<SpyWidget>::None);
        let (mut element, renders) = TestCtx::new().run(|ctx| ChildrenElement::new(children, ctx));
        let mut container = Container { children: renders };

        // The sequence built under the absent option is grafted onto an already-built tree, so the
        // rebuild must mount it the way a child created by a keyed reconcile is mounted.
        let next = (
            probe::<u8>(1, &mounts, &sink),
            Some(SpyWidget {
                mounts: Rc::clone(&spy_mounts),
            }),
        );
        TestCtx::new().run(|ctx| element.update(next, &mut container, ctx));

        assert_eq!(container.children.len(), 2);
        assert_eq!(spy_mounts.get(), 1, "the toggled-in subtree was mounted");
    }
}
