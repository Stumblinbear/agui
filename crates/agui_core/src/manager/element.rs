use std::any::TypeId;

use crate::{
    callback::CallbackId,
    manager::context::AguiContext,
    render::canvas::Canvas,
    unit::{Data, Layout, LayoutType, Rect, Size},
    widget::{
        dispatch::{WidgetDispatch, WidgetEquality},
        key::WidgetKey,
        BuildResult, Widget, WidgetRef,
    },
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

    pub fn update(&mut self, other: WidgetRef) -> bool {
        self.dispatch.update(other)
    }

    pub fn is_similar(&self, other: &WidgetRef) -> WidgetEquality {
        self.dispatch.is_similar(other)
    }

    pub fn downcast_ref<W>(&self) -> Option<&W>
    where
        W: WidgetDispatch,
    {
        self.dispatch.downcast_ref()
    }

    pub(crate) fn layout(&mut self, ctx: AguiContext) {
        let layout = self.dispatch.layout(ctx);

        self.layout_type = layout.layout_type;
        self.layout = layout.layout;
    }

    pub(crate) fn build(&mut self, ctx: AguiContext) -> BuildResult {
        self.dispatch.build(ctx)
    }

    /// Causes the widget to draw to a canvas.
    pub fn paint(&self) -> Option<Canvas> {
        self.rect
            .and_then(|rect| self.dispatch.paint(Size::from(rect)))
    }

    #[allow(clippy::borrowed_box)]
    pub(crate) fn call(
        &mut self,
        ctx: AguiContext,
        callback_id: CallbackId,
        arg: &Box<dyn Data>,
    ) -> bool {
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
    W: Widget,
{
    fn from(widget: W) -> Self {
        WidgetElement::new(widget.into()).unwrap()
    }
}
