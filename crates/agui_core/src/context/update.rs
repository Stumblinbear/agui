use std::{any::Any, future::Future, rc::Rc};

use crate::{
    context::TaskCtx,
    element::{BuildScope, RoutingId, RoutingPath},
    provide::ProvideScope,
    scheduling::{TaskHandle, TaskScheduler},
};

pub struct UpdateCtx<'a> {
    scheduler: &'a mut dyn TaskScheduler,

    routing_path: &'a mut Vec<RoutingId>,

    provide_scope: &'a ProvideScope,

    build_scope: &'a BuildScope,
}

impl<'a> UpdateCtx<'a> {
    pub fn new(
        scheduler: &'a mut dyn TaskScheduler,
        routing_path: &'a mut Vec<RoutingId>,
        provide_scope: &'a ProvideScope,
        build_scope: &'a BuildScope,
    ) -> Self {
        Self {
            scheduler,

            routing_path,

            provide_scope,

            build_scope,
        }
    }

    /// The path that addresses the current point in the build walk: its boundary and the ids within it.
    pub fn routing_path(&self) -> RoutingPath {
        RoutingPath::new(
            self.build_scope.boundary().unwrap_or_default(),
            self.routing_path.clone(),
        )
    }

    pub fn provide_scope(&self) -> &ProvideScope {
        self.provide_scope
    }

    pub fn get_provided<T>(&self) -> Option<Rc<T>>
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

    pub(crate) fn with_provided<V, T>(
        &mut self,
        value: Rc<V>,
        func: impl FnOnce(&mut UpdateCtx) -> T,
    ) -> T
    where
        V: Any,
    {
        let scope = self.provide_scope.provide(value);

        let mut update_ctx = UpdateCtx {
            scheduler: self.scheduler,
            routing_path: self.routing_path,
            provide_scope: &scope,
            build_scope: self.build_scope,
        };

        func(&mut update_ctx)
    }

    /// The build boundary the current subtree is reconciled under.
    pub fn build_scope(&self) -> &BuildScope {
        self.build_scope
    }

    /// Runs `func` under `scope`, with the within-boundary routing path reset, so ids pushed inside address
    /// relative to the entered boundary rather than to the parent.
    pub fn with_build_scope<T>(
        &mut self,
        scope: &BuildScope,
        func: impl FnOnce(&mut UpdateCtx) -> T,
    ) -> T {
        let mut routing_path = Vec::new();

        let mut update_ctx = UpdateCtx {
            scheduler: self.scheduler,
            routing_path: &mut routing_path,
            provide_scope: self.provide_scope,
            build_scope: scope,
        };

        func(&mut update_ctx)
    }
}
