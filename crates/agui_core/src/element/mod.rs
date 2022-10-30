use std::rc::Rc;

use enum_dispatch::enum_dispatch;
use slotmap::new_key_type;

use crate::{
    callback::CallbackId,
    inheritance::Inheritance,
    manager::context::AguiContext,
    render::canvas::Canvas,
    unit::{Data, Layout, LayoutType, Rect, Size},
    widget::{
        instance::{WidgetDispatch, WidgetEquality},
        key::WidgetKey,
        BuildResult, InheritedWidget, LayoutResult, Widget, WidgetRef, WidgetView,
    },
};

use self::{
    context::ElementContext, inherited::InheritedElement, stateful::StatefulElement,
    stateless::StatelessElement,
};

pub mod context;
mod inherited;
mod stateful;
mod stateless;

new_key_type! {
    pub struct ElementId;
}

#[enum_dispatch]
pub trait ElementLifecycle: std::fmt::Debug + std::ops::Deref<Target = dyn WidgetDispatch> {
    fn mount(&mut self, ctx: ElementContext);

    fn unmount(&mut self, ctx: ElementContext);

    fn update(&mut self, other: WidgetRef) -> bool;

    fn layout(&mut self, ctx: ElementContext) -> LayoutResult;

    fn build(&mut self, ctx: ElementContext) -> BuildResult;

    fn paint(&self, size: Size) -> Option<Canvas>;

    #[allow(clippy::borrowed_box)]
    fn call(&mut self, ctx: ElementContext, callback_id: CallbackId, arg: &Box<dyn Data>) -> bool;
}

#[enum_dispatch(ElementLifecycle)]
pub enum ElementType {
    Stateless(StatelessElement),
    Stateful(StatefulElement),
    Inherited(InheritedElement),
}

impl ElementType {
    pub fn new_stateless<W>(widget: Rc<W>) -> ElementType
    where
        W: WidgetView,
    {
        ElementType::Stateless(StatelessElement::new(widget))
    }

    pub fn new_stateful<W>(widget: Rc<W>) -> ElementType
    where
        W: WidgetView,
    {
        ElementType::Stateless(StatelessElement::new(widget))
    }

    pub fn new_inherited<W>(widget: Rc<W>) -> ElementType
    where
        W: WidgetView + InheritedWidget,
    {
        ElementType::Stateless(StatelessElement::new(widget))
    }
}

impl std::ops::Deref for ElementType {
    type Target = dyn WidgetDispatch;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Stateless(e) => &**e,
            Self::Stateful(e) => &**e,
            Self::Inherited(e) => &**e,
        }
    }
}

impl std::fmt::Debug for ElementType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stateless(e) => e.fmt(f),
            Self::Stateful(e) => e.fmt(f),
            Self::Inherited(e) => e.fmt(f),
        }
    }
}

pub struct Element {
    key: Option<WidgetKey>,

    layout_type: LayoutType,
    layout: Layout,

    rect: Option<Rect>,

    lifecycle: ElementType,

    inheritance: Inheritance,
}

impl Element {
    pub(crate) fn new(key: Option<WidgetKey>, lifecycle: ElementType) -> Self {
        Self {
            key,

            layout_type: LayoutType::default(),
            layout: Layout::default(),

            rect: None,

            lifecycle,

            inheritance: Inheritance::default(),
        }
    }

    pub fn get_display_name(&self) -> String {
        self.lifecycle.get_display_name()
    }

    pub fn get_key(&self) -> Option<&WidgetKey> {
        self.key.as_ref()
    }

    pub fn get_layout_type(&self) -> &LayoutType {
        &self.layout_type
    }

    pub fn get_layout(&self) -> &Layout {
        &self.layout
    }

    pub fn get_rect(&self) -> Option<&Rect> {
        self.rect.as_ref()
    }

    pub fn set_rect(&mut self, rect: Option<Rect>) {
        self.rect = rect;
    }

    pub fn get_widget<W>(&self) -> Option<Rc<W>>
    where
        W: Widget,
    {
        self.lifecycle.get_widget().downcast_rc().ok()
    }

    pub fn get_state<W>(&self) -> Option<&W::State>
    where
        W: Widget,
    {
        self.lifecycle.get_state().downcast_ref()
    }

    pub fn is_similar(&self, other: &WidgetRef) -> WidgetEquality {
        self.lifecycle.is_similar(other)
    }

    pub fn mount(&mut self, ctx: AguiContext) {
        let span = tracing::error_span!("mount");
        let _enter = span.enter();
    }

    pub fn unmount(&mut self, ctx: AguiContext) {
        let span = tracing::error_span!("unmount");
        let _enter = span.enter();
    }

    pub fn update(&mut self, other: WidgetRef) -> bool {
        let span = tracing::error_span!("update");
        let _enter = span.enter();

        self.lifecycle.update(other)
    }

    pub fn layout(&mut self, ctx: AguiContext) {
        let span = tracing::error_span!("layout");
        let _enter = span.enter();

        let layout = self.lifecycle.layout(ElementContext {
            element_tree: ctx.element_tree,
            dirty: ctx.dirty,
            callback_queue: ctx.callback_queue,

            element_id: ctx.element_id,

            inheritance: &mut self.inheritance,
        });

        self.layout_type = layout.layout_type;
        self.layout = layout.layout;
    }

    pub fn build(&mut self, ctx: AguiContext) -> BuildResult {
        let span = tracing::error_span!("build");
        let _enter = span.enter();

        self.lifecycle.build(ElementContext {
            element_tree: ctx.element_tree,
            dirty: ctx.dirty,
            callback_queue: ctx.callback_queue,

            element_id: ctx.element_id,

            inheritance: &mut self.inheritance,
        })
    }

    pub fn paint(&self) -> Option<Canvas> {
        let span = tracing::error_span!("paint");
        let _enter = span.enter();

        self.rect
            .and_then(|rect| self.lifecycle.paint(Size::from(rect)))
    }

    #[allow(clippy::borrowed_box)]
    pub fn call(&mut self, ctx: AguiContext, callback_id: CallbackId, arg: &Box<dyn Data>) -> bool {
        let span = tracing::error_span!("callback");
        let _enter = span.enter();

        self.lifecycle.call(
            ElementContext {
                element_tree: ctx.element_tree,
                dirty: ctx.dirty,
                callback_queue: ctx.callback_queue,

                element_id: ctx.element_id,

                inheritance: &mut self.inheritance,
            },
            callback_id,
            arg,
        )
    }
}

impl std::fmt::Debug for Element {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.lifecycle.fmt(f)
    }
}
