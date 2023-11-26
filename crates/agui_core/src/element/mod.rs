use std::any::Any;

use crate::{
    callback::CallbackId,
    render::{RenderBox, RenderObject, RenderObjectId},
    widget::Widget,
};

use self::{
    build::ElementBuild, proxy::ElementProxy, render::ElementRender, widget::ElementWidget,
};

pub mod build;
mod builder;
mod context;
#[cfg(any(test, feature = "mocks"))]
pub mod mock;
pub mod proxy;
pub mod render;
mod update;
pub mod widget;

pub use builder::*;
pub use context::*;
pub use update::*;

slotmap::new_key_type! {
    pub struct ElementId;
}

pub struct Element {
    inner: ElementType,

    widget: Widget,

    render_object_id: Option<RenderObjectId>,
}

pub enum ElementType {
    Proxy(Box<dyn ElementProxy>),

    Widget(Box<dyn ElementBuild>),
    Render(Box<dyn ElementRender>),
}

impl Element {
    pub(crate) fn new(widget: Widget) -> Self {
        Self {
            inner: Widget::create_element(&widget),

            widget,

            render_object_id: None,
        }
    }

    pub fn element_name(&self) -> &'static str {
        match self.inner {
            ElementType::Proxy(ref element) => (**element).short_type_name(),

            ElementType::Widget(ref element) => (**element).short_type_name(),
            ElementType::Render(ref element) => (**element).short_type_name(),
        }
    }

    pub fn widget_name(&self) -> &str {
        self.widget.widget_name()
    }

    pub fn widget(&self) -> &Widget {
        &self.widget
    }

    pub fn render_object_id(&self) -> Option<RenderObjectId> {
        self.render_object_id
    }

    pub fn is<E>(&self) -> bool
    where
        E: ElementWidget,
    {
        match self.inner {
            ElementType::Proxy(ref widget) => (**widget).as_any().is::<E>(),

            ElementType::Widget(ref widget) => (**widget).as_any().is::<E>(),
            ElementType::Render(ref widget) => (**widget).as_any().is::<E>(),
        }
    }

    pub fn downcast<E>(&self) -> Option<&E>
    where
        E: ElementWidget,
    {
        match self.inner {
            ElementType::Proxy(ref widget) => (**widget).as_any().downcast_ref::<E>(),

            ElementType::Widget(ref widget) => (**widget).as_any().downcast_ref::<E>(),
            ElementType::Render(ref widget) => (**widget).as_any().downcast_ref::<E>(),
        }
    }

    pub fn downcast_mut<E>(&mut self) -> Option<&mut E>
    where
        E: ElementWidget,
    {
        match self.inner {
            ElementType::Proxy(ref mut widget) => (**widget).as_any_mut().downcast_mut::<E>(),

            ElementType::Widget(ref mut widget) => (**widget).as_any_mut().downcast_mut::<E>(),
            ElementType::Render(ref mut widget) => (**widget).as_any_mut().downcast_mut::<E>(),
        }
    }

    #[tracing::instrument(level = "debug", skip(self, ctx), fields(widget_name = self.widget_name()))]
    pub fn mount(&mut self, ctx: &mut ElementMountContext) {
        match self.inner {
            ElementType::Proxy(ref mut widget) => widget.mount(ctx),

            ElementType::Widget(ref mut widget) => widget.mount(ctx),
            ElementType::Render(ref mut widget) => widget.mount(ctx),
        }
    }

    #[tracing::instrument(level = "debug", skip(self, ctx), fields(widget_name = self.widget_name()))]
    pub fn unmount(&mut self, ctx: &mut ElementUnmountContext) {
        match self.inner {
            ElementType::Proxy(ref mut widget) => widget.unmount(ctx),

            ElementType::Widget(ref mut widget) => widget.unmount(ctx),
            ElementType::Render(ref mut widget) => widget.unmount(ctx),
        }
    }

    #[tracing::instrument(level = "debug", skip(self, ctx), fields(widget_name = self.widget_name()))]
    pub fn build(&mut self, ctx: &mut ElementBuildContext) -> Vec<Widget> {
        match self.inner {
            ElementType::Proxy(ref mut widget) => Vec::from([widget.child()]),

            ElementType::Widget(ref mut widget) => Vec::from([widget.build(ctx)]),
            ElementType::Render(ref mut widget) => widget.children(),
        }
    }

    #[tracing::instrument(level = "debug", skip(self, new_widget), fields(widget_name = self.widget_name()))]
    pub fn update(&mut self, new_widget: &Widget) -> ElementUpdate {
        if &self.widget == new_widget {
            return ElementUpdate::Noop;
        }

        let result = match self.inner {
            ElementType::Proxy(ref mut widget) => widget.update(new_widget),

            ElementType::Widget(ref mut widget) => widget.update(new_widget),
            ElementType::Render(ref mut widget) => widget.update(new_widget),
        };

        match result {
            ElementUpdate::Noop | ElementUpdate::RebuildNecessary => {
                self.widget = new_widget.clone();
            }

            ElementUpdate::Invalid => {}
        }

        result
    }

    #[tracing::instrument(level = "debug", skip(self, ctx), fields(widget_name = self.widget_name()))]
    pub fn call(
        &mut self,
        ctx: &mut ElementCallbackContext,
        callback_id: CallbackId,
        arg: Box<dyn Any>,
    ) -> bool {
        match self.inner {
            ElementType::Proxy(_) | ElementType::Render(_) => {
                tracing::warn!("attempted to call a callback on an unsupported element");

                false
            }

            ElementType::Widget(ref mut widget) => widget.call(ctx, callback_id, arg),
        }
    }

    pub(crate) fn set_render_object_id(&mut self, id: RenderObjectId) {
        self.render_object_id = Some(id);
    }

    pub fn create_render_object(&mut self, ctx: &mut ElementBuildContext) -> RenderObject {
        match self.inner {
            // Use the default render object for proxies and widgets
            ElementType::Proxy(_) | ElementType::Widget(_) => {
                RenderObject::new(RenderBox::default())
            }

            ElementType::Render(ref mut widget) => {
                widget.create_render_object(&mut RenderObjectBuildContext { inner: ctx })
            }
        }
    }

    #[tracing::instrument(level = "debug", skip(self, ctx), fields(widget_name = self.widget_name()))]
    pub(crate) fn update_render_object(
        &mut self,
        ctx: &mut ElementBuildContext,
        render_object: &mut RenderObject,
    ) {
        if let ElementType::Render(widget) = &mut self.inner {
            widget.update_render_object(
                &mut RenderObjectUpdateContext {
                    inner: ctx,

                    render_object_id: self
                        .render_object_id
                        .as_ref()
                        .expect("called update_render_object on an element without one attached"),
                },
                render_object,
            );
        } else {
            tracing::trace!("skipping render object update for non-render element");
        }
    }
}

impl std::fmt::Debug for Element {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.inner {
            ElementType::Proxy(ref widget) => widget.fmt(f),

            ElementType::Widget(ref widget) => widget.fmt(f),
            ElementType::Render(ref widget) => widget.fmt(f),
        }
    }
}
