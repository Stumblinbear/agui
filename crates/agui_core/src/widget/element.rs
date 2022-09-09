use std::{
    any::{type_name, TypeId},
    rc::Rc,
};

use fnv::FnvHashMap;

use crate::{
    callback::{CallbackContext, CallbackFunc, CallbackId},
    manager::context::AguiContext,
    render::{
        canvas::{painter::CanvasPainter, Canvas},
        context::RenderContext,
        renderer::RenderFn,
    },
    unit::{Data, Layout, LayoutType, Rect},
};

use super::{
    descriptor::WidgetDescriptor, BuildContext, BuildResult, WidgetBuilder, WidgetInstance,
};

pub struct WidgetElement<W>
where
    W: WidgetBuilder,
{
    desc: WidgetDescriptor,

    widget: Rc<W>,
    state: W::State,

    layout_type: LayoutType,
    layout: Layout,

    canvas: Option<Canvas>,
    renderer: Option<RenderFn<W>>,

    callbacks: FnvHashMap<CallbackId, Box<dyn CallbackFunc<W>>>,

    rect: Option<Rect>,
}

impl<W> WidgetElement<W>
where
    W: WidgetBuilder,
{
    pub fn new(desc: WidgetDescriptor, widget: Rc<W>) -> Self {
        Self {
            desc,

            widget,
            state: W::State::default(),

            layout_type: LayoutType::default(),
            layout: Layout::default(),

            canvas: None,
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
    fn get_descriptor(&self) -> &WidgetDescriptor {
        &self.desc
    }

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

            while let Some((part, rest)) = remaining.split_once('<') {
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

    fn build(&mut self, ctx: AguiContext) -> BuildResult {
        let span = tracing::error_span!("build");
        let _enter = span.enter();

        let mut ctx = BuildContext {
            plugins: ctx.plugins.unwrap(),
            tree: ctx.tree,
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

    fn get_canvas(&self) -> Option<&Canvas> {
        self.canvas.as_ref()
    }

    fn render(&mut self, rect: Rect) {
        let span = tracing::error_span!("on_draw");
        let _enter = span.enter();

        if let Some(renderer) = &self.renderer {
            let mut canvas = Canvas {
                rect,

                ..Canvas::default()
            };

            let ctx = RenderContext {
                widget: self.widget.as_ref(),
                state: &self.state,
            };

            renderer.call(&ctx, CanvasPainter::new(&mut canvas));
        } else if self.canvas.is_some() {
            self.canvas = None;
        }
    }

    fn call(&mut self, ctx: AguiContext, callback_id: CallbackId, arg: &dyn Data) -> bool {
        let span = tracing::error_span!("callback");
        let _enter = span.enter();

        if let Some(callback) = self.callbacks.get(&callback_id) {
            let mut ctx = CallbackContext {
                plugins: ctx.plugins.unwrap(),
                tree: ctx.tree,
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
