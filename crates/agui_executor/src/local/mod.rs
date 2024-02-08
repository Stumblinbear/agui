use std::{
    collections::VecDeque,
    future::Future,
    hash::BuildHasherDefault,
    sync::{mpsc, Arc},
    time::{Duration, Instant},
};

use agui_sync::notify;
use futures::{executor::LocalPool, future::FusedFuture, prelude::future::FutureExt};

use agui_core::{
    callback::strategies::CallbackStrategy,
    element::{deferred::resolver::DeferredResolver, ElementCallbackContext, ElementId},
    engine::{elements::ElementTree, rendering::RenderingTree},
    reactivity::{BuildError, SpawnAndInflateError},
    render::RenderObjectId,
    widget::IntoWidget,
};
use rustc_hash::{FxHashSet, FxHasher};
use slotmap::{SecondaryMap, SparseSecondaryMap};

use crate::{
    local::{
        callback::{InvokeCallback, LocalCallbacks},
        create_render_object::ImmediatelyCreateRenderObjects,
        inflate_root::InflateRoot,
        layout::LayoutRenderObjects,
        rebuild::RebuildStrategy,
        rendering_cleanup::RenderingTreeCleanup,
        scheduler::LocalScheduler,
        unmount::ElementTreeUnmount,
        update_render_object::ImmediatelyUpdateRenderObjects,
    },
    EngineExecutor,
};

mod callback;
mod create_render_object;
mod inflate_root;
mod layout;
mod rebuild;
mod rendering_cleanup;
mod scheduler;
mod unmount;
mod update_render_object;

pub struct LocalEngineExecutor {
    scheduler: LocalScheduler,
    callbacks: Arc<dyn CallbackStrategy>,

    element_tree: ElementTree,
    deferred_elements: SecondaryMap<RenderObjectId, (ElementId, Box<dyn DeferredResolver>)>,

    needs_build_rx: mpsc::Receiver<ElementId>,
    rebuild_queue: VecDeque<ElementId>,

    callback_rx: mpsc::Receiver<InvokeCallback>,

    spawned_elements: VecDeque<ElementId>,
    updated_elements: SparseSecondaryMap<ElementId, (), BuildHasherDefault<FxHasher>>,

    element_update_rx: notify::Subscriber,

    rendering_tree: RenderingTree,
    render_update_rx: notify::Subscriber,

    needs_layout_rx: mpsc::Receiver<RenderObjectId>,
    needs_paint_rx: mpsc::Receiver<RenderObjectId>,

    pool: LocalPool,
}

impl Default for LocalEngineExecutor {
    fn default() -> Self {
        let (needs_build_tx, needs_build_rx) = mpsc::channel();
        let (callback_tx, callback_rx) = mpsc::channel();

        let element_update_tx = notify::Flag::new();
        let element_update_rx = element_update_tx.subscribe();

        let render_update_tx = notify::Flag::new();
        let render_update_rx = render_update_tx.subscribe();

        let (needs_layout_tx, needs_layout_rx) = mpsc::channel();
        let (needs_paint_tx, needs_paint_rx) = mpsc::channel();

        let pool = LocalPool::default();

        let scheduler = LocalScheduler {
            needs_build_tx,

            element_update_tx: element_update_tx.clone(),

            needs_layout_tx,
            needs_paint_tx,

            render_update_tx,

            spawner: pool.spawner(),
        };

        Self {
            scheduler,
            #[allow(clippy::arc_with_non_send_sync)]
            callbacks: Arc::new(LocalCallbacks {
                callback_tx,
                element_update_tx,
            }),

            element_tree: ElementTree::default(),
            deferred_elements: SecondaryMap::default(),

            needs_build_rx,

            rebuild_queue: VecDeque::default(),

            callback_rx,

            spawned_elements: VecDeque::default(),
            updated_elements: SparseSecondaryMap::default(),

            element_update_rx,

            rendering_tree: RenderingTree::default(),
            render_update_rx,

            needs_layout_rx,
            needs_paint_rx,

            pool,
        }
    }
}

impl LocalEngineExecutor {
    pub fn with_root(root: impl IntoWidget) -> Result<Self, SpawnAndInflateError<ElementId>> {
        let mut executor = Self::default();

        executor.element_tree.inflate(
            &mut InflateRoot {
                scheduler: &mut executor.scheduler,
                callbacks: &executor.callbacks,

                spawned_elements: &mut executor.spawned_elements,
            },
            root.into_widget(),
        )?;

        Ok(executor)
    }
}

impl LocalEngineExecutor {
    #[tracing::instrument(level = "trace", skip(self))]
    fn flush_callbacks(&mut self) {
        tracing::trace!("flushing callbacks");

        while let Ok(invoke) = self.callback_rx.try_recv() {
            let element_id = invoke.callback_id.element_id();

            let existed = self
                .element_tree
                .with(invoke.callback_id.element_id(), |ctx, element| {
                    let changed = element.call(
                        &mut ElementCallbackContext {
                            scheduler: &mut ctx.scheduler.with_strategy(&mut self.scheduler),

                            element_tree: ctx.tree,
                            inheritance: ctx.inheritance,

                            element_id: &element_id,
                        },
                        invoke.callback_id,
                        invoke.arg,
                    );

                    if changed {
                        tracing::trace!("callback updated element, queueing for rebuild");

                        self.rebuild_queue.push_back(element_id);
                    }
                })
                .is_some();

            if !existed {
                tracing::warn!("callback invoked on an element that does not exist");
            }
        }
    }

    #[tracing::instrument(level = "trace", skip(self))]
    fn flush_needs_build(&mut self) -> bool {
        tracing::trace!("flushing needs build");

        while let Ok(element_id) = self.needs_build_rx.try_recv() {
            if self.element_tree.contains(element_id) {
                tracing::trace!(?element_id, "queueing element for rebuild");

                self.rebuild_queue.push_back(element_id);
            } else {
                tracing::warn!("queued an element for rebuild, but it does not exist in the tree");
            }
        }

        !self.rebuild_queue.is_empty()
    }

    #[tracing::instrument(level = "trace", skip(self))]
    fn flush_rebuilds(&mut self) {
        tracing::trace!("flushing rebuilds");

        // Keep track of which elements ended up being rebuilt, since build_and_realize
        // may end up rebuilding one that's currently in the queue.
        let mut rebuilt_elements = FxHashSet::default();

        rebuilt_elements.reserve(self.rebuild_queue.len().min(8));

        while let Some(element_id) = self.rebuild_queue.pop_front() {
            if rebuilt_elements.contains(&element_id) {
                tracing::trace!(
                    ?element_id,
                    "skipping element that was already rebuilt by another element"
                );

                continue;
            }

            if let Err(err) = self.element_tree.rebuild(
                &mut RebuildStrategy {
                    scheduler: &mut self.scheduler,
                    callbacks: &self.callbacks,

                    spawned_elements: &mut self.spawned_elements,
                    updated_elements: &mut self.updated_elements,

                    rebuilt_elements: &mut rebuilt_elements,
                },
                element_id,
            ) {
                match err {
                    BuildError::Broken => {
                        unreachable!("the tree is in an invalid state, aborting update");
                    }

                    BuildError::NotFound(element_id) => {
                        tracing::warn!(?element_id, "element was missing from the tree");
                    }

                    BuildError::InUse(element_id) => {
                        panic!(
                            "failed to rebuild element as it was in use: {:?}",
                            element_id
                        );
                    }
                }
            }
        }
    }

    #[tracing::instrument(level = "debug", skip(self))]
    fn update_widgets(&mut self) {
        tracing::trace!("widget update started");

        let start = Instant::now();

        self.flush_callbacks();

        let mut num_iterations = 0;

        // Rebuild the tree in a loop until it's fully settled. This is necessary as some
        // widgets being build may cause other widgets to be marked as dirty, which would
        // otherwise be missed in a single pass.
        while !self.rebuild_queue.is_empty() || self.flush_needs_build() {
            num_iterations += 1;

            self.flush_rebuilds();
        }

        let update_widget_tree_end = Instant::now();

        self.element_tree
            .cleanup(&mut ElementTreeUnmount {
                rendering_tree: &mut self.rendering_tree,

                updated_elements: &mut self.updated_elements,
            })
            .expect("failed to cleanup element tree");

        let mut needs_layout = SparseSecondaryMap::default();
        let mut needs_paint = FxHashSet::default();

        for element_id in self.spawned_elements.drain(..) {
            self.rendering_tree.create(
                &mut ImmediatelyCreateRenderObjects {
                    scheduler: &mut self.scheduler,

                    element_tree: &self.element_tree,
                    deferred_elements: &mut self.deferred_elements,

                    needs_layout: &mut needs_layout,
                    needs_paint: &mut needs_paint,
                },
                self.element_tree.as_ref().get_parent(element_id).copied(),
                element_id,
            );

            self.updated_elements.remove(element_id);
        }

        for element_id in self.updated_elements.drain().map(|(id, _)| id) {
            self.rendering_tree.update(
                &mut ImmediatelyUpdateRenderObjects {
                    scheduler: &mut self.scheduler,

                    element_tree: &self.element_tree,

                    needs_layout: &mut needs_layout,
                    needs_paint: &mut needs_paint,
                },
                element_id,
            );
        }

        self.rendering_tree
            .cleanup(&mut RenderingTreeCleanup {
                deferred_elements: &mut self.deferred_elements,
            })
            .expect("failed to cleanup rendering tree");

        let sync_render_tree_end = Instant::now();

        let timings = WidgetUpdateTimings {
            duration: start.elapsed(),

            update_widget_tree: update_widget_tree_end - start,
            sync_render_tree: sync_render_tree_end - update_widget_tree_end,
        };

        tracing::debug!(
            ?timings,
            num_iterations = num_iterations,
            "widget update complete"
        );

        if !needs_layout.is_empty() || !needs_paint.is_empty() {
            self.rendering_tree.layout(
                &mut LayoutRenderObjects {
                    scheduler: &mut self.scheduler,
                    callbacks: &self.callbacks,

                    element_tree: &mut self.element_tree,

                    deferred_elements: &mut self.deferred_elements,

                    needs_paint: &mut needs_paint,
                },
                needs_layout.into_iter().map(|(id, _)| id),
            );

            for render_object_id in needs_paint {
                self.rendering_tree.paint(render_object_id);
            }

            self.rendering_tree.sync_views();
        }
    }

    #[tracing::instrument(level = "debug", skip(self))]
    fn update_renderer(&mut self) {
        tracing::debug!("renderer update started");

        let start = Instant::now();

        let mut needs_paint = FxHashSet::default();

        self.rendering_tree.layout(
            &mut LayoutRenderObjects {
                scheduler: &mut self.scheduler,
                callbacks: &self.callbacks,

                element_tree: &mut self.element_tree,

                deferred_elements: &mut self.deferred_elements,

                needs_paint: &mut needs_paint,
            },
            self.needs_layout_rx.try_iter(),
        );

        let layout_end = Instant::now();

        // TODO: it's entirely possible for paint to be called multiple times on the
        // same render object.
        for render_object_id in needs_paint {
            self.rendering_tree.paint(render_object_id);
        }

        for render_object_id in self.needs_paint_rx.try_iter() {
            self.rendering_tree.paint(render_object_id);
        }

        let paint_end = Instant::now();

        self.rendering_tree.sync_views();

        let sync_views_end = Instant::now();

        let timings = RendererUpdateTimings {
            duration: start.elapsed(),

            layout: layout_end - start,
            paint: paint_end - layout_end,
            sync_views: sync_views_end - paint_end,
        };

        tracing::debug!(?timings, "renderer update complete");
    }
}

impl EngineExecutor for LocalEngineExecutor {
    #[tracing::instrument(level = "debug", skip(self))]
    fn update(&mut self) {
        self.update_widgets();

        self.update_renderer();
    }

    fn run_until_stalled(&mut self) {
        'update_tree: loop {
            let update_future = self.element_update_rx.wait().fuse();
            let render_future = self.render_update_rx.wait().fuse();

            self.update();

            // Run futures until no more progress can be made and no more tree updates are
            // pending.
            loop {
                if !self.pool.try_run_one() {
                    return;
                }

                if update_future.is_terminated() || render_future.is_terminated() {
                    continue 'update_tree;
                }
            }
        }
    }

    fn run_until<Fut, Out>(mut self, fut: Fut) -> Out
    where
        Fut: Future<Output = Out>,
    {
        let fut = fut.fuse();

        futures::pin_mut!(fut);

        let mut update_future = self.element_update_rx.wait().fuse();
        let mut render_future = self.render_update_rx.wait().fuse();

        self.update();

        loop {
            let output = self.pool.run_until(async {
                futures::select! {
                    _ = update_future => {
                        tracing::trace!("update triggered by widget notifier");
                        None
                    }

                    _ = render_future => {
                        tracing::trace!("update triggered by render notifier");
                        None
                    }

                    output = fut => {
                        Some(output)
                    }
                }
            });

            if let Some(output) = output {
                return output;
            }

            if update_future.is_terminated() {
                self.update_widgets();
            }

            if render_future.is_terminated() {
                self.update_renderer();
            }

            update_future = self.element_update_rx.wait().fuse();
            render_future = self.render_update_rx.wait().fuse();
        }
    }

    fn run(self) {
        self.run_until(std::future::pending::<()>())
    }
}

#[derive(Debug)]
#[allow(dead_code)]
struct WidgetUpdateTimings {
    duration: Duration,

    update_widget_tree: Duration,
    sync_render_tree: Duration,
}

#[derive(Debug)]
#[allow(dead_code)]
struct RendererUpdateTimings {
    duration: Duration,

    layout: Duration,
    paint: Duration,
    sync_views: Duration,
}
