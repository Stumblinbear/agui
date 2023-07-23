use std::{any::TypeId, marker::PhantomData};

use crate::{element::ElementId, unit::Data};

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

    pub fn get_type_id(&self) -> TypeId {
        self.type_id
    }
}

#[derive(Default, Clone)]
pub struct Callback<A>
where
    A: Data,
{
    phantom: PhantomData<A>,

    id: Option<CallbackId>,

    callback_queue: Option<CallbackQueue>,
}

impl<A> PartialEq for Callback<A>
where
    A: Data,
{
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

// #[allow(clippy::non_send_fields_in_send_ty)]
unsafe impl<A> Send for Callback<A> where A: Data {}
unsafe impl<A> Sync for Callback<A> where A: Data {}

impl<A> std::fmt::Debug for Callback<A>
where
    A: Data,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Callback").field(&self.id).finish()
    }
}

impl<A> Callback<A>
where
    A: Data,
{
    pub(crate) fn new<F: 'static>(element_id: ElementId, callback_queue: CallbackQueue) -> Self {
        Self {
            phantom: PhantomData,

            id: Some(CallbackId {
                element_id,
                type_id: TypeId::of::<F>(),
            }),

            callback_queue: Some(callback_queue),
        }
    }

    pub fn get_id(&self) -> Option<CallbackId> {
        self.id
    }

    pub fn is_some(&self) -> bool {
        self.id.is_some()
    }

    pub fn is_none(&self) -> bool {
        self.id.is_none()
    }
}

impl<A> Callback<A>
where
    A: Data,
{
    pub fn call(&self, arg: A) {
        if let Some(callback_queue) = &self.callback_queue {
            if let Some(callback_id) = self.id {
                callback_queue.call_unchecked(callback_id, Box::new(arg));
            }
        }
    }

    /// # Panics
    ///
    /// You must ensure the callback is expecting the type of the `args` passed in. If the type
    /// is different, it will panic.
    pub fn call_unchecked(&self, arg: Box<dyn Data>) {
        if let Some(callback_queue) = &self.callback_queue {
            if let Some(callback_id) = self.id {
                callback_queue.call_unchecked(callback_id, arg);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use agui_macros::StatelessWidget;

    use crate::{
        callback::Callback,
        manager::WidgetManager,
        widget::{BuildContext, WidgetBuild, WidgetRef},
    };

    thread_local! {
        pub static CALLBACK: RefCell<Vec<Callback<u32>>> = RefCell::default();
        pub static RESULT: RefCell<Vec<u32>> = RefCell::default();
    }

    #[derive(Default, StatelessWidget)]
    struct TestWidget {
        children: Vec<WidgetRef>,
    }

    impl WidgetBuild for TestWidget {
        type Child = Vec<WidgetRef>;

        fn build(&self, ctx: &mut BuildContext<Self>) -> Self::Child {
            let callback = ctx.callback::<u32, _>(|_ctx, val| {
                RESULT.with(|f| {
                    f.borrow_mut().push(*val);
                });
            });

            CALLBACK.with(|f| {
                f.borrow_mut().push(callback);
            });

            self.children.clone()
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

        manager.get_callback_queue().call(&callback, 18_u32);

        manager.update();

        RESULT.with(|f| {
            assert_eq!(
                f.borrow()[2],
                18,
                "callback through queue should have been called during update"
            );
        });

        manager
            .get_callback_queue()
            .call_unchecked(callback.get_id().unwrap(), Box::new(31_u32));

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
            children: vec![TestWidget::default().into()],
        });

        manager.update();

        let callbacks = CALLBACK.with(|f| f.borrow().clone());

        let callback_ids = callbacks
            .iter()
            .map(|cb| cb.get_id().unwrap())
            .collect::<Vec<_>>();

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

        manager
            .get_callback_queue()
            .call_many_unchecked(&callback_ids, Box::new(53_u32));

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
