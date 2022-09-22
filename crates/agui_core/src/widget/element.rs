use std::rc::Rc;

use fnv::FnvHashMap;

use crate::{
    callback::{CallbackContext, CallbackFunc, CallbackId},
    manager::context::AguiContext,
    render::{
        canvas::{
            painter::{CanvasPainter, Head},
            Canvas,
        },
        context::RenderContext,
        renderer::RenderFn,
    },
    unit::{Data, Layout, LayoutType, Rect},
};

use super::{BuildContext, BuildResult, WidgetBuilder, WidgetInstance, WidgetRef};

pub struct WidgetElement<W>
where
    W: WidgetBuilder,
{
    widget: Rc<W>,
    state: W::State,

    layout_type: LayoutType,
    layout: Layout,

    renderer: Option<RenderFn<W>>,

    callbacks: FnvHashMap<CallbackId, Box<dyn CallbackFunc<W>>>,

    rect: Option<Rect>,
}

impl<W> WidgetElement<W>
where
    W: WidgetBuilder,
{
    pub fn new(widget: Rc<W>) -> Self {
        Self {
            widget,
            state: W::State::default(),

            layout_type: LayoutType::default(),
            layout: Layout::default(),

            renderer: None,

            callbacks: FnvHashMap::default(),

            rect: None,
        }
    }
}

impl<W> WidgetElement<W>
where
    W: WidgetBuilder,
{
    pub fn get_widget(&self) -> &W {
        &self.widget
    }

    pub fn get_state(&self) -> &W::State {
        &self.state
    }
}

impl<W> WidgetInstance for WidgetElement<W>
where
    W: WidgetBuilder,
{
    fn is_similar(&self, other: &WidgetRef) -> bool {
        if let Some(other) = other.downcast_ref::<W>() {
            self.widget == other
        } else {
            false
        }
    }

    fn get_layout_type(&self) -> Option<LayoutType> {
        Some(self.layout_type)
    }

    fn get_layout(&self) -> Option<Layout> {
        Some(self.layout)
    }

    fn set_rect(&mut self, rect: Option<Rect>) {
        self.rect = rect;
    }

    fn get_rect(&self) -> Option<Rect> {
        self.rect
    }

    fn build(&mut self, ctx: AguiContext) -> BuildResult {
        let span = tracing::error_span!("build");
        let _enter = span.enter();

        let mut ctx = BuildContext {
            plugins: ctx.plugins.unwrap(),
            widget_tree: ctx.tree,
            dirty: ctx.dirty,
            callback_queue: ctx.callback_queue,

            widget_id: ctx.widget_id.unwrap(),
            widget: self.widget.as_ref(),
            state: &mut self.state,

            layout_type: LayoutType::default(),
            layout: Layout::default(),
            rect: self.rect,

            renderer: None,
            callbacks: FnvHashMap::default(),
        };

        let result = self.widget.build(&mut ctx);

        self.layout_type = ctx.layout_type;
        self.layout = ctx.layout;

        self.renderer = ctx.renderer;
        self.callbacks = ctx.callbacks;

        result
    }

    fn render(&self) -> Option<Canvas> {
        let span = tracing::error_span!("on_draw");
        let _enter = span.enter();

        self.renderer
            .as_ref()
            .zip(self.rect)
            .map(|(renderer, rect)| {
                let mut canvas = Canvas {
                    rect,

                    ..Canvas::default()
                };

                let ctx = RenderContext {
                    widget: self.widget.as_ref(),
                    state: &self.state,
                };

                renderer.call(&ctx, CanvasPainter::<Head>::new(&mut canvas));

                canvas
            })
    }

    fn call(&mut self, ctx: AguiContext, callback_id: CallbackId, arg: &dyn Data) -> bool {
        let span = tracing::error_span!("callback");
        let _enter = span.enter();

        if let Some(callback) = self.callbacks.get(&callback_id) {
            let mut ctx = CallbackContext {
                plugins: ctx.plugins.unwrap(),
                widget_tree: ctx.tree,
                dirty: ctx.dirty,
                callback_queue: ctx.callback_queue,

                widget: self.widget.as_ref(),
                state: &mut self.state,

                rect: self.rect,

                changed: false,
            };

            callback.call(&mut ctx, arg);

            ctx.changed
        } else {
            tracing::warn!(
                callback_id = format!("{:?}", callback_id).as_str(),
                "callback not found"
            );

            false
        }
    }
}

impl<W> std::fmt::Debug for WidgetElement<W>
where
    W: WidgetBuilder + std::fmt::Debug,
    <W>::State: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WidgetElement")
            .field("widget", &self.widget)
            .field("state", &self.state)
            .finish()
    }
}
