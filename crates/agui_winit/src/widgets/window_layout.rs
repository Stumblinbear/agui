use std::sync::Arc;

use agui_core::{
    element::{ContextDirtyRenderObject, RenderObjectCreateContext, RenderObjectUpdateContext},
    render::object::{
        RenderObjectImpl, RenderObjectIntrinsicSizeContext, RenderObjectLayoutContext,
    },
    task::{context::ContextSpawnRenderingTask, TaskHandle},
    unit::{Constraints, IntrinsicDimension, Size},
    widget::Widget,
};
use agui_elements::render::RenderObjectWidget;
use agui_macros::RenderObjectWidget;
use parking_lot::Mutex;

#[derive(RenderObjectWidget)]
pub struct WinitWindowLayout {
    size_rx: async_channel::Receiver<Size>,

    child: Widget,
}

impl RenderObjectWidget for WinitWindowLayout {
    type RenderObject = RenderWinitWindowLayout;

    fn children(&self) -> Vec<Widget> {
        vec![self.child.clone()]
    }

    fn create_render_object(&self, ctx: &mut RenderObjectCreateContext) -> Self::RenderObject {
        RenderWinitWindowLayout::new(ctx, self.size_rx.clone())
    }

    fn update_render_object(
        &self,
        ctx: &mut RenderObjectUpdateContext,
        render_object: &mut Self::RenderObject,
    ) {
        render_object.update_size_rx(ctx, self.size_rx.clone());
    }
}

pub struct RenderWinitWindowLayout {
    size: Arc<Mutex<Size>>,

    size_task: Option<TaskHandle<()>>,
}

impl RenderWinitWindowLayout {
    fn new(ctx: &mut RenderObjectCreateContext, size_rx: async_channel::Receiver<Size>) -> Self {
        let mut ro = Self {
            size: Arc::new(Mutex::new(Size::default())),

            size_task: None,
        };

        ro.update_size_rx(ctx, size_rx);

        ro
    }

    fn update_size_rx<C>(&mut self, ctx: &mut C, size_rx: async_channel::Receiver<Size>)
    where
        C: ContextSpawnRenderingTask,
    {
        let size = Arc::clone(&self.size);

        self.size_task = ctx
            .spawn_task(move |mut ctx| async move {
                while let Ok(new_size) = size_rx.recv().await {
                    let mut size = size.lock();

                    if *size == new_size {
                        continue;
                    }

                    *size = new_size;
                    ctx.mark_needs_layout();
                }
            })
            .ok();
    }
}

impl RenderObjectImpl for RenderWinitWindowLayout {
    fn intrinsic_size(
        &self,
        ctx: &mut RenderObjectIntrinsicSizeContext,
        dimension: IntrinsicDimension,
        cross_extent: f32,
    ) -> f32 {
        ctx.iter_children().next().map_or(0.0, |child| {
            child.compute_intrinsic_size(dimension, cross_extent)
        })
    }

    fn layout(&self, ctx: &mut RenderObjectLayoutContext, _: Constraints) -> Size {
        let size = *self.size.lock();

        let mut children = ctx.iter_children_mut();

        while let Some(mut child) = children.next() {
            child.layout(Constraints::from(size));
        }

        size
    }
}
