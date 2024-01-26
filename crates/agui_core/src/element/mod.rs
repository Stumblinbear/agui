use std::any::Any;

use crate::{
    callback::CallbackId,
    element::{deferred::ElementDeferred, inherited::ElementInherited, view::ElementView},
    render::object::{RenderBox, RenderObject},
    widget::Widget,
};

use self::{build::ElementBuild, lifecycle::ElementLifecycle, render::ElementRender};

pub mod build;
mod builder;
mod comparison;
mod context;
pub mod deferred;
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

#[cfg(not(miri))]
/// The amount of space to allocate on the stack for an element.
/// This is used to avoid indirection for small elements, which
/// is a very common case.
type ElementSpace = smallbox::space::S8;

#[cfg(not(miri))]
type ElementBox<T> = smallbox::SmallBox<T, ElementSpace>;

#[cfg(miri)]
type ElementBox<T> = Box<T>;

pub enum Element {
    Widget(ElementBox<dyn ElementBuild>),
    Inherited(ElementBox<dyn ElementInherited>),

    View(ElementBox<dyn ElementView>),
    Render(ElementBox<dyn ElementRender>),

    Deferred(ElementBox<dyn ElementDeferred>),
}

impl Element {
    pub fn new_widget<E>(element: E) -> Self
    where
        E: ElementBuild,
    {
        #[cfg(not(miri))]
        {
            Element::Widget(smallbox::smallbox!(element))
        }

        #[cfg(miri)]
        {
            Element::Widget(Box::new(element))
        }
    }

    pub fn new_inherited<E>(element: E) -> Self
    where
        E: ElementInherited,
    {
        #[cfg(not(miri))]
        {
            Element::Inherited(smallbox::smallbox!(element))
        }

        #[cfg(miri)]
        {
            Element::Inherited(Box::new(element))
        }
    }

    pub fn new_view<E>(element: E) -> Self
    where
        E: ElementView,
    {
        #[cfg(not(miri))]
        {
            Element::View(smallbox::smallbox!(element))
        }

        #[cfg(miri)]
        {
            Element::View(Box::new(element))
        }
    }

    pub fn new_render<E>(element: E) -> Self
    where
        E: ElementRender,
    {
        #[cfg(not(miri))]
        {
            Element::Render(smallbox::smallbox!(element))
        }

        #[cfg(miri)]
        {
            Element::Render(Box::new(element))
        }
    }

    pub fn new_deferred<E>(element: E) -> Self
    where
        E: ElementDeferred,
    {
        #[cfg(not(miri))]
        {
            Element::Deferred(smallbox::smallbox!(element))
        }

        #[cfg(miri)]
        {
            Element::Deferred(Box::new(element))
        }
    }
}

impl Element {
    pub fn is<E>(&self) -> bool
    where
        E: ElementLifecycle,
    {
        match self {
            Element::Widget(ref element) => (**element).as_any().is::<E>(),
            Element::Inherited(ref element) => (**element).as_any().is::<E>(),

            Element::View(ref element) => (**element).as_any().is::<E>(),
            Element::Render(ref element) => (**element).as_any().is::<E>(),

            Element::Deferred(ref element) => (**element).as_any().is::<E>(),
        }
    }

    pub fn downcast<E>(&self) -> Option<&E>
    where
        E: ElementLifecycle,
    {
        match self {
            Element::Widget(ref element) => (**element).as_any().downcast_ref::<E>(),
            Element::Inherited(ref element) => (**element).as_any().downcast_ref::<E>(),

            Element::View(ref element) => (**element).as_any().downcast_ref::<E>(),
            Element::Render(ref element) => (**element).as_any().downcast_ref::<E>(),

            Element::Deferred(ref element) => (**element).as_any().downcast_ref::<E>(),
        }
    }

    pub fn downcast_mut<E>(&mut self) -> Option<&mut E>
    where
        E: ElementLifecycle,
    {
        match self {
            Element::Widget(ref mut element) => (**element).as_any_mut().downcast_mut::<E>(),
            Element::Inherited(ref mut element) => (**element).as_any_mut().downcast_mut::<E>(),

            Element::View(ref mut element) => (**element).as_any_mut().downcast_mut::<E>(),
            Element::Render(ref mut element) => (**element).as_any_mut().downcast_mut::<E>(),

            Element::Deferred(ref mut element) => (**element).as_any_mut().downcast_mut::<E>(),
        }
    }

    #[tracing::instrument(level = "trace", skip_all)]
    pub fn mount(&mut self, ctx: &mut ElementMountContext) {
        match self {
            Element::Widget(ref mut element) => element.mount(ctx),
            Element::Inherited(ref mut element) => {
                ctx.inheritance.create_scope(
                    element.inherited_type_id(),
                    *ctx.parent_element_id,
                    *ctx.element_id,
                );

                element.mount(ctx);
            }

            Element::View(ref mut element) => element.mount(ctx),
            Element::Render(ref mut element) => element.mount(ctx),

            Element::Deferred(ref mut element) => element.mount(ctx),
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

    #[tracing::instrument(level = "trace", skip_all)]
    pub fn unmount(&mut self, ctx: &mut ElementUnmountContext) {
        match self {
            Element::Widget(ref mut element) => element.unmount(ctx),
            Element::Inherited(ref mut element) => element.unmount(ctx),

            Element::View(ref mut element) => element.unmount(ctx),
            Element::Render(ref mut element) => element.unmount(ctx),

            Element::Deferred(ref mut element) => element.unmount(ctx),
        }

        ctx.inheritance.remove(*ctx.element_id);
    }

    #[tracing::instrument(level = "trace", skip_all)]
    pub fn build(&mut self, ctx: &mut ElementBuildContext) -> Vec<Widget> {
        match self {
            Element::Widget(ref mut element) => Vec::from([element.build(ctx)]),
            Element::Inherited(ref mut element) => {
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

            Element::View(ref mut element) => element.children(),
            Element::Render(ref mut element) => element.children(),

            Element::Deferred(_) => Vec::new(),
        }
    }

    #[tracing::instrument(level = "trace", skip_all)]
    pub fn update(&mut self, new_widget: &Widget) -> ElementComparison {
        match self {
            Element::Widget(ref mut element) => element.update(new_widget),
            Element::Inherited(ref mut element) => element.update(new_widget),

            Element::View(ref mut element) => element.update(new_widget),
            Element::Render(ref mut element) => element.update(new_widget),

            Element::Deferred(ref mut element) => element.update(new_widget),
        }
    }

    #[tracing::instrument(level = "trace", skip_all)]
    pub fn call(
        &mut self,
        ctx: &mut ElementCallbackContext,
        callback_id: CallbackId,
        arg: Box<dyn Any>,
    ) -> bool {
        match self {
            Element::Inherited(_)
            | Element::View(_)
            | Element::Render(_)
            | Element::Deferred(_) => {
                tracing::warn!("attempted to call a callback on an unsupported element");

                false
            }

            Element::Widget(ref mut element) => element.call(ctx, callback_id, arg),
        }
    }

    pub fn create_render_object(&self, ctx: &mut RenderObjectCreateContext) -> RenderObject {
        match self {
            // Use the default render object
            Element::Widget(_) | Element::Inherited(_) | Element::Deferred(_) => {
                RenderObject::new(RenderBox::default())
            }

            Element::View(ref element) => element.create_render_object(ctx),
            Element::Render(ref element) => element.create_render_object(ctx),
        }
    }

    #[tracing::instrument(level = "trace", skip_all)]
    pub(crate) fn update_render_object(
        &self,
        ctx: &mut RenderObjectUpdateContext,
        render_object: &mut RenderObject,
    ) {
        if let Element::Render(element) = &self {
            element.update_render_object(ctx, render_object);
        } else {
            tracing::trace!("skipping render object update for non-render element");
        }
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

        f.debug_struct(match self {
            Element::Widget(_) => "Element::Widget",
            Element::Inherited(_) => "Element::Inherited",

            Element::View(_) => "Element::View",
            Element::Render(_) => "Element::Render",

            Element::Deferred(_) => "Element::Deferred",
        })
        .finish()
    }
}
