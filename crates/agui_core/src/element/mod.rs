use std::any::Any;

use crate::{
    callback::CallbackId,
    element::view::ElementView,
    render::{RenderBox, RenderObject, RenderObjectId, RenderView},
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
pub mod view;
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

    View(Box<dyn ElementView>),
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
            ElementType::Proxy(ref element) => (**element).as_any().is::<E>(),

            ElementType::Widget(ref element) => (**element).as_any().is::<E>(),

            ElementType::View(ref element) => (**element).as_any().is::<E>(),
            ElementType::Render(ref element) => (**element).as_any().is::<E>(),
        }
    }

    pub fn downcast<E>(&self) -> Option<&E>
    where
        E: ElementWidget,
    {
        match self.inner {
            ElementType::Proxy(ref element) => (**element).as_any().downcast_ref::<E>(),

            ElementType::Widget(ref element) => (**element).as_any().downcast_ref::<E>(),

            ElementType::View(ref element) => (**element).as_any().downcast_ref::<E>(),
            ElementType::Render(ref element) => (**element).as_any().downcast_ref::<E>(),
        }
    }

    pub fn downcast_mut<E>(&mut self) -> Option<&mut E>
    where
        E: ElementWidget,
    {
        match self.inner {
            ElementType::Proxy(ref mut element) => (**element).as_any_mut().downcast_mut::<E>(),

            ElementType::Widget(ref mut element) => (**element).as_any_mut().downcast_mut::<E>(),

            ElementType::View(ref mut element) => (**element).as_any_mut().downcast_mut::<E>(),
            ElementType::Render(ref mut element) => (**element).as_any_mut().downcast_mut::<E>(),
        }
    }

    #[tracing::instrument(level = "debug", skip(self, ctx), fields(widget_name = self.widget_name()))]
    pub fn mount(&mut self, ctx: &mut ElementMountContext) {
        match self.inner {
            ElementType::Proxy(ref mut element) => element.mount(ctx),

            ElementType::Widget(ref mut element) => element.mount(ctx),

            ElementType::View(ref mut element) => element.mount(ctx),
            ElementType::Render(ref mut element) => element.mount(ctx),
        }
    }

    #[tracing::instrument(level = "debug", skip(self, ctx), fields(widget_name = self.widget_name()))]
    pub fn unmount(&mut self, ctx: &mut ElementUnmountContext) {
        match self.inner {
            ElementType::Proxy(ref mut element) => element.unmount(ctx),

            ElementType::Widget(ref mut element) => element.unmount(ctx),

            ElementType::View(ref mut element) => element.unmount(ctx),
            ElementType::Render(ref mut element) => element.unmount(ctx),
        }
    }

    #[tracing::instrument(level = "debug", skip(self, ctx), fields(widget_name = self.widget_name()))]
    pub fn build(&mut self, ctx: &mut ElementBuildContext) -> Vec<Widget> {
        match self.inner {
            ElementType::Proxy(ref mut element) => Vec::from([element.child()]),

            ElementType::Widget(ref mut element) => Vec::from([element.build(ctx)]),

            ElementType::View(ref mut element) => Vec::from([element.child()]),
            ElementType::Render(ref mut element) => element.children(),
        }
    }

    #[tracing::instrument(level = "debug", skip(self, new_widget), fields(widget_name = self.widget_name()))]
    pub fn update(&mut self, new_widget: &Widget) -> ElementUpdate {
        if &self.widget == new_widget {
            return ElementUpdate::Noop;
        }

        let result = match self.inner {
            ElementType::Proxy(ref mut element) => element.update(new_widget),

            ElementType::Widget(ref mut element) => element.update(new_widget),

            ElementType::View(ref mut element) => element.update(new_widget),
            ElementType::Render(ref mut element) => element.update(new_widget),
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
            ElementType::Proxy(_) | ElementType::View(_) | ElementType::Render(_) => {
                tracing::warn!("attempted to call a callback on an unsupported element");

                false
            }

            ElementType::Widget(ref mut element) => element.call(ctx, callback_id, arg),
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

            ElementType::View(_) => RenderObject::new(RenderView::default()),

            ElementType::Render(ref mut element) => {
                element.create_render_object(&mut RenderObjectBuildContext { inner: ctx })
            }
        }
    }

    #[tracing::instrument(level = "debug", skip(self, ctx), fields(widget_name = self.widget_name()))]
    pub(crate) fn update_render_object(
        &mut self,
        ctx: &mut RenderObjectUpdateContext,
        render_object: &mut RenderObject,
    ) {
        if let ElementType::Render(element) = &mut self.inner {
            element.update_render_object(ctx, render_object);
        } else {
            tracing::trace!("skipping render object update for non-render element");
        }
    }
}

impl AsRef<ElementType> for Element {
    fn as_ref(&self) -> &ElementType {
        &self.inner
    }
}

impl AsMut<ElementType> for Element {
    fn as_mut(&mut self) -> &mut ElementType {
        &mut self.inner
    }
}

impl std::fmt::Debug for Element {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        struct DebugWidget(&'static str);

        impl std::fmt::Debug for DebugWidget {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct(self.0).finish_non_exhaustive()
            }
        }

        f.debug_struct(match self.inner {
            ElementType::Proxy(_) => "ProxyElement",
            ElementType::Widget(_) => "WidgetElement",
            ElementType::View(_) => "ViewElement",
            ElementType::Render(_) => "RenderElement",
        })
        .field("widget", &DebugWidget(self.widget.widget_name()))
        .finish()
    }
}
