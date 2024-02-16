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
use slotmap::SparseSecondaryMap;

use crate::{
    local::{create_render_object::ImmediatelyCreateRenderObjects, scheduler::LocalScheduler},
    shared::{
        callbacks::{InvokeCallback, QueueCallbacks},
        cleanup_rendering_tree::CleanupRenderingTree,
        inflate_root::InflateRoot,
        layout_render_objects::LayoutRenderingTree,
        rebuild::RebuildElements,
        unmount::ElementTreeUnmount,
        update_render_object::ImmediatelyUpdateRenderObjects,
    },
    EngineExecutor,
};

mod create_render_object;
mod scheduler;

pub struct LocalEngineExecutor {
    pool: LocalPool,
    scheduler: LocalScheduler,

    element_tree: ElementTree,

    callbacks: Arc<dyn CallbackStrategy>,
    callback_rx: mpsc::Receiver<InvokeCallback>,

    needs_build_rx: mpsc::Receiver<ElementId>,

    element_update_rx: notify::Subscriber,

    rendering_tree: RenderingTree,

    deferred_elements: SparseSecondaryMap<
        RenderObjectId,
        (ElementId, Box<dyn DeferredResolver>),
        BuildHasherDefault<FxHasher>,
    >,

    needs_layout_rx: mpsc::Receiver<RenderObjectId>,
    needs_paint_rx: mpsc::Receiver<RenderObjectId>,

    render_update_rx: notify::Subscriber,
}

impl Default for LocalEngineExecutor {
    fn default() -> Self {
        let (callback_tx, callback_rx) = mpsc::channel();

        let (needs_build_tx, needs_build_rx) = mpsc::channel();

        let element_update_tx = notify::Flag::new();
        let element_update_rx = element_update_tx.subscribe();

        let (needs_layout_tx, needs_layout_rx) = mpsc::channel();
        let (needs_paint_tx, needs_paint_rx) = mpsc::channel();

        let render_update_tx = notify::Flag::new();
        let render_update_rx = render_update_tx.subscribe();

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
            pool,
            scheduler,

            element_tree: ElementTree::default(),

            #[allow(clippy::arc_with_non_send_sync)]
            callbacks: Arc::new(QueueCallbacks {
                callback_tx,
                element_update_tx,
            }),
            callback_rx,

            needs_build_rx,

            element_update_rx,

            rendering_tree: RenderingTree::default(),

            deferred_elements: SparseSecondaryMap::default(),

            needs_layout_rx,
            needs_paint_rx,

            render_update_rx,
        }
    }
}

impl LocalEngineExecutor {
    pub fn with_root(root: impl IntoWidget) -> Result<Self, SpawnAndInflateError<ElementId>> {
        let mut executor = Self::default();

        let mut spawned_elements = VecDeque::<ElementId>::default();

        executor.element_tree.inflate(
            &mut InflateRoot {
                scheduler: &mut executor.scheduler,
                callbacks: &executor.callbacks,

                spawned_elements: &mut spawned_elements,
            },
            root.into_widget(),
        )?;

        let mut needs_layout = FxHashSet::default();
        let mut needs_paint = FxHashSet::default();

        for element_id in spawned_elements {
            let parent_element_id = executor
                .element_tree
                .as_ref()
                .get_parent(element_id)
                .copied();

            executor.rendering_tree.create(
                &mut ImmediatelyCreateRenderObjects {
                    scheduler: &mut executor.scheduler,

                    element_tree: &mut executor.element_tree,
                    deferred_elements: &mut executor.deferred_elements,

                    needs_layout: &mut needs_layout,
                    needs_paint: &mut needs_paint,
                },
                parent_element_id,
                element_id,
            );
        }

        executor.rendering_tree.layout(
            &mut LayoutRenderingTree {
                scheduler: &mut executor.scheduler,
                callbacks: &executor.callbacks,

                element_tree: &mut executor.element_tree,

                deferred_elements: &mut executor.deferred_elements,

                needs_paint: &mut needs_paint,
            },
            needs_layout,
        );

        for render_object_id in needs_paint {
            executor.rendering_tree.paint(render_object_id);
        }

        executor.rendering_tree.sync_views();

        Ok(executor)
    }
}

impl LocalEngineExecutor {
    #[tracing::instrument(level = "debug", skip(self))]
    fn update_widgets(&mut self) {
        tracing::trace!("widget update started");

        let start = Instant::now();

        let mut needs_build = VecDeque::<ElementId>::default();

        tracing::trace!("executing pending callbacks");

        // There's no particular reason to prefer callback rebuilds over scheduler rebuilds,
        // I just had to pick one.
        //
        // We collect this so that callbacks that execute other callbacks don't cause the
        // executor to hang.
        for invoke in self.callback_rx.try_iter().collect::<Vec<_>>() {
            let element_id = invoke.callback_id.element_id();

            let existed = self
                .element_tree
                .with(invoke.callback_id.element_id(), |ctx, element| {
                    // let exec_start = if tracing::span_enabled!(tracing::Level::DEBUG) {
                    //     Some(Instant::now())
                    // } else {
                    //     None
                    // };

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

                    // if let Some(exec_start) = exec_start {
                    //     let duration = exec_start.elapsed();

                    //     tracing::debug!(?element_id, ?duration, ?changed, "executed callback");
                    // }

                    if changed {
                        tracing::trace!("callback updated element, queueing for rebuild");

                        needs_build.push_back(element_id);
                    }
                })
                .is_some();

            if !existed {
                tracing::warn!("callback invoked on an element that does not exist");
            }
        }

        let execute_callbacks_end = Instant::now();

        tracing::trace!("flushing scheduler rebuilds");

        for element_id in self.needs_build_rx.try_iter() {
            if self.element_tree.contains(element_id) {
                tracing::trace!(?element_id, "queueing element for rebuild");

                needs_build.push_back(element_id);
            } else {
                tracing::warn!("queued an element for rebuild, but it does not exist in the tree");
            }
        }

        let flush_scheduler_end = Instant::now();

        let mut spawned_elements = Vec::<ElementId>::default();
        let mut updated_elements =
            SparseSecondaryMap::<ElementId, (), BuildHasherDefault<FxHasher>>::default();

        // Keep track of which elements ended up being rebuilt, since build_and_realize
        // may end up rebuilding one that's currently in the queue.
        let mut rebuilt_elements = FxHashSet::default();

        rebuilt_elements.reserve(needs_build.len().min(8));

        for element_id in needs_build {
            if rebuilt_elements.contains(&element_id) {
                tracing::trace!(
                    ?element_id,
                    "skipping element that was already rebuilt by another element"
                );

                continue;
            }

            if let Err(err) = self.element_tree.rebuild(
                &mut RebuildElements {
                    scheduler: &mut self.scheduler,
                    callbacks: &self.callbacks,

                    spawned_elements: &mut spawned_elements,
                    updated_elements: &mut updated_elements,

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

        let update_widget_tree_end = Instant::now();

        self.element_tree
            .cleanup(&mut ElementTreeUnmount {
                rendering_tree: &mut self.rendering_tree,

                updated_elements: &mut updated_elements,
            })
            .expect("failed to cleanup element tree");

        let mut needs_layout = FxHashSet::default();
        let mut needs_paint = FxHashSet::default();

        for element_id in spawned_elements {
            let parent_element_id = self.element_tree.as_ref().get_parent(element_id).copied();

            self.rendering_tree.create(
                &mut ImmediatelyCreateRenderObjects {
                    scheduler: &mut self.scheduler,

                    element_tree: &mut self.element_tree,
                    deferred_elements: &mut self.deferred_elements,

                    needs_layout: &mut needs_layout,
                    needs_paint: &mut needs_paint,
                },
                parent_element_id,
                element_id,
            );

            updated_elements.remove(element_id);
        }

        for element_id in updated_elements.drain().map(|(id, _)| id) {
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
            .cleanup(&mut CleanupRenderingTree {
                deferred_elements: &mut self.deferred_elements,
            })
            .expect("failed to cleanup rendering tree");

        let sync_render_tree_end = Instant::now();

        let timings = WidgetUpdateTimings {
            duration: start.elapsed(),

            execute_callbacks: execute_callbacks_end - start,
            flush_scheduler: flush_scheduler_end - execute_callbacks_end,
            update_widget_tree: update_widget_tree_end - flush_scheduler_end,
            sync_render_tree: sync_render_tree_end - update_widget_tree_end,
        };

        tracing::debug!(?timings, "widget update complete");

        if !needs_layout.is_empty() || !needs_paint.is_empty() {
            self.rendering_tree.layout(
                &mut LayoutRenderingTree {
                    scheduler: &mut self.scheduler,
                    callbacks: &self.callbacks,

                    element_tree: &mut self.element_tree,

                    deferred_elements: &mut self.deferred_elements,

                    needs_paint: &mut needs_paint,
                },
                needs_layout,
            );

            for render_object_id in needs_paint {
                self.rendering_tree.paint(render_object_id);
            }

            self.rendering_tree.sync_views();
        }
    }

    #[tracing::instrument(level = "debug", skip(self))]
    fn update_renderer(&mut self) {
        tracing::trace!("renderer update started");

        let start = Instant::now();

        let mut needs_paint = FxHashSet::default();

        self.rendering_tree.layout(
            &mut LayoutRenderingTree {
                scheduler: &mut self.scheduler,
                callbacks: &self.callbacks,

                element_tree: &mut self.element_tree,

                deferred_elements: &mut self.deferred_elements,

                needs_paint: &mut needs_paint,
            },
            self.needs_layout_rx.try_iter(),
        );

        let layout_end = Instant::now();

        let needs_paint = needs_paint
            .into_iter()
            .chain(self.needs_paint_rx.try_iter())
            .collect::<FxHashSet<_>>();

        for render_object_id in needs_paint {
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

    execute_callbacks: Duration,
    flush_scheduler: Duration,
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
