use std::{
    any::{type_name, TypeId},
    cell::{Ref, RefCell, RefMut},
    rc::Rc,
};

use fnv::FnvHashMap;
use slotmap::new_key_type;

use crate::{
    callback::{CallbackContext, CallbackFunc, CallbackId},
    canvas::{context::RenderContext, renderer::RenderFn, Canvas},
    unit::{Layout, LayoutType, Rect},
    util::tree::Tree,
    widget::{BuildContext, BuildResult, WidgetBuilder, WidgetImpl, WidgetKey},
};

use super::{context::AguiContext, Data};

new_key_type! {
    pub struct WidgetId;
}

impl<'ui> morphorm::Node<'ui> for WidgetId {
    type Data = Tree<Self, Widget>;

    fn layout_type(&self, store: &'_ Self::Data) -> Option<morphorm::LayoutType> {
        store
            .get(*self)
            .and_then(Widget::get)
            .and_then(|node| node.get_layout_type())
            .map(Into::into)
    }

    fn position_type(&self, store: &'_ Self::Data) -> Option<morphorm::PositionType> {
        store
            .get(*self)
            .and_then(Widget::get)
            .and_then(|node| node.get_layout())
            .map(|layout| layout.position.into())
    }

    fn width(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(Widget::get)
            .and_then(|node| node.get_layout())
            .map(|layout| layout.sizing.get_width().into())
    }

    fn height(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(Widget::get)
            .and_then(|node| node.get_layout())
            .map(|layout| layout.sizing.get_height().into())
    }

    fn min_width(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(Widget::get)
            .and_then(|node| node.get_layout())
            .map(|layout| layout.min_sizing.get_width().into())
    }

    fn min_height(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(Widget::get)
            .and_then(|node| node.get_layout())
            .map(|layout| layout.min_sizing.get_height().into())
    }

    fn max_width(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(Widget::get)
            .and_then(|node| node.get_layout())
            .map(|layout| layout.max_sizing.get_width().into())
    }

    fn max_height(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(Widget::get)
            .and_then(|node| node.get_layout())
            .map(|layout| layout.max_sizing.get_height().into())
    }

    fn top(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(Widget::get)
            .and_then(|node| node.get_layout())
            .and_then(|layout| layout.position.get_top())
            .map(Into::into)
    }

    fn right(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(Widget::get)
            .and_then(|node| node.get_layout())
            .and_then(|layout| layout.position.get_right())
            .map(Into::into)
    }

    fn bottom(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(Widget::get)
            .and_then(|node| node.get_layout())
            .and_then(|layout| layout.position.get_bottom())
            .map(Into::into)
    }

    fn left(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(Widget::get)
            .and_then(|node| node.get_layout())
            .and_then(|layout| layout.position.get_left())
            .map(Into::into)
    }

    fn min_top(&self, _store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(morphorm::Units::Auto)
    }

    fn max_top(&self, _store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(morphorm::Units::Auto)
    }

    fn min_right(&self, _store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(morphorm::Units::Auto)
    }

    fn max_right(&self, _store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(morphorm::Units::Auto)
    }

    fn min_bottom(&self, _store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(morphorm::Units::Auto)
    }

    fn max_bottom(&self, _store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(morphorm::Units::Auto)
    }

    fn min_left(&self, _store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(morphorm::Units::Auto)
    }

    fn max_left(&self, _store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(morphorm::Units::Auto)
    }

    fn child_top(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(Widget::get)
            .and_then(|node| node.get_layout())
            .map(|layout| layout.margin.get_top().into())
    }

    fn child_right(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(Widget::get)
            .and_then(|node| node.get_layout())
            .map(|layout| layout.margin.get_right().into())
    }

    fn child_bottom(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(Widget::get)
            .and_then(|node| node.get_layout())
            .map(|layout| layout.margin.get_bottom().into())
    }

    fn child_left(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(Widget::get)
            .and_then(|node| node.get_layout())
            .map(|layout| layout.margin.get_left().into())
    }

    fn row_between(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(Widget::get)
            .and_then(|node| node.get_layout_type())
            .and_then(|layout_type| layout_type.get_column_spacing())
            .map(Into::into)
    }

    fn col_between(&self, store: &'_ Self::Data) -> Option<morphorm::Units> {
        store
            .get(*self)
            .and_then(Widget::get)
            .and_then(|node| node.get_layout_type())
            .and_then(|layout_type| layout_type.get_row_spacing())
            .map(Into::into)
    }

    fn grid_rows(&self, store: &'_ Self::Data) -> Option<Vec<morphorm::Units>> {
        store
            .get(*self)
            .and_then(Widget::get)
            .and_then(|node| node.get_layout_type())
            .and_then(|layout_type| layout_type.get_rows())
            .map(|val| val.into_iter().map(Into::into).collect())
    }

    fn grid_cols(&self, store: &'_ Self::Data) -> Option<Vec<morphorm::Units>> {
        store
            .get(*self)
            .and_then(Widget::get)
            .and_then(|node| node.get_layout_type())
            .and_then(|layout_type| layout_type.get_columns())
            .map(|val| val.into_iter().map(Into::into).collect())
    }

    fn row_index(&self, _store: &'_ Self::Data) -> Option<usize> {
        Some(0)
    }

    fn col_index(&self, _store: &'_ Self::Data) -> Option<usize> {
        Some(0)
    }

    fn row_span(&self, _store: &'_ Self::Data) -> Option<usize> {
        Some(1)
    }

    fn col_span(&self, _store: &'_ Self::Data) -> Option<usize> {
        Some(1)
    }

    fn border_top(&self, _store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(morphorm::Units::Auto)
    }

    fn border_right(&self, _store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(morphorm::Units::Auto)
    }

    fn border_bottom(&self, _store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(morphorm::Units::Auto)
    }

    fn border_left(&self, _store: &'_ Self::Data) -> Option<morphorm::Units> {
        Some(morphorm::Units::Auto)
    }
}

#[derive(Clone)]
pub enum Widget {
    None,

    Some {
        key: Option<WidgetKey>,
        inner: Rc<RefCell<dyn WidgetImpl>>,
    },
}

impl Default for Widget {
    fn default() -> Self {
        Self::None
    }
}

impl Widget {
    pub(crate) fn new<W>(key: Option<WidgetKey>, widget: W) -> Self
    where
        W: WidgetImpl,
    {
        Self::Some {
            key,
            inner: Rc::new(RefCell::new(widget)),
        }
    }

    pub fn get_key(&self) -> Option<WidgetKey> {
        if let Widget::Some { key, .. } = self {
            return *key;
        }

        None
    }

    pub fn is_empty(&self) -> bool {
        matches!(self, Widget::None)
    }

    pub fn get(&self) -> Option<Ref<dyn WidgetImpl>> {
        match self {
            Widget::None => None,
            Widget::Some { inner, .. } => Some(inner.borrow()),
        }
    }

    pub fn get_mut(&self) -> Option<RefMut<dyn WidgetImpl>> {
        match self {
            Widget::None => None,
            Widget::Some { inner, .. } => Some(inner.borrow_mut()),
        }
    }

    pub fn get_as<W>(&self) -> Option<Ref<WidgetElement<W>>>
    where
        W: WidgetBuilder,
    {
        if let Widget::Some { inner, .. } = self {
            let widget = RefCell::borrow(inner);

            if widget.get_type_id() == TypeId::of::<W>() {
                return Some(Ref::map(widget, |x| x.downcast_ref().unwrap()));
            }
        }

        None
    }

    pub fn get_as_mut<W>(&self) -> Option<RefMut<WidgetElement<W>>>
    where
        W: WidgetBuilder,
    {
        if let Widget::Some { inner, .. } = self {
            let widget = RefCell::borrow_mut(inner);

            if widget.get_type_id() == TypeId::of::<W>() {
                return Some(RefMut::map(widget, |x| x.downcast_mut().unwrap()));
            }
        }

        None
    }
}

impl std::fmt::Debug for Widget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "None"),
            Self::Some { key, inner } => f
                .debug_struct("Widget")
                .field("key", &key)
                .field("inner", &inner.borrow())
                .finish(),
        }
    }
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

    fn build(&mut self, ctx: AguiContext) -> BuildResult {
        let span = tracing::error_span!("build");
        let _enter = span.enter();

        let mut ctx = BuildContext {
            plugins: ctx.plugins.unwrap(),
            tree: ctx.tree,
            dirty: ctx.dirty,
            callback_queue: ctx.callback_queue,

            widget_id: ctx.widget_id.unwrap(),
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

    fn call(&mut self, ctx: AguiContext, callback_id: CallbackId, arg: &dyn Data) -> bool {
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
