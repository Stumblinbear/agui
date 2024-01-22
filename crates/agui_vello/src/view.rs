use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use agui_core::{
    render::{canvas::Canvas, view::View, RenderObjectId},
    unit::{Offset, Size},
    util::ptr_eq::PtrEqual,
};
use agui_renderer::FrameNotifier;
use parking_lot::{Mutex, RwLock};

use crate::render::VelloScene;

#[derive(Default)]
pub struct VelloView {
    scene: Arc<RwLock<VelloScene>>,

    changes: Vec<Change>,

    frame_notifier: Arc<Mutex<Option<FrameNotifier>>>,
}

#[derive(Clone)]
pub struct VelloViewHandle {
    scene: Arc<RwLock<VelloScene>>,

    frame_notifier: Arc<Mutex<Option<FrameNotifier>>>,
}

impl VelloViewHandle {
    pub(crate) fn set_frame_notifier(&self, frame_notifier: FrameNotifier) {
        self.frame_notifier.lock().replace(frame_notifier);
    }
}

impl VelloView {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_same_view(&self, other: &Self) -> bool {
        self.scene.is_exact_ptr(&other.scene)
    }

    pub(crate) fn handle(&self) -> VelloViewHandle {
        VelloViewHandle {
            scene: Arc::clone(&self.scene),

            frame_notifier: Arc::clone(&self.frame_notifier),
        }
    }

    pub(crate) fn clone(&self) -> VelloView {
        VelloView {
            scene: Arc::clone(&self.scene),

            changes: Vec::new(),

            frame_notifier: Arc::clone(&self.frame_notifier),
        }
    }
}

impl VelloViewHandle {
    pub(crate) fn with_scene<F, Ret>(&self, func: F) -> Ret
    where
        F: FnOnce(&VelloScene) -> Ret,
    {
        let scene = self.scene.read();

        func(&scene)
    }
}

impl View for VelloView {
    fn on_attach(
        &mut self,
        parent_render_object_id: Option<RenderObjectId>,
        render_object_id: RenderObjectId,
    ) {
        tracing::trace!(
            "VelloView::on_attach {:?} {:?}",
            parent_render_object_id,
            render_object_id
        );

        self.changes.push(Change::Attach {
            parent_render_object_id,
            render_object_id,
        });
    }

    fn on_detach(&mut self, render_object_id: RenderObjectId) {
        tracing::trace!("VelloView::on_detach {:?}", render_object_id);

        self.changes.push(Change::Detach { render_object_id });
    }

    fn on_size_changed(&mut self, render_object_id: RenderObjectId, size: Size) {
        tracing::trace!(
            "VelloView::on_size_changed {:?} {:?}",
            render_object_id,
            size
        );

        self.changes.push(Change::SizeChanged {
            render_object_id,
            size,
        });
    }

    fn on_offset_changed(&mut self, render_object_id: RenderObjectId, offset: Offset) {
        tracing::trace!(
            "VelloView::on_offset_changed {:?} {:?}",
            render_object_id,
            offset
        );

        self.changes.push(Change::OffsetChanged {
            render_object_id,
            offset,
        });
    }

    fn on_paint(&mut self, render_object_id: RenderObjectId, canvas: Canvas) {
        tracing::trace!("VelloView::on_paint {:?} {:?}", render_object_id, canvas);

        self.changes.push(Change::Paint {
            render_object_id,
            canvas,
        });
    }

    fn on_sync(&mut self) {
        tracing::trace!("VelloView::on_sync");

        let start = Instant::now();

        // TODO: if this is locked, we should somehow check if another frame is ready and
        // skip this one
        let mut scene = self.scene.write();

        let lock_scene_end = Instant::now();

        for change in self.changes.drain(..) {
            match change {
                Change::Attach {
                    parent_render_object_id,
                    render_object_id,
                } => scene.attach(parent_render_object_id, render_object_id),

                Change::Detach { render_object_id } => scene.detatch(render_object_id),

                Change::SizeChanged {
                    render_object_id,
                    size,
                } => scene.set_size(render_object_id, size),

                Change::OffsetChanged {
                    render_object_id,
                    offset,
                } => scene.set_offset(render_object_id, offset),

                Change::Paint {
                    render_object_id,
                    canvas,
                } => scene.paint(render_object_id, canvas),
            }
        }

        let apply_changes_end = Instant::now();

        scene.redraw();

        let redraw_end = Instant::now();

        let frame_notifier = self.frame_notifier.lock();

        if let Some(frame_notifier) = frame_notifier.as_ref() {
            frame_notifier.notify();
        } else {
            tracing::warn!("a frame was rendered, but no frame notifier was set");
        }

        let frame_notify_end = Instant::now();

        let timings = SyncTimings {
            duration: start.elapsed(),

            lock_scene: lock_scene_end - start,
            apply_changes: apply_changes_end - lock_scene_end,
            redraw: redraw_end - apply_changes_end,
            frame_notify: frame_notify_end - redraw_end,
        };

        tracing::debug!(?timings, "sync complete");
    }
}

enum Change {
    Attach {
        parent_render_object_id: Option<RenderObjectId>,
        render_object_id: RenderObjectId,
    },

    Detach {
        render_object_id: RenderObjectId,
    },

    SizeChanged {
        render_object_id: RenderObjectId,
        size: Size,
    },

    OffsetChanged {
        render_object_id: RenderObjectId,
        offset: Offset,
    },

    Paint {
        render_object_id: RenderObjectId,
        canvas: Canvas,
    },
}

#[derive(Debug)]
#[allow(dead_code)]
struct SyncTimings {
    duration: Duration,

    lock_scene: Duration,
    apply_changes: Duration,
    redraw: Duration,
    frame_notify: Duration,
}
