use std::any::{type_name, TypeId};

use downcast_rs::{impl_downcast, Downcast};
use fnv::FnvHashMap;

use crate::{
    callback::{CallbackContext, CallbackFunc, CallbackId},
    canvas::{context::RenderContext, renderer::RenderFn, Canvas},
    unit::{Layout, LayoutType, Rect},
    widget::{BuildContext, BuildResult, Widget, WidgetId},
};

use super::{context::EngineContext, Data};

pub trait WidgetImpl: std::fmt::Debug + Downcast {
    fn get_type_id(&self) -> TypeId;
    fn get_display_name(&self) -> String;

    fn get_layout_type(&self) -> Option<LayoutType>;
    fn get_layout(&self) -> Option<Layout>;

    fn set_rect(&mut self, rect: Option<Rect>);
    fn get_rect(&self) -> Option<Rect>;

    fn build(&mut self, ctx: EngineContext, widget_id: WidgetId) -> BuildResult;

    fn call(&mut self, ctx: EngineContext, callback_id: CallbackId, arg: &dyn Data) -> bool;

    fn render(&self, canvas: &mut Canvas);
}

impl_downcast!(WidgetImpl);

/// Implements the widget's `build()` method.
pub trait WidgetBuilder: std::fmt::Debug + Downcast + Sized {
    type State: Data + Default;

    /// Called whenever this widget is rebuilt.
    ///
    /// This method may be called when any parent is rebuilt or when its internal state changes.
    fn build(&self, ctx: &mut BuildContext<Self>) -> BuildResult;
}

#[derive(Default)]
pub struct WidgetElement<W>
where
    W: WidgetBuilder,
{
    widget: W,
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
    pub fn new(widget: W) -> Self {
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

impl<W> WidgetImpl for WidgetElement<W>
where
    W: WidgetBuilder,
{
    fn get_type_id(&self) -> TypeId {
        TypeId::of::<W>()
    }

    fn get_display_name(&self) -> String {
        let type_name = type_name::<W>();

        if !type_name.contains('<') {
            String::from(type_name.rsplit("::").next().unwrap())
        } else {
            let mut name = String::new();

            let mut remaining = String::from(type_name);

            while let Some((part, rest)) = remaining.split_once("<") {
                name.push_str(part.rsplit("::").next().unwrap());

                name.push('<');

                remaining = String::from(rest);
            }

            name.push_str(remaining.rsplit("::").next().unwrap());

            name
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

    fn build(&mut self, ctx: EngineContext, widget_id: WidgetId) -> BuildResult {
        let span = tracing::error_span!("build");
        let _enter = span.enter();

        let mut ctx = BuildContext {
            plugins: ctx.plugins.unwrap(),
            tree: ctx.tree,
            dirty: ctx.dirty,
            callback_queue: ctx.callback_queue,

            widget_id,
            widget: &self.widget,
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

    fn call(&mut self, ctx: EngineContext, callback_id: CallbackId, arg: &dyn Data) -> bool {
        let span = tracing::error_span!("callback");
        let _enter = span.enter();

        if let Some(callback) = self.callbacks.get(&callback_id) {
            let mut ctx = CallbackContext {
                plugins: ctx.plugins.unwrap(),
                tree: ctx.tree,
                dirty: ctx.dirty,
                callback_queue: ctx.callback_queue,

                widget: &self.widget,
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

    fn render(&self, canvas: &mut Canvas) {
        let span = tracing::error_span!("on_draw");
        let _enter = span.enter();

        if let Some(renderer) = &self.renderer {
            let ctx = RenderContext {
                widget: &self.widget,
                state: &self.state,
            };

            renderer.call(&ctx, canvas);
        }
    }
}

impl<W> std::fmt::Debug for WidgetElement<W>
where
    W: WidgetBuilder,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WidgetElement")
            .field("widget", &self.widget)
            .field("state", &self.state)
            .finish()
    }
}

impl<W> From<W> for WidgetElement<W>
where
    W: WidgetBuilder,
{
    fn from(widget: W) -> Self {
        Self::new(widget)
    }
}

impl<W> From<W> for Widget
where
    W: WidgetBuilder,
{
    fn from(widget: W) -> Self {
        Self::new(None, WidgetElement::from(widget))
    }
}
