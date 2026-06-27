use agui_core::tree::Slot;

use crate::{
    context::{CreateCtx, UpdateCtx},
    diagnostics::{Diagnostics, DiagnosticsNode, DiagnosticsNodeBuilder},
    element::{Element, MultiChildElement},
    pipeline::render_pipeline::LayoutScope,
    render_object::{
        MultiChildRenderObject, RenderChildren,
        box_layout::RenderBox,
        node::{RenderNode, RenderObjectCell, RenderObjectPtr},
    },
    widget::{Widget, any_widget::RenderBoxElement, any_widget::RenderBoxWrapper},
};

/// A statically-typed, nestable sequence of child widgets that a multi-child widget accepts. It is a widget
/// (one child), an [`Option`] of a sequence (that one or absent), a [`Vec`] of one widget type (a keyed
/// dynamic run), or a tuple of any of those. Nested sequences flatten: the leaves become one ordered run of
/// children, each its own node in the tree.
///
/// [`Renders`](Self::Renders) is the parallel render-object structure the owning render object holds. `create`
/// builds both sides and returns the renders for the parent to embed; `update` threads them back in to reconcile
/// in lockstep.
pub trait WidgetSequence {
    /// The child elements produced by [`create`](Self::create).
    type Elements: ElementSequence<Renders = Self::Renders>;

    /// The child render objects this sequence's render side produces.
    type Renders: RenderChildren;

    /// Builds every child element and its render object, returning the renders for the owning render object to
    /// hold. Ready to be mounted.
    fn create(self, ctx: &mut CreateCtx) -> (Self::Elements, Self::Renders);

    /// Reconciles `elements` and `renders` against this sequence in place, in lockstep. `layout_scope` is the
    /// owning render object's relayout boundary, marked when the reconcile changes the run structurally so a
    /// grafted child is laid out.
    ///
    /// # Safety
    /// `elements` must belong to the element at `ctx`'s position, and `renders` to its render object.
    unsafe fn update(
        self,
        ctx: &mut UpdateCtx<'_>,
        elements: &mut Self::Elements,
        renders: &mut Self::Renders,
        layout_scope: LayoutScope,
    );
}

/// The child elements of a [`WidgetSequence`]. The owning [`ChildrenElement`] delegates its mount, unmount,
/// and diagnostics to it; children are addressed in the tree by handle, so the sequence does no routing.
pub trait ElementSequence {
    /// The render objects paired with these elements.
    type Renders: RenderChildren;

    /// Mounts every child under the owning element, wiring each one's render edge into the matching slot of
    /// `renders` as it is pinned.
    ///
    /// # Safety
    /// Every slot in this sequence must belong to the element at `ctx`'s position, and `renders` must be the
    /// owning render object's matching child storage.
    unsafe fn mount(&mut self, ctx: &mut UpdateCtx<'_>, renders: &mut Self::Renders);

    /// Unmounts every child.
    ///
    /// # Safety
    /// As [`mount`](Self::mount).
    unsafe fn unmount(&mut self, ctx: &mut UpdateCtx<'_>);

    /// Threads each present child's diagnostics under `node`, in order.
    fn describe<'a>(&self, node: DiagnosticsNodeBuilder<'a>) -> DiagnosticsNodeBuilder<'a>;
}

/// The [`Element`] of a widget whose children are a [`WidgetSequence`], owning the child elements and the
/// widget's render object `R`.
pub struct ChildrenElement<L: WidgetSequence, R> {
    children: L::Elements,
    render: RenderObjectCell<R>,
}

impl<L: WidgetSequence, R> ChildrenElement<L, R> {
    /// Builds the element from `children`, then wraps the resulting render edges into the widget's render
    /// object with `build_render`. The edges are wired at mount, when the children are pinned.
    pub fn new(
        ctx: &mut CreateCtx,
        children: L,
        build_render: impl FnOnce(L::Renders) -> R,
    ) -> Self {
        let (elements, renders) = children.create(ctx);

        Self {
            children: elements,
            render: RenderObjectCell::new(build_render(renders)),
        }
    }
}

impl<L: WidgetSequence, R> ChildrenElement<L, R>
where
    R: MultiChildRenderObject<Children = L::Renders>,
{
    /// Reconciles the child elements and the render object's child edges against `children`, in lockstep.
    pub fn update(&mut self, ctx: &mut UpdateCtx<'_>, children: L) {
        let layout_scope = self.render.get().layout_scope();
        // SAFETY: `self.children` is our own sequence and the edges belong to our render object.
        unsafe {
            children.update(
                ctx,
                &mut self.children,
                self.render.get_mut().children_mut(),
                layout_scope,
            );
        };
    }
}

// SAFETY: reconciles its child list only through the cursor child operations — balanced registration, slots
// reused only on `can_update` — and resolves its render object from its own `RenderObjectCell`.
unsafe impl<L: WidgetSequence, R> Element for ChildrenElement<L, R>
where
    R: MultiChildRenderObject<Children = L::Renders>,
{
    type Render = R;

    fn render_object_mut(&mut self) -> &mut R {
        self.render.get_mut()
    }

    fn render_object_ptr(&self) -> RenderObjectPtr<R> {
        self.render.render_object_ptr()
    }

    fn mount(&mut self, ctx: &mut UpdateCtx<'_>) {
        // SAFETY: `self.children` is our own sequence, so its slots belong to us; the edges are our render
        // object's.
        unsafe {
            self.children
                .mount(ctx, self.render.get_mut().children_mut());
        };

        self.render.get_mut().attach(ctx);
    }

    fn unmount(&mut self, ctx: &mut UpdateCtx<'_>) {
        self.render.get_mut().detach(ctx);

        // SAFETY: as `mount`.
        unsafe { self.children.unmount(ctx) };
    }

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        self.children.describe(d.node_for::<Self>()).finish()
    }
}

impl<W> WidgetSequence for W
where
    W: Widget + 'static,
    W::Render: RenderBox + Sized + 'static,
    W::Element: 'static,
{
    type Elements = Slot<RenderBoxElement<W::Element>>;
    type Renders = RenderNode<dyn RenderBox>;

    fn create(
        self,
        ctx: &mut CreateCtx,
    ) -> (
        Slot<RenderBoxElement<W::Element>>,
        RenderNode<dyn RenderBox>,
    ) {
        let element = Widget::create(RenderBoxWrapper::new(self), ctx);

        (Slot::new(element), RenderNode::new(()))
    }

    unsafe fn update(
        self,
        ctx: &mut UpdateCtx<'_>,
        element: &mut Slot<RenderBoxElement<W::Element>>,
        _renders: &mut RenderNode<dyn RenderBox>,
        _layout_scope: LayoutScope,
    ) {
        // A single child is reconciled in place, never grafted, so the enclosing boundary needs no mark.
        // The element owns its render object; reused in place its address holds, so the edge stays valid.
        // SAFETY: the caller guarantees `element` belongs to the element at `ctx`.
        unsafe {
            ctx.with_child(element, |el, ctx| {
                Widget::update(RenderBoxWrapper::new(self), ctx, el);
            });
        };
    }
}

impl<E> ElementSequence for Slot<E>
where
    E: Element<Render = dyn RenderBox> + 'static,
{
    type Renders = RenderNode<dyn RenderBox>;

    unsafe fn mount(&mut self, ctx: &mut UpdateCtx<'_>, renders: &mut RenderNode<dyn RenderBox>) {
        // The child element already renders `dyn RenderBox`, so wiring its edge is an identity move.
        // SAFETY: the caller guarantees this slot belongs to the element at `ctx`.
        let mounted = unsafe { ctx.mount(self) };
        renders.set(mounted);
    }

    unsafe fn unmount(&mut self, ctx: &mut UpdateCtx<'_>) {
        // SAFETY: as `mount`.
        unsafe { ctx.unmount(self) };
    }

    fn describe<'a>(&self, node: DiagnosticsNodeBuilder<'a>) -> DiagnosticsNodeBuilder<'a> {
        node.child(|d| self.get().describe(d))
    }
}

impl<C: WidgetSequence> WidgetSequence for Option<C> {
    type Elements = Option<C::Elements>;
    type Renders = Option<C::Renders>;

    fn create(self, ctx: &mut CreateCtx) -> (Option<C::Elements>, Option<C::Renders>) {
        match self {
            Some(sequence) => {
                let (elements, renders) = sequence.create(ctx);
                (Some(elements), Some(renders))
            }
            None => (None, None),
        }
    }

    unsafe fn update(
        self,
        ctx: &mut UpdateCtx<'_>,
        elements: &mut Option<C::Elements>,
        renders: &mut Option<C::Renders>,
        layout_scope: LayoutScope,
    ) {
        if let Some(sequence) = self {
            if let (Some(present), Some(present_renders)) = (elements.as_mut(), renders.as_mut()) {
                // SAFETY: `present` is the caller's sub-sequence and `present_renders` its renders.
                unsafe { sequence.update(ctx, present, present_renders, layout_scope) };
            } else {
                // Absent to present: a child appears, so re-lay the enclosing boundary.
                ctx.mark_needs_layout(layout_scope);

                let (mut built, mut built_renders) = ctx.inflate(|ctx| sequence.create(ctx));
                // SAFETY: `built` is freshly created and owned here.
                unsafe { built.mount(ctx, &mut built_renders) };
                *elements = Some(built);
                *renders = Some(built_renders);
            }
        } else {
            if let Some(mut present) = elements.take() {
                // Present to absent: a child leaves, so re-lay the enclosing boundary.
                ctx.mark_needs_layout(layout_scope);

                // SAFETY: `present` is this sequence's own.
                unsafe { present.unmount(ctx) };
            }
            *renders = None;
        }
    }
}

impl<E: ElementSequence> ElementSequence for Option<E> {
    type Renders = Option<E::Renders>;

    unsafe fn mount(&mut self, ctx: &mut UpdateCtx<'_>, renders: &mut Option<E::Renders>) {
        if let Some(elements) = self {
            let renders = renders
                .as_mut()
                .expect("the render structure matches the elements at mount");
            // SAFETY: as the trait's contract.
            unsafe { elements.mount(ctx, renders) };
        }
    }

    unsafe fn unmount(&mut self, ctx: &mut UpdateCtx<'_>) {
        if let Some(elements) = self {
            // SAFETY: as the trait's contract.
            unsafe { elements.unmount(ctx) };
        }
    }

    fn describe<'a>(&self, node: DiagnosticsNodeBuilder<'a>) -> DiagnosticsNodeBuilder<'a> {
        match self.as_ref() {
            Some(elements) => elements.describe(node),
            None => node,
        }
    }
}

impl<W> WidgetSequence for Vec<W>
where
    W: Widget + 'static,
    W::Render: RenderBox + Sized + 'static,
    W::Element: 'static,
{
    type Elements = MultiChildElement<RenderBoxElement<W::Element>>;
    type Renders = Vec<RenderNode<dyn RenderBox>>;

    fn create(
        self,
        ctx: &mut CreateCtx,
    ) -> (
        MultiChildElement<RenderBoxElement<W::Element>>,
        Vec<RenderNode<dyn RenderBox>>,
    ) {
        MultiChildElement::new(ctx, self.into_iter().map(RenderBoxWrapper::new).collect())
    }

    unsafe fn update(
        self,
        ctx: &mut UpdateCtx<'_>,
        elements: &mut MultiChildElement<RenderBoxElement<W::Element>>,
        renders: &mut Vec<RenderNode<dyn RenderBox>>,
        layout_scope: LayoutScope,
    ) {
        elements.update(
            ctx,
            self.into_iter().map(RenderBoxWrapper::new).collect(),
            renders,
            layout_scope,
        );
    }
}

impl<E> ElementSequence for MultiChildElement<E>
where
    E: Element<Render = dyn RenderBox> + 'static,
{
    type Renders = Vec<RenderNode<dyn RenderBox>>;

    unsafe fn mount(
        &mut self,
        ctx: &mut UpdateCtx<'_>,
        renders: &mut Vec<RenderNode<dyn RenderBox>>,
    ) {
        self.mount(ctx, renders);
    }

    unsafe fn unmount(&mut self, ctx: &mut UpdateCtx<'_>) {
        self.unmount(ctx);
    }

    fn describe<'a>(&self, node: DiagnosticsNodeBuilder<'a>) -> DiagnosticsNodeBuilder<'a> {
        self.describe_children(node)
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
            fn create(self, ctx: &mut CreateCtx) -> (Self::Elements, Self::Renders) {
                let ($($T,)+) = self;
                $(let $T = $T.create(ctx);)+
                (($($T.0,)+), ($($T.1,)+))
            }

            #[allow(non_snake_case)]
            unsafe fn update(
                self,
                ctx: &mut UpdateCtx<'_>,
                elements: &mut Self::Elements,
                renders: &mut Self::Renders,
                layout_scope: LayoutScope,
            ) {
                let ($($T,)+) = self;
                // SAFETY: each component belongs to the element at `ctx`, its renders to the render object.
                unsafe { $($T.update(ctx, &mut elements.$i, &mut renders.$i, layout_scope);)+ }
            }
        }

        impl<P, $($T,)+> ElementSequence for ($($T,)+)
        where
            $($T: ElementSequence, $T::Renders: RenderChildren<ParentData = P>,)+
        {
            type Renders = ($($T::Renders,)+);

            unsafe fn mount(&mut self, ctx: &mut UpdateCtx<'_>, renders: &mut Self::Renders) {
                // SAFETY: each component belongs to the element at `ctx`, its renders to the render object.
                unsafe { $(self.$i.mount(ctx, &mut renders.$i);)+ }
            }

            unsafe fn unmount(&mut self, ctx: &mut UpdateCtx<'_>) {
                // SAFETY: as `mount`.
                unsafe { $(self.$i.unmount(ctx);)+ }
            }

            #[allow(non_snake_case)]
            fn describe<'a>(&self, node: DiagnosticsNodeBuilder<'a>) -> DiagnosticsNodeBuilder<'a> {
                $(let node = self.$i.describe(node);)+
                node
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
