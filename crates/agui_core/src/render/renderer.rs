use crate::widget::WidgetBuilder;

use super::{canvas::painter::CanvasPainter, context::RenderContext};

pub struct RenderFn<W>
where
    W: WidgetBuilder,
{
    func: Box<dyn Fn(&RenderContext<W>, CanvasPainter)>,
}

impl<W> RenderFn<W>
where
    W: WidgetBuilder,
{
    pub fn new<F>(func: F) -> Self
    where
        F: Fn(&RenderContext<W>, CanvasPainter) + 'static,
    {
        Self {
            func: Box::new(func),
        }
    }

    pub fn call(&self, ctx: &RenderContext<W>, canvas: CanvasPainter) {
        let span = tracing::trace_span!("render_fn");
        let _enter = span.enter();

        (self.func)(ctx, canvas);
    }
}
