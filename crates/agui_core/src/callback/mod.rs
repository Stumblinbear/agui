use std::{
    any::{Any, TypeId},
    marker::PhantomData,
    sync::Arc,
};

use crate::{element::ElementId, unit::AsAny, util::ptr_eq::PtrEqual};

mod queue;

pub use queue::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CallbackId {
    element_id: ElementId,
    type_id: TypeId,
}

impl CallbackId {
    pub fn element_id(&self) -> ElementId {
        self.element_id
    }
}

#[derive(Default, Clone)]
pub enum Callback<A> {
    #[default]
    None,
    Widget(WidgetCallback<A>),
    Func(FuncCallback<A>),
}

impl<A> Callback<A>
where
    A: AsAny,
{
    pub fn call(&self, arg: A) {
        match self {
            Self::None => {}
            Self::Widget(cb) => cb.call(arg),
            Self::Func(cb) => cb.call(arg),
        }
    }

    /// # Panics
    ///
    /// You must ensure the callback is expecting the type of the `args` passed in. If the type
    /// is different, it will panic.
    pub fn call_unchecked(&self, arg: Box<dyn Any + Send>) {
        match self {
            Self::None => {}
            Self::Widget(cb) => cb.call_unchecked(arg),
            Self::Func(cb) => cb.call_unchecked(arg),
        }
    }
}

impl<A: 'static> PartialEq for Callback<A> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::None, Self::None) => true,
            (Self::Widget(a), Self::Widget(b)) => a == b,
            (Self::Func(a), Self::Func(b)) => a == b,
            _ => false,
        }
    }
}

impl<A: 'static> std::fmt::Debug for Callback<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Callback::None => f.debug_tuple("None").finish(),
            Callback::Widget(cb) => f.debug_tuple("Widget").field(cb).finish(),
            Callback::Func(cb) => f.debug_tuple("Func").field(cb).finish(),
        }
    }
}

#[derive(Clone)]
pub struct WidgetCallback<A> {
    phantom: PhantomData<A>,

    id: CallbackId,

    callback_queue: CallbackQueue,
}

impl<A> WidgetCallback<A>
where
    A: AsAny,
{
    pub fn new<F: 'static>(element_id: ElementId, callback_queue: CallbackQueue) -> Self {
        Self::new_unchecked(element_id, TypeId::of::<F>(), callback_queue)
    }

    pub fn new_unchecked(
        element_id: ElementId,
        type_id: TypeId,
        callback_queue: CallbackQueue,
    ) -> Self {
        Self {
            phantom: PhantomData,

            id: CallbackId {
                element_id,
                type_id,
            },

            callback_queue,
        }
    }

    pub fn id(&self) -> CallbackId {
        self.id
    }

    pub fn call(&self, arg: A) {
        self.callback_queue.call_unchecked(self.id, Box::new(arg));
    }

    /// # Panics
    ///
    /// You must ensure the callback is expecting the type of the `args` passed in. If the type
    /// is different, it will panic.
    pub fn call_unchecked(&self, arg: Box<dyn Any + Send>) {
        self.callback_queue.call_unchecked(self.id, arg);
    }
}

unsafe impl<A> Send for WidgetCallback<A> {}
unsafe impl<A> Sync for WidgetCallback<A> {}

impl<A> PartialEq for WidgetCallback<A> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<A> std::fmt::Debug for WidgetCallback<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WidgetCallback")
            .field("id", &self.id)
            .finish()
    }
}

#[derive(Clone)]
pub struct FuncCallback<A> {
    func: Arc<dyn Fn(A) + Send + Sync>,
}

impl<A> FuncCallback<A>
where
    A: AsAny,
{
    pub fn call(&self, arg: A) {
        (self.func)(arg);
    }

    /// # Panics
    ///
    /// You must ensure the callback is expecting the type of the `args` passed in. If the type
    /// is different, it will panic.
    pub fn call_unchecked(&self, arg: Box<dyn Any>) {
        let arg = arg
            .downcast::<A>()
            .expect("failed to downcast callback argument");

        (self.func)(*arg)
    }
}

impl<A> PartialEq for FuncCallback<A>
where
    A: AsAny,
{
    fn eq(&self, other: &Self) -> bool {
        self.func.is_exact_ptr(&other.func)
    }
}

impl<A: 'static> std::fmt::Debug for FuncCallback<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FuncCallback")
            .field("func", &TypeId::of::<A>())
            .finish()
    }
}

impl<A, F> From<F> for Callback<A>
where
    A: AsAny,
    F: Fn(A) + Send + Sync + 'static,
{
    fn from(value: F) -> Self {
        Self::Func(FuncCallback {
            func: Arc::new(value),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::{any::TypeId, rc::Rc};

    use crate::{
        callback::WidgetCallback,
        element::mock::{build::MockBuildWidget, DummyWidget},
        engine::widgets::WidgetManager,
        widget::IntoWidget,
    };

    #[test]
    pub fn should_not_call_immediately() {
        let widget = MockBuildWidget::default();
        let widget_mock = Rc::clone(&widget.mock);
        {
            let mut widget_mock = widget_mock.borrow_mut();

            widget_mock
                .expect_build()
                .returning(|_| DummyWidget.into_widget());

            widget_mock.expect_call().never();
        }

        let mut manager = WidgetManager::builder().with_root(widget).build();

        manager.update();

        WidgetCallback::new_unchecked(
            manager.root(),
            TypeId::of::<()>(),
            manager.callback_queue().clone(),
        )
        .call(3);
    }

    #[test]
    pub fn can_fire_callbacks() {
        let widget = MockBuildWidget::default();
        let widget_mock = Rc::clone(&widget.mock);
        {
            let mut widget_mock = widget_mock.borrow_mut();

            widget_mock
                .expect_build()
                .returning(|_| DummyWidget.into_widget());

            widget_mock.expect_call().once().returning(|_, _, _| false);
        }

        let mut manager = WidgetManager::builder().with_root(widget).build();

        manager.update();

        WidgetCallback::new_unchecked(
            manager.root(),
            TypeId::of::<()>(),
            manager.callback_queue().clone(),
        )
        .call(7);

        manager.update();
    }
}
