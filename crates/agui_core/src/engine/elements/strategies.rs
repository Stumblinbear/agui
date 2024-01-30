use crate::{
    element::{Element, ElementId},
    engine::elements::context::ElementTreeContext,
    widget::Widget,
};

pub trait UpdateChildrenStrategy {
    fn on_spawned(&mut self, parent_id: Option<ElementId>, id: ElementId);

    fn on_updated(&mut self, id: ElementId);

    fn on_forgotten(&mut self, id: ElementId);
}

pub trait InflateStrategy {
    fn on_spawned(&mut self, parent_id: Option<ElementId>, id: ElementId);

    fn on_updated(&mut self, id: ElementId);

    fn on_forgotten(&mut self, id: ElementId);

    fn build(&mut self, ctx: ElementTreeContext, element: &mut Element) -> Vec<Widget>;
}

#[cfg(test)]
pub mod tests {
    use std::sync::mpsc;

    use agui_sync::notify;

    use crate::{
        callback::CallbackQueue,
        element::ElementBuildContext,
        engine::elements::scheduler::{CreateElementTask, ElementSchedulerStrategy},
        task::{error::TaskError, TaskHandle},
    };

    use super::*;

    pub struct MockSchedulerStratgy;

    impl ElementSchedulerStrategy for MockSchedulerStratgy {
        fn spawn_task(&mut self, _: CreateElementTask) -> Result<TaskHandle<()>, TaskError> {
            Err(TaskError::no_scheduler())
        }
    }

    pub struct MockInflateStrategy {
        pub spawned: Vec<ElementId>,
        pub updated: Vec<ElementId>,
        pub forgotten: Vec<ElementId>,

        pub callback_queue: CallbackQueue,
    }

    impl Default for MockInflateStrategy {
        fn default() -> Self {
            Self {
                spawned: Vec::new(),
                updated: Vec::new(),
                forgotten: Vec::new(),

                callback_queue: CallbackQueue::new(mpsc::channel().0, notify::Flag::new()),
            }
        }
    }

    impl InflateStrategy for MockInflateStrategy {
        fn on_spawned(&mut self, _: Option<ElementId>, id: ElementId) {
            self.spawned.push(id);
        }

        fn on_updated(&mut self, id: ElementId) {
            self.updated.push(id);
        }

        fn on_forgotten(&mut self, id: ElementId) {
            self.forgotten.push(id);
        }

        fn build(&mut self, ctx: ElementTreeContext, element: &mut Element) -> Vec<Widget> {
            element.build(&mut ElementBuildContext {
                scheduler: &mut ctx.scheduler.with_strategy(&mut MockSchedulerStratgy),

                element_tree: ctx.tree,
                inheritance: ctx.inheritance,
                callback_queue: &self.callback_queue,

                element_id: ctx.element_id,
            })
        }
    }
}
