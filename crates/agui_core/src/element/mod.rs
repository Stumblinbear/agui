use std::any::Any;

use crate::{
    callback::CallbackId,
    element::{inherited::ElementInherited, view::ElementView},
    render::object::{RenderBox, RenderObject},
    unit::Key,
    widget::Widget,
};

use self::{build::ElementBuild, lifecycle::ElementLifecycle, render::ElementRender};

pub mod build;
mod builder;
mod comparison;
mod context;
pub mod inherited;
pub mod lifecycle;
#[cfg(any(test, feature = "mocks"))]
pub mod mock;
pub mod render;
pub mod view;
pub mod widget;

pub use builder::*;
pub use comparison::*;
pub use context::*;

slotmap::new_key_type! {
    pub struct ElementId;
}

pub struct Element {
    meta: Box<ElementMetadata>,
    inner: ElementType,
}

struct ElementMetadata {
    name: &'static str,
    key: Option<Key>,
}

#[cfg(not(miri))]
/// The amount of space to allocate on the stack for an element.
/// This is used to avoid indirection for small elements, which
/// is a very common case.
type ElementSpace = smallbox::space::S8;

#[cfg(not(miri))]
type ElementBox<T> = smallbox::SmallBox<T, ElementSpace>;

#[cfg(miri)]
type ElementBox<T> = Box<T>;

pub enum ElementType {
    Widget(ElementBox<dyn ElementBuild>),

    Inherited(ElementBox<dyn ElementInherited>),

    View(ElementBox<dyn ElementView>),
    Render(ElementBox<dyn ElementRender>),
}

impl ElementType {
    pub fn new_widget<E>(element: E) -> Self
    where
        E: ElementBuild,
    {
        #[cfg(not(miri))]
        {
            ElementType::Widget(smallbox::smallbox!(element))
        }

        #[cfg(miri)]
        {
            ElementType::Widget(Box::new(element))
        }
    }

    pub fn new_inherited<E>(element: E) -> Self
    where
        E: ElementInherited,
    {
        #[cfg(not(miri))]
        {
            ElementType::Inherited(smallbox::smallbox!(element))
        }

        #[cfg(miri)]
        {
            ElementType::Inherited(Box::new(element))
        }
    }

    pub fn new_view<E>(element: E) -> Self
    where
        E: ElementView,
    {
        #[cfg(not(miri))]
        {
            ElementType::View(smallbox::smallbox!(element))
        }

        #[cfg(miri)]
        {
            ElementType::View(Box::new(element))
        }
    }

    pub fn new_render<E>(element: E) -> Self
    where
        E: ElementRender,
    {
        #[cfg(not(miri))]
        {
            ElementType::Render(smallbox::smallbox!(element))
        }

        #[cfg(miri)]
        {
            ElementType::Render(Box::new(element))
        }
    }
}

impl Element {
    pub fn new(name: &'static str, key: Option<Key>, element: ElementType) -> Self {
        Self {
            meta: Box::new(ElementMetadata { name, key }),
            inner: element,
        }
    }

    pub fn is<E>(&self) -> bool
    where
        E: ElementLifecycle,
    {
        match self.inner {
            ElementType::Widget(ref element) => (**element).as_any().is::<E>(),

            ElementType::Inherited(ref element) => (**element).as_any().is::<E>(),

            ElementType::View(ref element) => (**element).as_any().is::<E>(),
            ElementType::Render(ref element) => (**element).as_any().is::<E>(),
        }
    }

    pub fn downcast<E>(&self) -> Option<&E>
    where
        E: ElementLifecycle,
    {
        match self.inner {
            ElementType::Widget(ref element) => (**element).as_any().downcast_ref::<E>(),

            ElementType::Inherited(ref element) => (**element).as_any().downcast_ref::<E>(),

            ElementType::View(ref element) => (**element).as_any().downcast_ref::<E>(),
            ElementType::Render(ref element) => (**element).as_any().downcast_ref::<E>(),
        }
    }

    pub fn downcast_mut<E>(&mut self) -> Option<&mut E>
    where
        E: ElementLifecycle,
    {
        match self.inner {
            ElementType::Widget(ref mut element) => (**element).as_any_mut().downcast_mut::<E>(),

            ElementType::Inherited(ref mut element) => (**element).as_any_mut().downcast_mut::<E>(),

            ElementType::View(ref mut element) => (**element).as_any_mut().downcast_mut::<E>(),
            ElementType::Render(ref mut element) => (**element).as_any_mut().downcast_mut::<E>(),
        }
    }

    pub fn name(&self) -> &'static str {
        self.meta.name
    }

    pub fn key(&self) -> Option<Key> {
        self.meta.key
    }

    #[tracing::instrument(level = "trace", skip(self, ctx), fields(widget_name = self.meta.name))]
    pub fn mount(&mut self, ctx: &mut ElementMountContext) {
        match self.inner {
            ElementType::Widget(ref mut element) => element.mount(ctx),

            ElementType::Inherited(ref mut element) => {
                ctx.inheritance.create_scope(
                    element.inherited_type_id(),
                    *ctx.parent_element_id,
                    *ctx.element_id,
                );

                element.mount(ctx);
            }

            ElementType::View(ref mut element) => element.mount(ctx),
            ElementType::Render(ref mut element) => element.mount(ctx),
        }

        // Is it beneficial to delay building up these until an element actually needs them?
        ctx.inheritance
            .create_node(*ctx.parent_element_id, *ctx.element_id);
    }

    // pub fn remount() {
    //     let parent_scope_id = ctx.parent_element_id().and_then(|parent_element_id| {
    //         self.manager
    //             .get(parent_element_id)
    //             .expect("failed to get scope from parent")
    //             .scope()
    //     });

    //     let element_id = ctx.element_id();

    //     self.manager
    //         .update_inheritance_scope(ctx, element_id, parent_scope_id);
    // }

    #[tracing::instrument(level = "trace", skip(self, ctx), fields(widget_name = self.meta.name))]
    pub fn unmount(&mut self, ctx: &mut ElementUnmountContext) {
        match self.inner {
            ElementType::Widget(ref mut element) => element.unmount(ctx),

            ElementType::Inherited(ref mut element) => element.unmount(ctx),

            ElementType::View(ref mut element) => element.unmount(ctx),
            ElementType::Render(ref mut element) => element.unmount(ctx),
        }

        ctx.inheritance.remove(*ctx.element_id);
    }

    #[tracing::instrument(level = "trace", skip(self, ctx), fields(widget_name = self.meta.name))]
    pub fn build(&mut self, ctx: &mut ElementBuildContext) -> Vec<Widget> {
        match self.inner {
            ElementType::Widget(ref mut element) => Vec::from([element.build(ctx)]),

            ElementType::Inherited(ref mut element) => {
                if element.needs_notify() {
                    for element_id in ctx
                        .inheritance
                        .iter_listeners(*ctx.element_id)
                        .expect("failed to get the inherited element's scope during build")
                    {
                        ctx.needs_build.insert(element_id);
                    }
                }

                Vec::from([element.child()])
            }

            ElementType::View(ref mut element) => element.children(),
            ElementType::Render(ref mut element) => element.children(),
        }
    }

    #[tracing::instrument(level = "trace", skip(self, new_widget), fields(widget_name = self.meta.name))]
    pub fn update(&mut self, new_widget: &Widget) -> ElementComparison {
        match self.inner {
            ElementType::Widget(ref mut element) => element.update(new_widget),

            ElementType::Inherited(ref mut element) => element.update(new_widget),

            ElementType::View(ref mut element) => element.update(new_widget),
            ElementType::Render(ref mut element) => element.update(new_widget),
        }
    }

    #[tracing::instrument(level = "trace", skip(self, ctx), fields(widget_name = self.meta.name))]
    pub fn call(
        &mut self,
        ctx: &mut ElementCallbackContext,
        callback_id: CallbackId,
        arg: Box<dyn Any>,
    ) -> bool {
        match self.inner {
            ElementType::Inherited(_) | ElementType::View(_) | ElementType::Render(_) => {
                tracing::warn!("attempted to call a callback on an unsupported element");

                false
            }

            ElementType::Widget(ref mut element) => element.call(ctx, callback_id, arg),
        }
    }

    pub fn create_render_object(&self, ctx: &mut RenderObjectCreateContext) -> RenderObject {
        match self.inner {
            // Use the default render object
            ElementType::Widget(_) | ElementType::Inherited(_) => {
                RenderObject::new(RenderBox::default())
            }

            ElementType::View(ref element) => element.create_render_object(ctx),
            ElementType::Render(ref element) => element.create_render_object(ctx),
        }
    }

    #[tracing::instrument(level = "trace", skip(self, ctx), fields(widget_name = self.meta.name))]
    pub(crate) fn update_render_object(
        &self,
        ctx: &mut RenderObjectUpdateContext,
        render_object: &mut RenderObject,
    ) {
        if let ElementType::Render(element) = &self.inner {
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
            ElementType::Widget(_) => "Element::Widget",
            ElementType::Inherited(_) => "Element::Inherited",
            ElementType::View(_) => "Element::View",
            ElementType::Render(_) => "Element::Render",
        })
        .field("key", &self.meta.key)
        .field("widget", &DebugWidget(self.meta.name))
        .finish()
    }
}
