use std::any::TypeId;

use crate::{
    callback::CallbackId,
    manager::context::AguiContext,
    render::canvas::Canvas,
    unit::{Data, Layout, LayoutType, Rect},
    widget::{BuildResult, IntoWidget, WidgetDispatch, WidgetKey, WidgetRef},
};

pub struct WidgetElement {
    widget_ref: WidgetRef,
    dispatch: Box<dyn WidgetDispatch>,

    layout_type: LayoutType,
    layout: Layout,

    rect: Option<Rect>,
}

impl WidgetElement {
    pub(super) fn new(widget_ref: WidgetRef) -> Option<Self> {
        widget_ref.create().map(|dispatch| Self {
            widget_ref,
            dispatch,

            layout_type: LayoutType::default(),
            layout: Layout::default(),

            rect: None,
        })
    }

    pub fn get_type_id(&self) -> TypeId {
        self.widget_ref.get_type_id().unwrap()
    }

    pub fn get_display_name(&self) -> &str {
        self.widget_ref.get_display_name().unwrap()
    }

    pub fn get_key(&self) -> Option<&WidgetKey> {
        self.widget_ref.get_key()
    }

    pub fn get_ref(&self) -> &WidgetRef {
        &self.widget_ref
    }

    pub fn downcast_ref<W>(&self) -> Option<&W>
    where
        W: WidgetDispatch,
    {
        self.dispatch.downcast_ref()
    }

    pub fn get_layout_type(&self) -> LayoutType {
        self.layout_type
    }

    pub fn get_layout(&self) -> Layout {
        self.layout
    }

    pub fn get_rect(&self) -> Option<Rect> {
        self.rect
    }

    pub fn set_rect(&mut self, rect: Option<Rect>) {
        self.rect = rect;
    }

    pub fn is_similar(&self, other: &WidgetRef) -> bool {
        self.dispatch.is_similar(other)
    }

    pub fn build(&mut self, ctx: AguiContext) -> BuildResult {
        let result = self.dispatch.build(ctx);

        self.layout_type = result.layout_type;
        self.layout = result.layout;

        result
    }

    pub fn render(&self) -> Option<Canvas> {
        self.rect.and_then(|rect| self.dispatch.render(rect))
    }

    pub fn call(&mut self, ctx: AguiContext, callback_id: CallbackId, arg: &dyn Data) -> bool {
        self.dispatch.call(ctx, callback_id, arg)
    }
}

impl std::fmt::Debug for WidgetElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.widget_ref.fmt(f)
    }
}

impl std::fmt::Display for WidgetElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.widget_ref.fmt(f)
    }
}

impl<W> From<W> for WidgetElement
where
    W: IntoWidget,
{
    fn from(widget: W) -> Self {
        WidgetElement::new(widget.into()).unwrap()
    }
}
