use std::{future::Future, mem::MaybeUninit, pin::Pin};

use crate::{
    element::ElementId,
    task::{error::TaskError, TaskHandle},
};

pub type ElementTask = Pin<Box<dyn Future<Output = ()>>>;

#[allow(unused_variables)]
pub trait ElementSchedulerStrategy {
    fn spawn_task(&self, id: ElementId, task: ElementTask) -> Result<TaskHandle<()>, TaskError>;
}

impl ElementSchedulerStrategy for () {
    fn spawn_task(&self, _: ElementId, _: ElementTask) -> Result<TaskHandle<()>, TaskError> {
        Err(TaskError::no_scheduler())
    }
}

pub struct ElementScheduler<'ctx, const HAS_STRATEGY: bool = true> {
    element_id: &'ctx ElementId,
    strategy: MaybeUninit<&'ctx mut dyn ElementSchedulerStrategy>,
}

impl<'ctx> ElementScheduler<'ctx, false> {
    pub(super) fn new(element_id: &'ctx ElementId) -> Self {
        ElementScheduler {
            element_id,
            strategy: MaybeUninit::uninit(),
        }
    }

    pub fn with_strategy<'a>(
        self,
        strategy: &'a mut dyn ElementSchedulerStrategy,
    ) -> ElementScheduler<'a, true>
    where
        'ctx: 'a,
    {
        ElementScheduler {
            element_id: self.element_id,
            strategy: MaybeUninit::new(strategy),
        }
    }
}

impl<'ctx> ElementScheduler<'ctx, true> {
    pub fn spawn_task(&self, task: ElementTask) -> Result<TaskHandle<()>, TaskError> {
        // SAFETY: `ElementScheduler<true>` is only constructed with a valid strategy
        unsafe { self.strategy.assume_init_read() }.spawn_task(*self.element_id, task)
    }
}
