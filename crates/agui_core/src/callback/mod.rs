use std::{
    any::{Any, TypeId},
    marker::PhantomData,
    sync::Arc,
};

use crate::{element::ElementId, unit::AsAny};

mod context;
mod func;
mod queue;

pub use context::*;
pub(crate) use func::*;
pub use queue::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CallbackId {
    element_id: ElementId,
    type_id: TypeId,
}

impl CallbackId {
    pub fn get_element_id(&self) -> ElementId {
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
    pub fn call_unchecked(&self, arg: Box<dyn Any>) {
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
    pub(crate) fn new<F: 'static>(element_id: ElementId, callback_queue: CallbackQueue) -> Self {
        Self {
            phantom: PhantomData,

            id: CallbackId {
                element_id,
                type_id: TypeId::of::<F>(),
            },

            callback_queue,
        }
    }

    pub fn get_id(&self) -> CallbackId {
        self.id
    }

    pub fn call(&self, arg: A) {
        self.callback_queue.call_unchecked(self.id, Box::new(arg));
    }

    /// # Panics
    ///
    /// You must ensure the callback is expecting the type of the `args` passed in. If the type
    /// is different, it will panic.
    pub fn call_unchecked(&self, arg: Box<dyn Any>) {
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
        // war crimes
        std::ptr::eq(
            Arc::as_ptr(&self.func) as *const _ as *const (),
            Arc::as_ptr(&other.func) as *const _ as *const (),
        )
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
    use std::cell::RefCell;

    use agui_macros::{LayoutWidget, StatelessWidget};

    use crate::{
        callback::Callback,
        engine::Engine,
        unit::{Constraints, Size},
        widget::{BuildContext, LayoutContext, Widget, WidgetBuild, WidgetLayout},
    };

    thread_local! {
        pub static CALLBACK: RefCell<Vec<Callback<u32>>> = RefCell::default();
        pub static RESULT: RefCell<Vec<u32>> = RefCell::default();
    }

    #[derive(LayoutWidget)]
    struct TestDummyWidget;

    impl WidgetLayout for TestDummyWidget {
        fn build(&self, _: &mut BuildContext<Self>) -> Vec<Widget> {
            vec![]
        }

        fn layout(&self, _: &mut LayoutContext, _: Constraints) -> Size {
            Size::ZERO
        }
    }

    #[derive(StatelessWidget)]
    struct TestWidget {
        child: Widget,
    }

    impl WidgetBuild for TestWidget {
        fn build(&self, ctx: &mut BuildContext<Self>) -> Widget {
            let callback = ctx.callback::<u32, _>(|_ctx, val| {
                RESULT.with(|f| {
                    f.borrow_mut().push(val);
                });
            });

            CALLBACK.with(|f| {
                f.borrow_mut().push(callback);
            });

            self.child.clone()
        }
    }

    #[test]
    pub fn should_not_call_immediately() {
        let mut engine = Engine::builder()
            .with_root(TestWidget {
                child: TestDummyWidget.into(),
            })
            .build();

        engine.update();

        let callback = CALLBACK.with(|f| f.borrow()[0].clone());

        callback.call(3);

        RESULT.with(|f| {
            assert_ne!(
                f.borrow().first().copied(),
                Some(3),
                "callback should not have been called immediately"
            );
        });
    }

    #[test]
    pub fn can_fire_callbacks() {
        let mut engine = Engine::builder()
            .with_root(TestWidget {
                child: TestDummyWidget.into(),
            })
            .build();

        engine.update();

        let callback = CALLBACK.with(|f| f.borrow()[0].clone());

        callback.call(7);

        engine.update();

        RESULT.with(|f| {
            assert_eq!(f.borrow()[0], 7, "callback should have been executed");
        });

        callback.call_unchecked(Box::new(10_u32));

        engine.update();

        RESULT.with(|f| {
            assert_eq!(
                f.borrow()[1],
                10,
                "unsafe callback should have been executed"
            );
        });
    }

    #[test]
    pub fn can_fire_many_callbacks() {
        let mut engine = Engine::builder()
            .with_root(TestWidget {
                child: TestWidget {
                    child: TestDummyWidget.into(),
                }
                .into(),
            })
            .build();

        engine.update();

        let callbacks = CALLBACK.with(|f| f.borrow().clone());

        callbacks.iter().for_each(|callback| {
            callback.call(47);
        });

        engine.update();

        RESULT.with(|f| {
            assert_eq!(
                f.borrow()[0],
                47,
                "callback should have been called during update"
            );

            assert_eq!(
                f.borrow()[1],
                47,
                "callback should have been called during update"
            );
        });

        callbacks.iter().for_each(|callback| {
            callback.call(53);
        });

        engine.update();

        RESULT.with(|f| {
            assert_eq!(
                f.borrow()[2],
                53,
                "callback should have been called during update"
            );

            assert_eq!(
                f.borrow()[3],
                53,
                "callback should have been called during update"
            );
        });
    }
}
