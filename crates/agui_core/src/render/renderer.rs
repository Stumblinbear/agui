use crate::widget::WidgetBuilder;

use super::{
    canvas::painter::{CanvasPainter, Head},
    context::RenderContext,
};

pub struct RenderFn<W>
where
    W: WidgetBuilder,
{
    #[allow(clippy::type_complexity)]
    func: Box<dyn Fn(&RenderContext<W>, CanvasPainter<Head>)>,
}

impl<W> RenderFn<W>
where
    W: WidgetBuilder,
{
    pub fn new<F>(func: F) -> Self
    where
        F: Fn(&RenderContext<W>, CanvasPainter<Head>) + 'static,
    {
        Self {
            func: Box::new(func),
        }
    }

    pub fn call(&self, ctx: &RenderContext<W>, canvas: CanvasPainter<Head>) {
        let span = tracing::trace_span!("render_fn");
        let _enter = span.enter();

        (self.func)(ctx, canvas);
    }
}
