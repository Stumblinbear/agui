use std::{
    future::{Future, IntoFuture},
    pin::Pin,
};

use crate::{
    element::{ContextRenderObject, RenderingTaskContext, RenderingTaskNotifyStrategy},
    render::RenderObjectId,
    task::{error::TaskError, TaskHandle},
};

pub struct CreateRenderingTask {
    ctx: RenderingTaskContext,
    #[allow(clippy::type_complexity)]
    func: Box<dyn FnOnce(RenderingTaskContext) -> Pin<Box<dyn Future<Output = ()>>>>,
}

impl CreateRenderingTask {
    pub fn render_object_id(&self) -> RenderObjectId {
        self.ctx.render_object_id()
    }

    pub fn with_notify_strategy<T>(self, strategy: T) -> Self
    where
        T: RenderingTaskNotifyStrategy + 'static,
    {
        Self {
            ctx: self.ctx.with_notify_strategy(strategy),

            func: self.func,
        }
    }
}

impl IntoFuture for CreateRenderingTask {
    type Output = ();

    type IntoFuture = Pin<Box<dyn Future<Output = ()>>>;

    fn into_future(self) -> Self::IntoFuture {
        (self.func)(self.ctx)
    }
}

#[allow(unused_variables)]
pub trait RenderingSchedulerStrategy {
    fn spawn_task(&mut self, task: CreateRenderingTask) -> Result<TaskHandle<()>, TaskError>;
}

impl RenderingSchedulerStrategy for () {
    fn spawn_task(&mut self, _: CreateRenderingTask) -> Result<TaskHandle<()>, TaskError> {
        Err(TaskError::no_scheduler())
    }
}

pub struct RenderingScheduler<'ctx> {
    render_object_id: &'ctx RenderObjectId,
    strategy: Option<&'ctx mut dyn RenderingSchedulerStrategy>,
}

impl<'ctx> RenderingScheduler<'ctx> {
    pub(super) fn new(render_object_id: &'ctx RenderObjectId) -> Self {
        RenderingScheduler {
            render_object_id,
            strategy: None,
        }
    }

    pub fn with_strategy<'a>(
        self,
        strategy: &'a mut dyn RenderingSchedulerStrategy,
    ) -> RenderingScheduler<'a>
    where
        'ctx: 'a,
    {
        RenderingScheduler {
            render_object_id: self.render_object_id,
            strategy: Some(strategy),
        }
    }

    pub fn spawn_task<Fut>(
        &mut self,
        func: impl FnOnce(RenderingTaskContext) -> Fut + 'static,
    ) -> Result<TaskHandle<()>, TaskError>
    where
        Fut: Future<Output = ()> + 'static,
    {
        let Some(strategy) = self.strategy.as_mut() else {
            return Err(TaskError::no_scheduler());
        };

        strategy.spawn_task(CreateRenderingTask {
            ctx: RenderingTaskContext::new(*self.render_object_id),
            func: Box::new(|ctx| Box::pin(func(ctx))),
        })
    }
}
