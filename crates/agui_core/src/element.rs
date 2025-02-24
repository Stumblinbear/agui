use std::any::Any;

use smallbox::SmallBox;

use crate::{
    context::{MessageCtx, UpdateCtx},
    view::{MountView, View},
    view_id::ViewId,
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

    pub fn downcast_ref<V>(&self) -> &<V as View>::State
    where
        V: View,
        V::State: Any,
    {
        self.0.downcast_ref().expect("node state downcast failed")
    }

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

    pub fn child<'a, Child>(&'a self, idx: u16, view: &'a Child) -> ElementRef<'a, Child>
    where
        Child: View,
    {
        self.children[idx as usize].as_ref(ViewId::new(idx), view)
    }

    pub fn child_mut<'a, Child>(&'a mut self, idx: u16, view: &'a Child) -> ElementMut<'a, Child>
    where
        Child: View,
    {
        self.children[idx as usize].as_mut(ViewId::new(idx), view)
    }

    pub fn as_ref<'a, V>(&'a self, view_id: ViewId, view: &'a V) -> ElementRef<'a, V>
    where
        V: View,
    {
        ElementRef {
            view_id,
            element: self,
            view,
        }
    }

    pub fn as_mut<'a, V>(&'a mut self, view_id: ViewId, view: &'a V) -> ElementMut<'a, V>
    where
        V: View,
    {
        ElementMut {
            view_id,
            element: self,
            view,
        }
    }
}

pub struct ElementRef<'a, Child> {
    #[allow(dead_code)]
    view_id: ViewId,
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
    view_id: ViewId,
    element: &'a mut Element,
    view: &'a Child,
}

impl<Child> ElementMut<'_, Child>
where
    Child: View,
{
    pub fn update(&mut self, old: &Child, ctx: &mut UpdateCtx) {
        ctx.with_view(self.view_id, |ctx| {
            if self.view.is_similar(&self.element.state) {
                self.view.update(self.element, old, ctx)
            } else {
                *self.element = Element::new(self.view, ctx)
            }
        })
    }

    pub fn message(self, ctx: MessageCtx) {
        self.view.message(self.element, ctx)
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
    use std::{collections::VecDeque, marker::PhantomData, sync::mpsc};

    use crate::{
        context::{MessageCtx, UpdateCtx},
        element::Element,
        render_object::RenderLeaf,
        view::{AsAnyView, View},
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

        fn message(&self, _: &mut Element, _: MessageCtx) {}

        fn create_render_object(&self, _: &Element) -> Self::Render {
            RenderLeaf::default()
        }

        fn update_render_object(&self, _: &Element, _: &mut Self::Render) {}
    }

    #[test]
    fn unit_state_is_inline() {
        let (tx, _) = mpsc::channel();
        let mut path = VecDeque::new();
        let mut update_ctx = UpdateCtx::new(&tx, &mut path);

        let view = TestView::<()>::default();

        let element = Element::new(&view, &mut update_ctx);
        assert!(
            !element.state.is_heap(),
            "concrete View should result in an inline state"
        );

        let element = Element::new(&view.into_boxed_view(), &mut update_ctx);
        assert!(
            !element.state.is_heap(),
            "dyn View should result in an inline state"
        );
    }

    #[test]
    fn small_states_are_inline() {
        let (tx, _) = mpsc::channel();
        let mut path = VecDeque::new();
        let mut update_ctx = UpdateCtx::new(&tx, &mut path);

        let view = TestView::<u16>::default();

        let element = Element::new(&view, &mut update_ctx);
        assert!(
            !element.state.is_heap(),
            "concrete View should result in an inline state"
        );

        let element = Element::new(&view.into_boxed_view(), &mut update_ctx);
        assert!(
            !element.state.is_heap(),
            "dyn View should result in an inline state"
        );
    }

    #[test]
    fn large_states_are_heaped() {
        let (tx, _) = mpsc::channel();
        let mut path = VecDeque::new();
        let mut update_ctx = UpdateCtx::new(&tx, &mut path);

        let view = TestView::<[u64; 16]>::default();

        let element = Element::new(&view.into_boxed_view(), &mut update_ctx);
        assert!(
            element.state.is_heap(),
            "dyn View should result in a heaped state"
        );
    }
}
