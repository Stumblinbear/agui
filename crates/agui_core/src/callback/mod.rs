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

#[derive(Default, Clone, PartialEq)]
pub enum Callback<A>
where
    A: 'static,
{
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

#[derive(Clone)]
pub struct WidgetCallback<A> {
    phantom: PhantomData<A>,

    id: CallbackId,

    callback_queue: CallbackQueue,
}

unsafe impl<A> Send for WidgetCallback<A> where A: AsAny {}
unsafe impl<A> Sync for WidgetCallback<A> where A: AsAny {}

impl<A> PartialEq for WidgetCallback<A>
where
    A: AsAny,
{
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
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

    use agui_macros::StatelessWidget;

    use crate::{
        callback::Callback,
        manager::WidgetManager,
        widget::{BuildContext, Widget, WidgetBuild},
    };

    thread_local! {
        pub static CALLBACK: RefCell<Vec<Callback<u32>>> = RefCell::default();
        pub static RESULT: RefCell<Vec<u32>> = RefCell::default();
    }

    #[derive(Default, StatelessWidget)]
    struct TestWidget {
        child: Option<Widget>,
    }

    impl WidgetBuild for TestWidget {
        type Child = Option<Widget>;

        fn build(&self, ctx: &mut BuildContext<Self>) -> Self::Child {
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
        let mut manager = WidgetManager::new();

        manager.set_root(TestWidget::default());

        manager.update();

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
        let mut manager = WidgetManager::new();

        manager.set_root(TestWidget::default());

        manager.update();

        let callback = CALLBACK.with(|f| f.borrow()[0].clone());

        callback.call(7);

        manager.update();

        RESULT.with(|f| {
            assert_eq!(f.borrow()[0], 7, "callback should have been executed");
        });

        callback.call_unchecked(Box::new(10_u32));

        manager.update();

        RESULT.with(|f| {
            assert_eq!(
                f.borrow()[1],
                10,
                "unsafe callback should have been executed"
            );
        });

        manager.get_callback_queue().call(callback.clone(), 18_u32);

        manager.update();

        RESULT.with(|f| {
            assert_eq!(
                f.borrow()[2],
                18,
                "callback through queue should have been called during update"
            );
        });

        manager.get_callback_queue().call(callback, 31_u32);

        manager.update();

        RESULT.with(|f| {
            assert_eq!(
                f.borrow()[3],
                31,
                "unsafe callback through queue should have been called during update"
            );
        });
    }

    #[test]
    pub fn can_fire_many_callbacks() {
        let mut manager = WidgetManager::new();

        manager.set_root(TestWidget {
            child: TestWidget::default().into(),
        });

        manager.update();

        let callbacks = CALLBACK.with(|f| f.borrow().clone());

        let callback_ids = callbacks.iter().collect::<Vec<_>>();

        manager.get_callback_queue().call_many(&callbacks, 47);

        manager.update();

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

        manager.get_callback_queue().call_many(callback_ids, 53_u32);

        manager.update();

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
