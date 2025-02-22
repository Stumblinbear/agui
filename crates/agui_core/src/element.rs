use std::any::{type_name_of_val, Any};

use smallbox::SmallBox;
use typed_floats::{Positive, PositiveFinite};

use crate::{
    constraints::Constraints,
    context::{MessageCtx, UpdateCtx},
    hit_test::HitTestResult,
    offset::Offset,
    renderer::Canvas,
    size::Size,
    text_baseline::TextBaseline,
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

    size: Size,
}

impl Element {
    pub fn empty() -> Self {
        Self {
            state: ElementState::empty(),

            children: Vec::default(),

            size: Size::ZERO,
        }
    }

    pub fn new<V>(view: &V, ctx: &mut UpdateCtx) -> Self
    where
        V: MountView,
    {
        let (children, state) = view.mount(ctx);

        Self {
            state,

            children,

            size: Size::ZERO,
        }
    }

    pub const fn size(&self) -> Size {
        self.size
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
    pub fn min_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.view.min_intrinsic_width(self.element, height)
    }

    pub fn max_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.view.max_intrinsic_width(self.element, height)
    }

    pub fn min_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.view.min_intrinsic_height(self.element, width)
    }

    pub fn max_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.view.max_intrinsic_height(self.element, width)
    }

    pub fn measure(&self, constraints: Constraints) -> Size {
        self.view.measure(self.element, constraints)
    }

    pub fn measure_baseline(
        &self,
        constraints: Constraints,
        baseline: TextBaseline,
    ) -> Option<PositiveFinite<f32>> {
        self.view
            .measure_baseline(self.element, constraints, baseline)
    }

    pub fn hit_test(&self, result: &mut HitTestResult, position: Offset) -> bool {
        self.view.hit_test(self.element, result, position)
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

    pub fn min_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.view.min_intrinsic_width(self.element, height)
    }

    pub fn max_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.view.max_intrinsic_width(self.element, height)
    }

    pub fn min_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.view.min_intrinsic_height(self.element, width)
    }

    pub fn max_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.view.max_intrinsic_height(self.element, width)
    }

    pub fn measure(&self, constraints: Constraints) -> Size {
        self.view.measure(self.element, constraints)
    }

    pub fn layout(&mut self, constraints: Constraints) -> ChildLayoutRef {
        let size = self.view.layout(self.element, constraints);

        #[cfg(debug_assertions)]
        if !size.is_finite() {
            panic!(
                "{} was given an infinite size during layout. The given constraints were: {:?}",
                type_name_of_val(self.view),
                constraints
            );
        }

        #[cfg(debug_assertions)]
        if !constraints.is_satisfied_by(size) {
            panic!(
                "{} did not satisfy the given constraints. The given constraints were: {:?}, but the size was: {:?}",
                type_name_of_val(self.view),
                constraints,
                size
            );
        }

        self.element.size = size;

        ChildLayoutRef {
            element: self.element,
        }
    }

    pub fn measure_baseline(
        &self,
        constraints: Constraints,
        baseline: TextBaseline,
    ) -> Option<PositiveFinite<f32>> {
        self.view
            .measure_baseline(self.element, constraints, baseline)
    }

    pub fn distance_to_baseline(&mut self, baseline: TextBaseline) -> Option<PositiveFinite<f32>> {
        self.view.distance_to_baseline(self.element, baseline)
    }

    pub fn hit_test(&self, result: &mut HitTestResult, position: Offset) -> bool {
        self.view.hit_test(self.element, result, position)
    }

    pub fn draw(&mut self, canvas: &mut Canvas)
    where
        Child: View,
    {
        self.view.draw(self.element, canvas)
    }
}

pub struct ChildLayoutRef<'a> {
    element: &'a Element,
}

impl ChildLayoutRef<'_> {
    pub const fn size(self) -> Size {
        self.element.size()
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::VecDeque, marker::PhantomData, sync::mpsc};

    use typed_floats::{Positive, PositiveFinite};

    use crate::{
        constraints::Constraints,
        context::{MessageCtx, UpdateCtx},
        element::Element,
        hit_test::HitTestResult,
        offset::Offset,
        renderer::Canvas,
        size::Size,
        text_baseline::TextBaseline,
        view::{AsAnyView, HasIntrinsic, NoIntrinsic, Unbounded, View, ViewLayoutMarker},
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

    impl<T> ViewLayoutMarker for TestView<T> {
        type Width = Unbounded;
        type Height = Unbounded;

        type WidthIntrinsic = NoIntrinsic;
        type HeightIntrinsic = NoIntrinsic;
    }

    impl<T> View for TestView<T>
    where
        T: Default + 'static,
    {
        type State = T;

        fn mount(&self, _: &mut UpdateCtx) -> (Vec<Element>, Self::State) {
            (vec![], T::default())
        }

        fn update(&self, _: &mut Element, _: &Self, _: &mut UpdateCtx) {}

        fn message(&self, _: &mut Element, _: MessageCtx) {}

        fn min_intrinsic_width(
            &self,
            _: &Element,
            _: Positive<f32>,
        ) -> Option<PositiveFinite<f32>> {
            None
        }

        fn max_intrinsic_width(
            &self,
            _: &Element,
            _: Positive<f32>,
        ) -> Option<PositiveFinite<f32>> {
            None
        }

        fn min_intrinsic_height(
            &self,
            _: &Element,
            _: Positive<f32>,
        ) -> Option<PositiveFinite<f32>> {
            None
        }

        fn max_intrinsic_height(
            &self,
            _: &Element,
            _: Positive<f32>,
        ) -> Option<PositiveFinite<f32>> {
            None
        }

        fn measure(&self, _: &Element, _: Constraints) -> Size {
            Size::ZERO
        }

        fn layout(&self, _: &mut Element, _: Constraints) -> Size {
            Size::ZERO
        }

        fn measure_baseline(
            &self,
            _: &Element,
            _: Constraints,
            _: TextBaseline,
        ) -> Option<PositiveFinite<f32>> {
            None
        }

        fn distance_to_baseline(
            &self,
            _: &mut Element,
            _: TextBaseline,
        ) -> Option<PositiveFinite<f32>> {
            None
        }

        fn hit_test(&self, _: &Element, _: &mut HitTestResult, _: Offset) -> bool {
            false
        }

        fn draw(&self, _: &mut Element, _: &mut Canvas) {}
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
