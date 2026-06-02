use std::{any::Any, future::Future, rc::Rc};

use crate::{
    context::TaskCtx,
    provide::ProvideScope,
    routing_id::{RoutingId, RoutingPath},
    task::{TaskHandle, scheduler::TaskScheduler},
};

pub struct UpdateCtx<'a> {
    scheduler: &'a mut dyn TaskScheduler,

    routing_path: &'a mut Vec<RoutingId>,

    provide_scope: ProvideScope,
}

impl<'a> UpdateCtx<'a> {
    pub fn new(
        scheduler: &'a mut dyn TaskScheduler,
        routing_path: &'a mut Vec<RoutingId>,
        provide_scope: ProvideScope,
    ) -> Self {
        Self {
            scheduler,

            routing_path,

            provide_scope,
        }
    }

    pub fn routing_path(&self) -> RoutingPath {
        RoutingPath::from(self.routing_path.clone())
    }

    pub fn provide_scope(&self) -> &ProvideScope {
        &self.provide_scope
    }

    pub fn get_provided<T>(&self) -> Option<&T>
    where
        T: Any,
    {
        self.provide_scope.get()
    }

    /// Spawn a task tied to this element. `func` receives a [`TaskCtx`] it can use to post messages
    /// back to this element.
    pub fn spawn<F, Fut>(&mut self, func: F) -> Result<TaskHandle, Box<dyn std::error::Error>>
    where
        F: FnOnce(TaskCtx) -> Fut + 'static,
        Fut: Future<Output = ()> + 'static,
    {
        let task_ctx = TaskCtx::new(self.scheduler.event_tx(), self.routing_path());

        self.scheduler.spawn(Box::pin(func(task_ctx)))
    }

    /// An owned scheduler handle that outlives this build.
    pub fn deferred_scheduler(&self) -> Box<dyn TaskScheduler> {
        self.scheduler.deferred()
    }

    pub fn with_routing_id<T>(
        &mut self,
        id: RoutingId,
        func: impl FnOnce(&mut UpdateCtx) -> T,
    ) -> T {
        self.routing_path.push(id);

        let ret = func(self);

        self.routing_path.pop();

        ret
    }

    pub fn with_provided<V, T>(&mut self, value: Rc<V>, func: impl FnOnce(&mut UpdateCtx) -> T) -> T
    where
        V: Any,
    {
        let new_scope = self.provide_scope.provide(value);
        let old_scope = std::mem::replace(&mut self.provide_scope, new_scope);

        let ret = func(self);

        self.provide_scope = old_scope;

        ret
    }
}
