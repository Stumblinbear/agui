use std::{sync::mpsc, time::Instant};

use crate::{
    element::ElementId,
    engine::{
        render_manager::RenderManager,
        widgets::{WidgetManager, WidgetManagerHooks},
    },
    widget::Widget,
};

mod dirty;
pub mod render_manager;
pub mod update_notifier;
pub mod widgets;

pub use dirty::Dirty;

pub struct Engine {
    widget_manager: WidgetManager<EngineHooks>,
    render_manager: RenderManager,

    spawned_rx: mpsc::Receiver<ElementId>,
    rebuilt_rx: mpsc::Receiver<ElementId>,
    forget_rx: mpsc::Receiver<ElementId>,
}

impl Engine {
    pub fn with_root(root: Widget) -> Engine {
        let (spawned_tx, spawned_rx) = mpsc::channel();
        let (rebuilt_tx, rebuilt_rx) = mpsc::channel();
        let (forget_tx, forget_rx) = mpsc::channel();

        Engine {
            widget_manager: WidgetManager::builder()
                .with_root(root)
                .with_hooks(EngineHooks {
                    spawned_tx,
                    rebuilt_tx,
                    forget_tx,
                })
                .build(),
            render_manager: RenderManager::default(),

            spawned_rx,
            rebuilt_rx,
            forget_rx,
        }
    }

    pub fn update(&mut self) {
        let now = Instant::now();

        self.widget_manager.update();

        tracing::debug!(elapsed = ?now.elapsed(), "widget tree updated");

        let now = Instant::now();

        for element_id in self.forget_rx.try_iter() {
            self.render_manager.forget_element(element_id)
        }

        for element_id in self.spawned_rx.try_iter() {
            self.render_manager.on_create_element(element_id)
        }

        for element_id in self.rebuilt_rx.try_iter() {
            self.render_manager.on_needs_update(element_id)
        }

        self.render_manager
            .sync_render_objects(self.widget_manager.tree());

        // TODO: maybe do this async in case an update triggers while we're
        // performaing layout and painting?
        self.render_manager.flush_layout();

        self.render_manager.flush_needs_paint();

        tracing::trace!(elapsed = ?now.elapsed(), "render tree updated");

        self.render_manager.flush_view_sync();
    }

    pub fn wait_for_update(&self) {
        self.widget_manager.wait_for_update();
    }
}

struct EngineHooks {
    spawned_tx: mpsc::Sender<ElementId>,
    rebuilt_tx: mpsc::Sender<ElementId>,
    forget_tx: mpsc::Sender<ElementId>,
}

impl WidgetManagerHooks for EngineHooks {
    fn on_element_spawned(&mut self, _: Option<ElementId>, id: ElementId) {
        self.spawned_tx.send(id).ok();
    }

    fn on_element_needs_rebuild(&mut self, id: ElementId) {
        self.rebuilt_tx.send(id).ok();
    }

    fn on_element_destroyed(&mut self, id: ElementId) {
        self.forget_tx.send(id).ok();
    }
}
