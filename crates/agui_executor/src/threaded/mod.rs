use core::panic;
use std::{
    collections::VecDeque,
    future::{Future, IntoFuture},
    hash::BuildHasherDefault,
    sync::{mpsc, Arc},
    time::{Duration, Instant},
};

use agui_core::{
    callback::strategies::CallbackStrategy,
    element::{ElementCallbackContext, ElementId},
    engine::{elements::ElementTree, rendering::RenderingTree},
    reactivity::{BuildError, SpawnAndInflateError},
    widget::IntoWidget,
};
use agui_sync::notify;
use futures::{future::FusedFuture, task::LocalSpawnExt, FutureExt, SinkExt, StreamExt};
use futures_executor::LocalPool;
use parking_lot::Mutex;
use rustc_hash::{FxHashSet, FxHasher};
use slotmap::SparseSecondaryMap;

use crate::{
    shared::{
        callbacks::{InvokeCallback, QueueCallbacks},
        cleanup_rendering_tree::CleanupRenderingTree,
        deferred::{
            create_render_object::DeferredCreateRenderObjects,
            update_render_object::DeferredUpdateRenderObjects,
        },
        inflate_root::InflateRoot,
        rebuild::RebuildElements,
        unmount::ElementTreeUnmount,
        update_render_object::ImmediatelyUpdateRenderObjects,
    },
    threaded::{
        elements::{
            create_render_object::ImmediatelyCreateRenderObjects,
            rendering_cleanup::RenderingTreeCleanup, scheduler::ThreadedElementScheduler,
        },
        rendering::{
            scheduler::{SpawnTask, ThreadedRenderingScheduler},
            ThreadedRenderingExecutor,
        },
        resolve_deferred::{ResolveDeferredElement, ResolveDeferredElementReply},
        sync_rendering_tree::SyncRenderingTree,
    },
    EngineExecutor,
};

mod elements;
mod rendering;
mod resolve_deferred;
mod sync_rendering_tree;

pub struct ThreadedEngineExecutor {
    scheduler: ThreadedElementScheduler,

    element_tree: ElementTree,

    callbacks: Arc<dyn CallbackStrategy>,
    callback_rx: mpsc::Receiver<InvokeCallback>,

    needs_build_rx: mpsc::Receiver<ElementId>,

    rendering_tree: Arc<Mutex<RenderingTree>>,
    rendering_scheduler: ThreadedRenderingScheduler,

    sync_tree_tx: futures::channel::mpsc::Sender<SyncRenderingTree>,
    resolve_deferred_rx: futures::channel::mpsc::Receiver<ResolveDeferredElement>,

    pool: LocalPool,
    element_update_rx: notify::Subscriber,
}

impl Default for ThreadedEngineExecutor {
    fn default() -> Self {
        let (sync_tree_tx, sync_tree_rx) = futures::channel::mpsc::channel(0);
        let (resolve_deferred_tx, resolve_deferred_rx) = futures::channel::mpsc::channel(0);

        let (tx, rx) = mpsc::sync_channel(0);

        std::thread::Builder::new()
            .name("agui rendering tree".to_string())
            .spawn({
                move || {
                    let render_update_tx = notify::Flag::new();
                    let render_update_rx = render_update_tx.subscribe();

                    let rendering_tree = Arc::new(Mutex::new(RenderingTree::default()));

                    let pool = LocalPool::new();

                    let (task_tx, mut task_rx) = futures::channel::mpsc::unbounded::<SpawnTask>();

                    let _ = pool.spawner().spawn_local({
                        let spawner = pool.spawner();

                        async move {
                            while let Some(spawn_task) = task_rx.next().await {
                                if let Ok(handle) =
                                    spawner.spawn_local_with_handle(spawn_task.task.into_future())
                                {
                                    let _ = spawn_task.reply_tx.send(handle);
                                }
                            }
                        }
                    });

                    let (needs_layout_tx, needs_layout_rx) = mpsc::channel();
                    let (needs_paint_tx, needs_paint_rx) = mpsc::channel();

                    let scheduler = ThreadedRenderingScheduler {
                        task_tx,

                        needs_layout_tx,
                        needs_paint_tx,

                        render_update_tx,
                    };

                    let _ = tx.send((Arc::clone(&rendering_tree), scheduler.clone()));

                    ThreadedRenderingExecutor {
                        pool,
                        scheduler,

                        sync_tree_rx,
                        resolve_deferred_tx,

                        rendering_tree,

                        deferred_elements: SparseSecondaryMap::default(),

                        needs_layout_rx,
                        needs_paint_rx,

                        render_update_rx,
                    }
                    .run();
                }
            })
            .expect("failed to spawn rendering tree thread");

        let (rendering_tree, rendering_scheduler) =
            rx.recv().expect("failed to receive rendering tree");

        let (callback_tx, callback_rx) = mpsc::channel();

        let (needs_build_tx, needs_build_rx) = mpsc::channel();

        let element_update_tx = notify::Flag::new();
        let element_update_rx = element_update_tx.subscribe();

        let pool = LocalPool::default();
        let spawner = pool.spawner();

        Self {
            scheduler: ThreadedElementScheduler {
                needs_build_tx,

                element_update_tx: element_update_tx.clone(),

                spawner,
            },

            #[allow(clippy::arc_with_non_send_sync)]
            callbacks: Arc::new(QueueCallbacks {
                callback_tx,
                element_update_tx,
            }),

            element_tree: ElementTree::default(),

            needs_build_rx,
            callback_rx,

            rendering_tree,
            rendering_scheduler,

            sync_tree_tx,
            resolve_deferred_rx,

            pool,
            element_update_rx,
        }
    }
}

impl ThreadedEngineExecutor {
    pub fn with_root(root: impl IntoWidget) -> Result<Self, SpawnAndInflateError<ElementId>> {
        let mut executor = Self::default();

        executor.set_root(root)?;

        Ok(executor)
    }

    fn set_root(&mut self, root: impl IntoWidget) -> Result<(), SpawnAndInflateError<ElementId>> {
        let mut spawned_elements = VecDeque::<ElementId>::default();

        self.element_tree.inflate(
            &mut InflateRoot {
                scheduler: &mut self.scheduler,
                callbacks: &self.callbacks,

                spawned_elements: &mut spawned_elements,
            },
            root.into_widget(),
        )?;

        let mut rendering_tree = self.rendering_tree.lock();

        let mut sync_tree = SyncRenderingTree::default();

        for element_id in spawned_elements {
            rendering_tree.create(
                &mut ImmediatelyCreateRenderObjects {
                    scheduler: &mut self.rendering_scheduler,

                    element_tree: &self.element_tree,
                    new_deferred_elements: &mut sync_tree.new_deferred_elements,

                    needs_layout: &mut sync_tree.needs_layout,
                    needs_paint: &mut sync_tree.needs_paint,
                },
                self.element_tree.as_ref().get_parent(element_id).copied(),
                element_id,
            );
        }

        if !sync_tree.is_empty() {
            futures::executor::block_on(self.sync_tree_tx.send(sync_tree))
                .expect("failed to sync tree");
        }

        drop(rendering_tree);

        Ok(())
    }

    fn resolve_deferred(
        &mut self,
        ResolveDeferredElement {
            mut rendering_tree,

            mut deferred_elements,

            render_object_id,

            reply_tx,
        }: ResolveDeferredElement,
    ) {
        let mut spawned_elements = Vec::new();
        let mut updated_elements = SparseSecondaryMap::default();
        let mut rebuilt_elements = FxHashSet::default();

        let mut needs_paint = FxHashSet::default();

        let (element_id, resolver) = deferred_elements
            .get(render_object_id)
            .expect("deferred element not found");

        let result = self.element_tree.resolve_deferred(
            &mut RebuildElements {
                scheduler: &mut self.scheduler,
                callbacks: &self.callbacks,

                spawned_elements: &mut spawned_elements,
                updated_elements: &mut updated_elements,

                rebuilt_elements: &mut rebuilt_elements,
            },
            *element_id,
            resolver.as_ref(),
        );

        if let Err(err) = result {
            match err {
                // This can happen in the theoretically rare chance that the element was removed
                // from the tree before we got around to resolving it (i.e. there's an animation running
                // that changes the element's size which triggers a deferred element resolve, but it
                // was removed while layout was in progress).
                //
                // This case will result in a "stale" layout pass, which means the exact constraints
                // requested by the user won't be visually honored, but it should be a rare enough
                // case (and only visible for a fraction of a millisecond due to the impending tree sync)
                // that it's not worth the complexity to handle it.
                BuildError::NotFound(missing_element_id) if missing_element_id == *element_id => {
                    tracing::warn!(
                        ?element_id,
                        "resolved a deferred element, but it was missing from the tree"
                    );
                }

                err => {
                    panic!("failed to resolve deferred element: {:?}", err);
                }
            }
        }

        self.element_tree
            .cleanup(&mut ElementTreeUnmount {
                rendering_tree: &mut rendering_tree,

                updated_elements: &mut updated_elements,
            })
            .expect("failed to cleanup element tree");

        for element_id in spawned_elements {
            rendering_tree.create(
                &mut DeferredCreateRenderObjects {
                    scheduler: &mut self.rendering_scheduler,

                    element_tree: &self.element_tree,
                    deferred_elements: &mut deferred_elements,

                    needs_paint: &mut needs_paint,
                },
                self.element_tree.as_ref().get_parent(element_id).copied(),
                element_id,
            );

            updated_elements.remove(element_id);
        }

        for element_id in updated_elements.drain().map(|(id, _)| id) {
            rendering_tree.update(
                &mut DeferredUpdateRenderObjects {
                    scheduler: &mut self.rendering_scheduler,

                    element_tree: &self.element_tree,

                    needs_paint: &mut needs_paint,
                },
                element_id,
            );
        }

        rendering_tree
            .cleanup(&mut CleanupRenderingTree {
                deferred_elements: &mut deferred_elements,
            })
            .expect("failed to cleanup rendering tree");

        reply_tx
            .send(ResolveDeferredElementReply {
                rendering_tree,

                deferred_elements,

                needs_paint,
            })
            .expect("failed to send deferred element resolve reply");
    }
}

impl EngineExecutor for ThreadedEngineExecutor {
    #[tracing::instrument(level = "debug", skip(self))]
    fn update(&mut self) {
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

        let renderer_timings = if !spawned_elements.is_empty() || !updated_elements.is_empty() {
            let mut rendering_tree = match self.rendering_tree.try_lock() {
                Some(rendering_tree) => rendering_tree,
                None => {
                    todo!("rendering tree is currently being updated, check if it has requested we run layout");
                }
            };

            let lock_renderer_end = Instant::now();

            self.element_tree
                .cleanup(&mut ElementTreeUnmount {
                    rendering_tree: &mut rendering_tree,

                    updated_elements: &mut updated_elements,
                })
                .expect("failed to cleanup element tree");

            let mut sync_tree = SyncRenderingTree::default();

            for element_id in spawned_elements {
                rendering_tree.create(
                    &mut ImmediatelyCreateRenderObjects {
                        scheduler: &mut self.rendering_scheduler,

                        element_tree: &self.element_tree,
                        new_deferred_elements: &mut sync_tree.new_deferred_elements,

                        needs_layout: &mut sync_tree.needs_layout,
                        needs_paint: &mut sync_tree.needs_paint,
                    },
                    self.element_tree.as_ref().get_parent(element_id).copied(),
                    element_id,
                );

                updated_elements.remove(element_id);
            }

            for element_id in updated_elements.drain().map(|(id, _)| id) {
                rendering_tree.update(
                    &mut ImmediatelyUpdateRenderObjects {
                        scheduler: &mut self.rendering_scheduler,

                        element_tree: &self.element_tree,

                        needs_layout: &mut sync_tree.needs_layout,
                        needs_paint: &mut sync_tree.needs_paint,
                    },
                    element_id,
                );
            }

            rendering_tree
                .cleanup(&mut RenderingTreeCleanup {
                    removed_deferred_elements: &mut sync_tree.removed_deferred_elements,
                })
                .expect("failed to cleanup rendering tree");

            // TODO: maybe check if any deferred element needs updated and do layout here?

            drop(rendering_tree);

            if !sync_tree.is_empty() {
                futures::executor::block_on(self.sync_tree_tx.send(sync_tree))
                    .expect("failed to sync tree");
            }

            let sync_render_tree_end = Instant::now();

            Some(RendererUpdateTimings {
                lock_renderer: lock_renderer_end - update_widget_tree_end,
                sync_render_tree: sync_render_tree_end - lock_renderer_end,
            })
        } else {
            None
        };

        let timings = WidgetUpdateTimings {
            duration: start.elapsed(),

            execute_callbacks: execute_callbacks_end - start,
            flush_scheduler: flush_scheduler_end - execute_callbacks_end,
            update_widget_tree: update_widget_tree_end - flush_scheduler_end,

            renderer: renderer_timings,
        };

        tracing::debug!(?timings, "widget update complete");
    }

    #[tracing::instrument(level = "trace", skip_all)]
    fn run_until_stalled(&mut self) {
        futures::executor::block_on(async {
            'update_tree: loop {
                let update_future = self.element_update_rx.wait().fuse();

                self.update();

                // TODO: This should wait for the render manager to be stalled as well to match the
                // behavior of the single threaded executor.

                // Run futures until no more progress can be made and no more tree updates are
                // pending.
                loop {
                    if !self.pool.try_run_one() {
                        return;
                    }

                    if update_future.is_terminated() {
                        continue 'update_tree;
                    }
                }
            }
        })
    }

    fn run_until<Fut, Out>(mut self, fut: Fut) -> Out
    where
        Fut: Future<Output = Out>,
    {
        let fut = fut.fuse();

        futures::pin_mut!(fut);

        loop {
            let mut update_future = self.element_update_rx.wait().fuse();

            self.update();

            let output = self.pool.run_until(async {
                futures::select! {
                    _ = update_future => {
                        ElementUpdate::Notified
                    }

                    deferred_resolver = self.resolve_deferred_rx.next() => {
                        ElementUpdate::ResolveDeferred(deferred_resolver.expect("resolve deferred element stream ended"))
                    }

                    out = fut => {
                        ElementUpdate::Output(out)
                    }
                }
            });

            match output {
                ElementUpdate::Notified => {
                    tracing::trace!("update triggered by widget notifier");
                    continue;
                }

                ElementUpdate::ResolveDeferred(request) => {
                    tracing::debug!("received request to resolve deferred element");

                    self.resolve_deferred(request);

                    continue;
                }

                ElementUpdate::Output(output) => {
                    return output;
                }
            }
        }
    }

    #[tracing::instrument(level = "trace", skip_all)]
    fn run(self) {
        self.run_until(std::future::pending::<()>())
    }
}

enum ElementUpdate<O> {
    Notified,
    ResolveDeferred(ResolveDeferredElement),
    Output(O),
}

#[derive(Debug)]
#[allow(dead_code)]
struct WidgetUpdateTimings {
    duration: Duration,

    execute_callbacks: Duration,
    flush_scheduler: Duration,
    update_widget_tree: Duration,

    renderer: Option<RendererUpdateTimings>,
}

#[derive(Debug)]
#[allow(dead_code)]
struct RendererUpdateTimings {
    lock_renderer: Duration,
    sync_render_tree: Duration,
}
