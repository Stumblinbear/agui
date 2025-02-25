use std::{
    any::{Any, TypeId},
    mem::ManuallyDrop,
};

use crate::{
    context::{MessageCtx, UpdateCtx},
    element::{Element, ElementState},
    render_object::{RenderLeaf, RenderObject},
};

mod any_view;

pub use any_view::*;

pub trait View {
    type Render: RenderObject;

    type State: Any
    where
        Self: Sized;

    fn is_same_type(&self, other: &Self) -> bool {
        let _ = other;
        true
    }

    fn mount(&self, ctx: &mut UpdateCtx) -> (Vec<Element>, Self::State)
    where
        Self: Sized;

    /// Called when the tree is updated and the `state` in the [`Element`] is of the same type as `Self::State`.
    fn update(&self, element: &mut Element, old: &Self, ctx: &mut UpdateCtx);

    fn message(&self, element: &mut Element, ctx: MessageCtx);

    fn create_render_object(&self, element: &Element) -> Self::Render;

    fn update_render_object(&self, element: &Element, render_object: &mut Self::Render);
}

#[diagnostic::on_unimplemented(
    message = "Trait bound View is not satisfied.",
    note = "dyn View is not supported, use dyn AnyView via .as_dyn_view() or .into_boxed_view() instead."
)]
pub trait MountView {
    fn mount(&self, ctx: &mut UpdateCtx) -> (Vec<Element>, ElementState);
}

impl<T> MountView for T
where
    T: View,
{
    fn mount(&self, ctx: &mut UpdateCtx) -> (Vec<Element>, ElementState) {
        let (children, state) = <T as View>::mount(self, ctx);

        if TypeId::of::<T::State>() == TypeId::of::<ElementState>()
            && size_of::<T::State>() == size_of::<ElementState>()
        {
            // Since this is an owned value, we need to mark it as a manually dropped value so that
            // it doesn't get immediately dropped when we return it after transmuting it.
            let state = ManuallyDrop::new(state);

            // SAFETY: This is probably safe so long as there are no TypeId + size collisions
            let state = unsafe { std::mem::transmute_copy::<T::State, ElementState>(&state) };

            return (children, state);
        }

        (children, ElementState::new(state))
    }
}

impl View for () {
    type Render = RenderLeaf;

    type State = ();

    fn mount(&self, _: &mut UpdateCtx) -> (Vec<Element>, Self::State) {
        (Vec::new(), ())
    }

    fn update(&self, _: &mut Element, _: &Self, _: &mut UpdateCtx) {}

    fn message(&self, _: &mut Element, _: MessageCtx) {}

    fn create_render_object(&self, _: &Element) -> Self::Render {
        RenderLeaf::default()
    }

    fn update_render_object(&self, _: &Element, _: &mut Self::Render) {}
}
