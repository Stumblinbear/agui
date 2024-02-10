use std::{future::IntoFuture, sync::mpsc};

use agui_sync::notify;
use futures::{executor::LocalSpawner, task::LocalSpawnExt};

use agui_core::{
    element::{ElementId, ElementTaskNotifyStrategy},
    engine::elements::scheduler::{CreateElementTask, ElementSchedulerStrategy},
    task::{error::TaskError, TaskHandle},
};

#[derive(Clone)]
pub struct ThreadedElementScheduler {
    pub needs_build_tx: mpsc::Sender<ElementId>,

    pub element_update_tx: notify::Flag,

    pub spawner: LocalSpawner,
}

impl ElementSchedulerStrategy for ThreadedElementScheduler {
    #[tracing::instrument(level = "trace", skip(self, task))]
    fn spawn_task(&mut self, task: CreateElementTask) -> Result<TaskHandle<()>, TaskError> {
        struct NotifyStrategy {
            needs_build_tx: mpsc::Sender<ElementId>,
            element_update_tx: notify::Flag,
        }

        impl ElementTaskNotifyStrategy for NotifyStrategy {
            fn mark_needs_build(&mut self, element_id: ElementId) {
                self.needs_build_tx.send(element_id).ok();
                self.element_update_tx.notify();
            }
        }

        tracing::trace!("spawning task for {:?}", task.element_id());

        let fut = task
            .with_notify_strategy(NotifyStrategy {
                needs_build_tx: self.needs_build_tx.clone(),
                element_update_tx: self.element_update_tx.clone(),
            })
            .into_future();

        match self.spawner.spawn_local_with_handle(fut) {
            Ok(handle) => Ok(TaskHandle::from(handle)),
            Err(_) => Err(TaskError::Shutdown),
        }
    }
}
