use std::{
    hash::BuildHasherDefault,
    sync::{mpsc, Arc},
    time::{Duration, Instant},
};

use agui_core::{
    element::{deferred::resolver::DeferredResolver, ElementId},
    engine::rendering::RenderingTree,
    render::RenderObjectId,
};
use agui_sync::notify;
use futures::{prelude::stream::StreamExt, FutureExt};
use futures_executor::LocalPool;
use parking_lot::Mutex;
use rustc_hash::{FxHashSet, FxHasher};
use slotmap::SparseSecondaryMap;

use crate::threaded::{
    rendering::{layout::ThreadedLayoutRenderingTree, scheduler::ThreadedRenderingScheduler},
    resolve_deferred::ResolveDeferredElement,
    sync_rendering_tree::SyncRenderingTree,
};

mod layout;
pub mod scheduler;

pub struct ThreadedRenderingExecutor {
    pub pool: LocalPool,
    pub scheduler: ThreadedRenderingScheduler,

    pub sync_tree_rx: futures::channel::mpsc::Receiver<SyncRenderingTree>,
    pub resolve_deferred_tx: futures::channel::mpsc::Sender<ResolveDeferredElement>,

    pub rendering_tree: Arc<Mutex<RenderingTree>>,

    pub deferred_elements: SparseSecondaryMap<
        RenderObjectId,
        (ElementId, Box<dyn DeferredResolver>),
        BuildHasherDefault<FxHasher>,
    >,

    pub needs_layout_rx: mpsc::Receiver<RenderObjectId>,
    pub needs_paint_rx: mpsc::Receiver<RenderObjectId>,

    pub render_update_rx: notify::Subscriber,
}

impl ThreadedRenderingExecutor {
    #[tracing::instrument(level = "debug", skip_all)]
    fn update(
        &mut self,
        needs_layout: FxHashSet<RenderObjectId>,
        needs_paint: FxHashSet<RenderObjectId>,
    ) {
        tracing::trace!("renderer update started");

        let start = Instant::now();

        let mut rendering_tree = self.rendering_tree.lock();

        let lock_renderer_end = Instant::now();

        let mut needs_paint = needs_paint.into_iter().collect::<FxHashSet<_>>();

        rendering_tree.layout(
            &mut ThreadedLayoutRenderingTree {
                scheduler: &mut self.scheduler,

                deferred_elements: &mut self.deferred_elements,

                needs_paint: &mut needs_paint,

                resolve_deferred_tx: &mut self.resolve_deferred_tx,
            },
            needs_layout,
        );

        let layout_end = Instant::now();

        for render_object_id in needs_paint {
            rendering_tree.paint(render_object_id);
        }

        let paint_end = Instant::now();

        rendering_tree.sync_views();

        let sync_views_end = Instant::now();

        let timings = RendererUpdateTimings {
            duration: start.elapsed(),

            lock_renderer: lock_renderer_end - start,

            layout: layout_end - lock_renderer_end,
            paint: paint_end - layout_end,
            sync_views: sync_views_end - paint_end,
        };

        tracing::debug!(?timings, "renderer update complete");
    }

    #[tracing::instrument(level = "debug", skip(self))]
    pub fn run(mut self) {
        let mut render_future = self.render_update_rx.wait().fuse();

        loop {
            let output = self.pool.run_until(async {
                futures::select! {
                    sync_tree = self.sync_tree_rx.next() => {
                        RenderUpdate::Sync(sync_tree)
                    }

                    _ = render_future => {
                        RenderUpdate::Notified
                    }
                }
            });

            match output {
                RenderUpdate::Notified => {
                    tracing::trace!("rendering triggered by render notifier");

                    render_future = self.render_update_rx.wait().fuse();

                    self.update(
                        self.needs_layout_rx.try_iter().collect(),
                        self.needs_paint_rx.try_iter().collect(),
                    );
                }

                RenderUpdate::Sync(Some(mut sync_tree)) => {
                    tracing::trace!("rendering tree was synced");

                    for (render_object_id, entry) in sync_tree.new_deferred_elements.drain(..) {
                        self.deferred_elements.insert(render_object_id, entry);
                    }

                    for render_object_id in sync_tree.removed_deferred_elements.drain(..) {
                        self.deferred_elements.remove(render_object_id);
                    }

                    self.update(sync_tree.needs_layout, sync_tree.needs_paint);
                }

                RenderUpdate::Sync(None) => {
                    tracing::trace!("sync tree stream ended, shutting down rendering");
                    break;
                }
            }
        }
    }
}

enum RenderUpdate {
    Notified,
    Sync(Option<SyncRenderingTree>),
}

#[derive(Debug)]
#[allow(dead_code)]
struct RendererUpdateTimings {
    duration: Duration,

    lock_renderer: Duration,

    layout: Duration,
    paint: Duration,
    sync_views: Duration,
}
