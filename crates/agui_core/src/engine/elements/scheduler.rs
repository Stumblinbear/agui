use std::{
    future::{Future, IntoFuture},
    pin::Pin,
};

use crate::{
    element::{ContextElement, ElementId, ElementTaskContext, ElementTaskNotifyStrategy},
    task::{error::TaskError, TaskHandle},
};

pub struct CreateElementTask {
    ctx: ElementTaskContext,
    #[allow(clippy::type_complexity)]
    func: Box<dyn FnOnce(ElementTaskContext) -> Pin<Box<dyn Future<Output = ()>>>>,
}

impl CreateElementTask {
    pub fn element_id(&self) -> ElementId {
        self.ctx.element_id()
    }

    pub fn with_notify_strategy<T>(self, strategy: T) -> Self
    where
        T: ElementTaskNotifyStrategy + 'static,
    {
        Self {
            ctx: self.ctx.with_notify_strategy(strategy),

            func: self.func,
        }
    }
}

impl IntoFuture for CreateElementTask {
    type Output = ();

    type IntoFuture = Pin<Box<dyn Future<Output = ()>>>;

    fn into_future(self) -> Self::IntoFuture {
        (self.func)(self.ctx)
    }
}

#[allow(unused_variables)]
pub trait ElementSchedulerStrategy {
    fn spawn_task(&mut self, task: CreateElementTask) -> Result<TaskHandle<()>, TaskError>;
}

impl ElementSchedulerStrategy for () {
    fn spawn_task(&mut self, _: CreateElementTask) -> Result<TaskHandle<()>, TaskError> {
        Err(TaskError::no_scheduler())
    }
}

pub struct ElementScheduler<'ctx> {
    strategy: Option<&'ctx mut dyn ElementSchedulerStrategy>,

    element_id: &'ctx ElementId,
}

impl<'ctx> ElementScheduler<'ctx> {
    pub(super) fn new(element_id: &'ctx ElementId) -> Self {
        ElementScheduler {
            element_id,
            strategy: None,
        }
    }

    pub fn with_strategy<'a>(
        self,
        strategy: &'a mut dyn ElementSchedulerStrategy,
    ) -> ElementScheduler<'a>
    where
        'ctx: 'a,
    {
        ElementScheduler {
            element_id: self.element_id,
            strategy: Some(strategy),
        }
    }

    pub fn spawn_task<Fut>(
        &mut self,
        func: impl FnOnce(ElementTaskContext) -> Fut + 'static,
    ) -> Result<TaskHandle<()>, TaskError>
    where
        Fut: Future<Output = ()> + 'static,
    {
        let Some(strategy) = self.strategy.as_mut() else {
            return Err(TaskError::no_scheduler());
        };

        strategy.spawn_task(CreateElementTask {
            ctx: ElementTaskContext::new(*self.element_id),
            func: Box::new(|ctx| Box::pin(func(ctx))),
        })
    }
}
