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
    unit::{Data, Rect},
};

use crate::widget::{BuildContext, BuildResult, WidgetDispatch, WidgetRef, WidgetView};

use super::{dispatch::WidgetEquality, LayoutContext, LayoutResult, WidgetState};

pub struct WidgetInstance<W>
where
    W: WidgetView + WidgetState,
{
    widget: Rc<W>,
    state: W::State,

    renderer: Option<RenderFn<W>>,

    callbacks: FnvHashMap<CallbackId, Box<dyn CallbackFunc<W>>>,
}

impl<W> WidgetInstance<W>
where
    W: WidgetView + WidgetState,
{
    pub fn new(widget: Rc<W>) -> Self {
        let state = widget.create_state();

        Self {
            widget,
            state,

            renderer: None,

            callbacks: FnvHashMap::default(),
        }
    }

    pub fn get_widget(&self) -> &W {
        &self.widget
    }
}

impl<W> WidgetDispatch for WidgetInstance<W>
where
    W: WidgetView + WidgetState,
{
    fn is_similar(&self, other: &WidgetRef) -> WidgetEquality {
        if let Some(other) = other.downcast_rc::<W>() {
            if self.widget == other {
                WidgetEquality::Equal
            } else {
                WidgetEquality::Unequal
            }
        } else {
            WidgetEquality::Invalid
        }
    }

    fn update(&mut self, other: WidgetRef) -> bool {
        let other = other
            .downcast_rc::<W>()
            .expect("cannot update a widget instance with a different type");

        let needs_build = self.widget.updated(&other);

        self.widget = other;

        needs_build
    }

    fn layout(&mut self, ctx: AguiContext) -> LayoutResult {
        let span = tracing::error_span!("layout");
        let _enter = span.enter();

        let mut ctx = LayoutContext {
            plugins: ctx.plugins.unwrap(),
            widget_tree: ctx.tree,

            widget_id: ctx.widget_id.unwrap(),
            widget: self.widget.as_ref(),
            state: &mut self.state,
        };

        self.widget.layout(&mut ctx)
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

            renderer: None,
            callbacks: FnvHashMap::default(),
        };

        let result = self.widget.build(&mut ctx);

        self.renderer = ctx.renderer;
        self.callbacks = ctx.callbacks;

        result
    }

    fn render(&self, rect: Rect) -> Option<Canvas> {
        let span = tracing::error_span!("on_draw");
        let _enter = span.enter();

        self.renderer.as_ref().map(|renderer| {
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

    fn call(&mut self, ctx: AguiContext, callback_id: CallbackId, arg: &Box<dyn Data>) -> bool {
        let span = tracing::error_span!("callback");
        let _enter = span.enter();

        if let Some(callback) = self.callbacks.get(&callback_id) {
            let mut ctx = CallbackContext {
                plugins: ctx.plugins.unwrap(),
                widget_tree: ctx.tree,
                dirty: ctx.dirty,
                callback_queue: ctx.callback_queue,

                widget_id: ctx.widget_id.unwrap(),
                widget: self.widget.as_ref(),
                state: &mut self.state,

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

impl<W> std::fmt::Debug for WidgetInstance<W>
where
    W: WidgetView + WidgetState + std::fmt::Debug,
    <W>::State: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WidgetInstance")
            .field("widget", &self.widget)
            .field("state", &self.state)
            .finish()
    }
}
