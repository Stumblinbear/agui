use crate::widget::WidgetImpl;

use super::{canvas::painter::CanvasPainter, context::RenderContext};

pub struct RenderFn<W>
where
    W: WidgetImpl,
{
    func: Box<dyn Fn(&RenderContext<W>, &mut CanvasPainter)>,
}

impl<W> RenderFn<W>
where
    W: WidgetImpl,
{
    pub fn new<F>(func: F) -> Self
    where
        F: Fn(&RenderContext<W>, &mut CanvasPainter) + 'static,
    {
        Self {
            func: Box::new(func),
        }
    }

    pub fn call(&self, ctx: &RenderContext<W>, canvas: &mut CanvasPainter) {
        let span = tracing::trace_span!("render_fn");
        let _enter = span.enter();

        (self.func)(ctx, canvas);
    }
}
