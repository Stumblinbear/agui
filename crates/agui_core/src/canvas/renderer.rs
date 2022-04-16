use crate::engine::widget::WidgetBuilder;

use super::{context::RenderContext, Canvas};

pub struct RenderFn<W>
where
    W: WidgetBuilder,
{
    func: Box<dyn Fn(&RenderContext<W>, &mut Canvas)>,
}

impl<W> RenderFn<W>
where
    W: WidgetBuilder,
{
    pub fn new<F>(func: F) -> Self
    where
        F: Fn(&RenderContext<W>, &mut Canvas) + 'static,
    {
        Self {
            func: Box::new(func),
        }
    }

    pub fn call(&self, ctx: &RenderContext<W>, canvas: &mut Canvas) {
        let span = tracing::trace_span!("render_fn");
        let _enter = span.enter();

        (self.func)(ctx, canvas);
    }
}
