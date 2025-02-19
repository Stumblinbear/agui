use std::any::{type_name_of_val, Any};

use typed_floats::{Positive, PositiveFinite};

use crate::{
    constraints::Constraints,
    context::{MessageCtx, UpdateCtx},
    hit_test::HitTestResult,
    offset::Offset,
    size::Size,
    text_baseline::TextBaseline,
    view::{ViewDraw, ViewLayout, ViewLifecycle},
    view_id::ViewId,
};

pub struct ElementState(Option<Box<dyn Any>>);

impl ElementState {
    pub fn none() -> Self {
        Self(None)
    }

    pub fn new<T>(value: T) -> Self
    where
        T: Any,
    {
        Self(Some(Box::new(value)))
    }

    pub fn as_ref(&self) -> Option<&dyn Any> {
        self.0.as_deref()
    }

    pub fn as_mut(&mut self) -> Option<&mut dyn Any> {
        self.0.as_deref_mut()
    }
}

pub struct Element {
    state: ElementState,

    pub children: Vec<Element>,

    pub(crate) size: Size,
}

impl Element {
    pub fn empty() -> Self {
        Self {
            state: ElementState::none(),

            children: Vec::new(),

            size: Size::ZERO,
        }
    }

    pub fn new(view: &impl ViewLifecycle) -> Self {
        Self {
            state: view.state(),

            children: view.children(),

            size: Size::ZERO,
        }
    }

    pub fn update(&mut self, view: &impl ViewLifecycle) {
        // TODO(trevin): diff the tree

        self.state = view.state();
        self.children = view.children();
    }

    pub fn state<T>(&self) -> &T
    where
        T: Any,
    {
        self.state
            .as_ref()
            .expect("no state")
            .downcast_ref()
            .expect("node state downcast failed")
    }

    pub fn state_mut<T>(&mut self) -> &mut T
    where
        T: Any,
    {
        self.state
            .as_mut()
            .expect("no state")
            .downcast_mut()
            .expect("node state downcast failed")
    }

    pub fn child<'a, Child>(&'a self, idx: u16, view: &'a Child) -> ElementRef<'a, Child> {
        ElementRef {
            element: &self.children[idx as usize],
            view,
        }
    }

    pub fn child_mut<'a, Child>(&'a mut self, idx: u16, view: &'a Child) -> ElementMut<'a, Child> {
        ElementMut {
            view_id: ViewId::new(idx),
            element: &mut self.children[idx as usize],
            view,
        }
    }

    pub const fn size(&self) -> Size {
        self.size
    }
}

pub struct ElementRef<'a, Child> {
    element: &'a Element,
    view: &'a Child,
}

impl<Child> ElementRef<'_, Child>
where
    Child: ViewLayout,
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
    Child: ViewLifecycle,
{
    pub fn update(&mut self, mut ctx: UpdateCtx) {
        ctx.with_view(self.view_id, |ctx| self.view.update(self.element, ctx))
    }

    pub fn message(self, ctx: MessageCtx) {
        self.view.message(self.element, ctx)
    }
}

impl<Child> ElementMut<'_, Child>
where
    Child: ViewLayout,
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
}

impl<Child> ElementMut<'_, Child> {
    pub fn draw<Renderer>(&mut self, renderer: &mut Renderer)
    where
        Child: ViewDraw<Renderer>,
    {
        self.view.draw(self.element, renderer)
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
