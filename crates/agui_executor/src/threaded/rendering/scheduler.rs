use std::sync::mpsc;

use agui_core::{
    element::RenderingTaskNotifyStrategy,
    engine::rendering::scheduler::{CreateRenderingTask, RenderingSchedulerStrategy},
    render::RenderObjectId,
    task::{error::TaskError, TaskHandle},
};
use agui_sync::notify;
use futures::future::RemoteHandle;

#[derive(Clone)]
pub struct ThreadedRenderingScheduler {
    pub task_tx: futures::channel::mpsc::UnboundedSender<SpawnTask>,

    pub needs_layout_tx: mpsc::Sender<RenderObjectId>,
    pub needs_paint_tx: mpsc::Sender<RenderObjectId>,

    pub render_update_tx: notify::Flag,
}

impl RenderingSchedulerStrategy for ThreadedRenderingScheduler {
    fn spawn_task(&mut self, task: CreateRenderingTask) -> Result<TaskHandle<()>, TaskError> {
        struct NotifyStrategy {
            needs_layout_tx: mpsc::Sender<RenderObjectId>,
            needs_paint_tx: mpsc::Sender<RenderObjectId>,

            render_update_tx: notify::Flag,
        }

        impl RenderingTaskNotifyStrategy for NotifyStrategy {
            fn mark_needs_layout(&mut self, id: RenderObjectId) {
                self.needs_layout_tx.send(id).ok();
                self.render_update_tx.notify();
            }

            fn mark_needs_paint(&mut self, id: RenderObjectId) {
                self.needs_paint_tx.send(id).ok();
                self.render_update_tx.notify();
            }
        }

        tracing::trace!("spawning task for {:?}", task.render_object_id());

        let (reply_tx, reply_rx) = mpsc::sync_channel(0);

        self.task_tx
            .unbounded_send(SpawnTask {
                task: task.with_notify_strategy(NotifyStrategy {
                    needs_layout_tx: self.needs_layout_tx.clone(),
                    needs_paint_tx: self.needs_paint_tx.clone(),

                    render_update_tx: self.render_update_tx.clone(),
                }),
                reply_tx,
            })
            .map_err(|_| TaskError::Shutdown)?;

        match reply_rx.recv() {
            Ok(handle) => Ok(TaskHandle::from(handle)),
            Err(_) => Err(TaskError::Shutdown),
        }
    }
}

pub struct SpawnTask {
    pub task: CreateRenderingTask,
    pub reply_tx: mpsc::SyncSender<RemoteHandle<()>>,
}
