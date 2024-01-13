use agui_core::{
    render::{canvas::Canvas, view::View, RenderObjectId},
    unit::{Offset, Size},
};

pub struct VelloView {
    tx: async_channel::Sender<ViewEvent>,
    rx: async_channel::Receiver<ViewEvent>,
}

impl VelloView {
    pub fn new() -> Self {
        let (tx, rx) = async_channel::unbounded();

        Self { tx, rx }
    }

    pub fn handle(&self) -> VelloViewHandle {
        VelloViewHandle {
            rx: self.rx.clone(),
        }
    }

    pub(crate) fn clone(&self) -> VelloView {
        VelloView {
            tx: self.tx.clone(),
            rx: self.rx.clone(),
        }
    }
}

impl View for VelloView {
    fn on_attach(
        &mut self,
        parent_render_object_id: Option<RenderObjectId>,
        render_object_id: RenderObjectId,
    ) {
        tracing::debug!(
            "VelloView::on_attach {:?} {:?}",
            parent_render_object_id,
            render_object_id
        );
    }

    fn on_detach(&mut self, render_object_id: RenderObjectId) {
        tracing::debug!("VelloView::on_detach {:?}", render_object_id);
    }

    fn on_size_changed(&mut self, render_object_id: RenderObjectId, size: Size) {
        tracing::debug!(
            "VelloView::on_size_changed {:?} {:?}",
            render_object_id,
            size
        );
    }

    fn on_offset_changed(&mut self, render_object_id: RenderObjectId, offset: Offset) {
        tracing::debug!(
            "VelloView::on_offset_changed {:?} {:?}",
            render_object_id,
            offset
        );
    }

    fn on_paint(&mut self, render_object_id: RenderObjectId, canvas: Canvas) {
        tracing::debug!("VelloView::on_paint {:?} {:?}", render_object_id, canvas);
    }

    fn on_sync(&mut self) {
        tracing::debug!("VelloView::on_sync");
    }
}

#[derive(Clone)]
pub struct VelloViewHandle {
    rx: async_channel::Receiver<ViewEvent>,
}

pub(crate) enum ViewEvent {
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

    Sync,
}
