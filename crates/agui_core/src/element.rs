use std::any::Any;

use smallbox::SmallBox;

use crate::{
    context::{Dispatch, MessageCtx, UpdateCtx},
    routing_id::RoutingId,
    view::{MountView, View},
};

pub struct ElementState(SmallBox<dyn Any, smallbox::space::S2>);

impl ElementState {
    pub fn empty() -> Self {
        Self(smallbox::smallbox!(()))
    }

    pub fn new<S>(state: S) -> Self
    where
        S: Any,
    {
        Self(smallbox::smallbox!(state))
    }

    pub fn is_heap(&self) -> bool {
        self.0.is_heap()
    }

    #[track_caller]
    pub fn downcast_ref<V>(&self) -> &<V as View>::State
    where
        V: View,
        V::State: Any,
    {
        self.0.downcast_ref().expect("node state downcast failed")
    }

    #[track_caller]
    pub fn downcast_mut<V>(&mut self) -> &mut <V as View>::State
    where
        V: View,
        V::State: Any,
    {
        self.0.downcast_mut().expect("node state downcast failed")
    }
}

pub struct Element {
    pub state: ElementState,

    pub children: Vec<Element>,
}

impl Element {
    pub fn empty() -> Self {
        Self {
            state: ElementState::empty(),

            children: Vec::default(),
        }
    }

    pub fn new<V>(view: &V, ctx: &mut UpdateCtx) -> Self
    where
        V: MountView,
    {
        let (children, state) = view.mount(ctx);

        Self { state, children }
    }

    pub fn child<'a, Child>(&'a self, idx: usize, view: &'a Child) -> ElementRef<'a, Child>
    where
        Child: View,
    {
        debug_assert!(self.children.len() > idx, "child index out of bounds");

        self.children[idx].as_ref(view)
    }

    pub fn child_mut<'a, Child>(&'a mut self, idx: usize, view: &'a Child) -> ElementMut<'a, Child>
    where
        Child: View,
    {
        debug_assert!(self.children.len() > idx, "child index out of bounds");

        self.children[idx].as_mut(view)
    }

    pub fn as_ref<'a, V>(&'a self, view: &'a V) -> ElementRef<'a, V>
    where
        V: View,
    {
        ElementRef {
            element: self,
            view,
        }
    }

    pub fn as_mut<'a, V>(&'a mut self, view: &'a V) -> ElementMut<'a, V>
    where
        V: View,
    {
        ElementMut {
            element: self,
            view,
        }
    }

    /// Invoke the leaf method on `view` matching the [`Dispatch`] variant. Called by a
    /// view's `dispatch` body when the routing path is empty (i.e. this element is
    /// the destination).
    pub fn dispatch<V>(&mut self, view: &V, action: Dispatch)
    where
        V: View,
    {
        match action {
            Dispatch::Message(msg) => view.message(self, msg),
            Dispatch::Rebuild(ctx) => view.rebuild(self, ctx),
        }
    }
}

pub struct ElementRef<'a, Child> {
    element: &'a Element,
    view: &'a Child,
}

impl<Child> ElementRef<'_, Child>
where
    Child: View,
{
    pub fn create_render_object(&self) -> Child::Render {
        self.view.create_render_object(self.element)
    }

    pub fn update_render_object(&self, render_object: &mut Child::Render) {
        self.view.update_render_object(self.element, render_object)
    }
}

pub struct ElementMut<'a, Child> {
    element: &'a mut Element,
    view: &'a Child,
}

impl<Child> ElementMut<'_, Child>
where
    Child: View,
{
    pub fn update(self, new_view: &Child, ctx: &mut UpdateCtx) {
        new_view.update(self.element, self.view, ctx)
    }

    pub fn rebuild(self, ctx: &mut UpdateCtx) {
        self.view.rebuild(self.element, ctx)
    }

    pub fn message(self, ctx: &mut MessageCtx) {
        self.view.message(self.element, ctx)
    }

    pub fn dispatch(self, path: &[RoutingId], action: Dispatch) {
        self.view.dispatch(self.element, path, action)
    }

    pub fn create_render_object(&self) -> Child::Render {
        self.view.create_render_object(self.element)
    }

    pub fn update_render_object(&self, render_object: &mut Child::Render) {
        self.view.update_render_object(self.element, render_object)
    }
}

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;

    use crate::{
        context::{Dispatch, MessageCtx, UpdateCtx},
        element::Element,
        render_object::RenderLeaf,
        routing_id::RoutingId,
        test_harness::TestHarness,
        view::View,
    };

    struct TestView<T> {
        _phantom: PhantomData<T>,
    }

    impl<T> Default for TestView<T> {
        fn default() -> Self {
            Self {
                _phantom: PhantomData,
            }
        }
    }

    impl<T> View for TestView<T>
    where
        T: Default + 'static,
    {
        type Render = RenderLeaf;

        type State = T;

        fn mount(&self, _: &mut UpdateCtx) -> (Vec<Element>, Self::State) {
            (vec![], T::default())
        }

        fn update(&self, _: &mut Element, _: &Self, _: &mut UpdateCtx) {}

        fn rebuild(&self, _: &mut Element, _: &mut UpdateCtx) {}

        fn message(&self, _: &mut Element, _: &mut MessageCtx) {}

        fn dispatch(&self, element: &mut Element, path: &[RoutingId], action: Dispatch) {
            debug_assert!(path.is_empty(), "leaf view has no children to route to");
            element.dispatch(self, action);
        }

        fn create_render_object(&self, _: &Element) -> Self::Render {
            RenderLeaf::default()
        }

        fn update_render_object(&self, _: &Element, _: &mut Self::Render) {}
    }

    #[test]
    fn unit_state_is_inline() {
        let view = TestView::<()>::default();

        let harness = TestHarness::mount(&view);

        assert!(
            !harness.root.state.is_heap(),
            "concrete View should result in an inline state"
        );
    }

    #[test]
    fn small_states_are_inline() {
        let view = TestView::<u16>::default();

        let harness = TestHarness::mount(&view);

        assert!(
            !harness.root.state.is_heap(),
            "concrete View should result in an inline state"
        );
    }

    #[test]
    fn large_states_are_heaped() {
        let view = TestView::<[u64; 16]>::default();

        let harness = TestHarness::mount(&view);

        assert!(
            harness.root.state.is_heap(),
            "should result in a heaped state"
        );
    }
}
