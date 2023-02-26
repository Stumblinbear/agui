use slotmap::new_key_type;

use crate::{
    callback::CallbackId,
    inheritance::Inheritance,
    manager::context::AguiContext,
    render::canvas::Canvas,
    unit::{Data, Layout, LayoutType, Rect, Size},
    widget::{instance::ElementWidget, key::WidgetKey, Children, WidgetRef},
};

use self::context::ElementContext;

pub mod context;

new_key_type! {
    pub struct ElementId;
}

pub struct Element {
    key: Option<WidgetKey>,

    layout_type: LayoutType,
    layout: Layout,

    rect: Option<Rect>,

    widget: Box<dyn ElementWidget>,

    inheritance: Inheritance,
}

impl Element {
    pub(crate) fn new(key: Option<WidgetKey>, widget: Box<dyn ElementWidget>) -> Self {
        Self {
            key,

            layout_type: LayoutType::default(),
            layout: Layout::default(),

            rect: None,

            widget,

            inheritance: Inheritance::default(),
        }
    }

    pub fn type_name(&self) -> &'static str {
        self.widget.type_name()
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

    pub fn is_similar(&self, other: &WidgetRef) -> bool {
        self.widget.is_similar(other)
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

        self.widget.update(other)
    }

    pub fn layout(&mut self, ctx: AguiContext) {
        let span = tracing::error_span!("layout");
        let _enter = span.enter();

        let layout = self.widget.layout(ElementContext {
            element_tree: ctx.element_tree,
            dirty: ctx.dirty,
            callback_queue: ctx.callback_queue,

            element_id: ctx.element_id,

            inheritance: &mut self.inheritance,
        });

        self.layout_type = layout.layout_type;
        self.layout = layout.layout;
    }

    pub fn build(&mut self, ctx: AguiContext) -> Children {
        let span = tracing::error_span!("build");
        let _enter = span.enter();

        self.widget.build(ElementContext {
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
            .and_then(|rect| self.widget.paint(Size::from(rect)))
    }

    #[allow(clippy::borrowed_box)]
    pub fn call(&mut self, ctx: AguiContext, callback_id: CallbackId, arg: &Box<dyn Data>) -> bool {
        let span = tracing::error_span!("callback");
        let _enter = span.enter();

        self.widget.call(
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
        self.widget.fmt(f)
    }
}
