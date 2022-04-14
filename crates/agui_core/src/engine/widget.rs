use std::{any::TypeId, rc::Rc};

use downcast_rs::{impl_downcast, Downcast};
use fnv::{FnvHashMap, FnvHashSet};

use crate::{
    callback::{CallbackContext, CallbackFunc, CallbackId},
    canvas::renderer::RenderFn,
    plugin::{Plugin, PluginId},
    unit::{Layout, LayoutType, Rect},
    widget::{BuildContext, BuildResult, Widget, WidgetId},
};

use super::{tree::Tree, Data, NotifyCallback};

pub trait WidgetImpl: std::fmt::Debug + Downcast {
    fn get_type_id(&self) -> TypeId;

    fn get_layout_type(&self) -> Option<LayoutType>;
    fn get_layout(&self) -> Option<Layout>;

    fn set_rect(&mut self, rect: Option<Rect>);

    fn build(
        &mut self,
        plugins: &mut FnvHashMap<PluginId, Plugin>,
        tree: &Tree<WidgetId, Widget>,
        dirty: &mut FnvHashSet<WidgetId>,
        notifier: NotifyCallback,
        widget_id: WidgetId,
    ) -> BuildResult;

    fn call(
        &mut self,
        notifier: NotifyCallback,
        callback_id: CallbackId,
        arg: Rc<dyn Data>,
    ) -> bool;
}

impl_downcast!(WidgetImpl);

/// Implements the widget's `build()` method.
pub trait WidgetBuilder: std::fmt::Debug + Downcast {
    type State: Data + Default;

    /// Called whenever this widget is rebuilt.
    ///
    /// This method may be called when any parent is rebuilt or when its internal state changes.
    fn build(&self, ctx: &mut BuildContext<Self::State>) -> BuildResult;
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

    renderer: Option<RenderFn>,
    callbacks: FnvHashMap<CallbackId, Box<dyn CallbackFunc<W::State>>>,

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

    fn get_layout_type(&self) -> Option<LayoutType> {
        Some(self.layout_type)
    }

    fn get_layout(&self) -> Option<Layout> {
        Some(self.layout)
    }

    fn set_rect(&mut self, rect: Option<Rect>) {
        self.rect = rect;
    }

    fn build(
        &mut self,
        plugins: &mut FnvHashMap<PluginId, Plugin>,
        tree: &Tree<WidgetId, Widget>,
        dirty: &mut FnvHashSet<WidgetId>,
        notifier: NotifyCallback,
        widget_id: WidgetId,
    ) -> BuildResult {
        let mut ctx = BuildContext {
            plugins,
            tree,
            dirty,
            notifier,

            widget_id,
            state: &mut self.state,

            layout_type: LayoutType::default(),
            layout: Layout::default(),
            rect: None,

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

    fn call(
        &mut self,
        notifier: NotifyCallback,
        callback_id: CallbackId,
        arg: Rc<dyn Data>,
    ) -> bool {
        let mut ctx = CallbackContext {
            state: &mut self.state,
            changed: false,
        };

        self.callbacks
            .get(&callback_id)
            .expect("callback not found")
            .call(&mut ctx, arg);

        ctx.changed
    }
}

impl<W> std::fmt::Debug for WidgetElement<W>
where
    W: WidgetBuilder,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Widget")
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
