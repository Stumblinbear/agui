use std::sync::Arc;

use agui_core::{
    render::{canvas::Canvas, view::View, RenderObjectId},
    unit::{Offset, Size},
};
use async_channel::TrySendError;
use parking_lot::RwLock;

use crate::render::VelloScene;

pub struct VelloView {
    tx: async_channel::Sender<()>,
    rx: async_channel::Receiver<()>,

    scene: Arc<RwLock<VelloScene>>,

    changes: Vec<Change>,
}

#[derive(Clone)]
pub struct VelloViewHandle {
    rx: async_channel::Receiver<()>,
    scene: Arc<RwLock<VelloScene>>,
}

impl VelloViewHandle {
    pub fn notifier(&self) -> async_channel::Receiver<()> {
        self.rx.clone()
    }

    pub(crate) fn with_scene<F, Ret>(&self, func: F) -> Ret
    where
        F: FnOnce(&VelloScene) -> Ret,
    {
        let scene = self.scene.read();

        func(&scene)
    }
}

impl Default for VelloView {
    fn default() -> Self {
        let (tx, rx) = async_channel::bounded(1);

        Self {
            tx,
            rx,

            scene: Arc::default(),

            changes: Vec::new(),
        }
    }
}

impl VelloView {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn handle(&self) -> VelloViewHandle {
        VelloViewHandle {
            rx: self.rx.clone(),

            scene: Arc::clone(&self.scene),
        }
    }

    pub(crate) fn clone(&self) -> VelloView {
        VelloView {
            tx: self.tx.clone(),
            rx: self.rx.clone(),

            scene: Arc::clone(&self.scene),

            changes: Vec::new(),
        }
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

        // TODO: if this is locked, we should somehow check if another frame is ready and
        // skip this one
        let mut scene = self.scene.write();

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

        scene.redraw();

        if let Err(TrySendError::Full(_)) = self.tx.try_send(()) {
            tracing::warn!("sync occurred before the previous frame was rendered (frame skipped)");
        }
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
