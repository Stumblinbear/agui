use std::{
    any::{Any, TypeId},
    mem::ManuallyDrop,
};

use typed_floats::{as_const, Positive, PositiveFinite};

use crate::{
    constraints::Constraints,
    context::{MessageCtx, UpdateCtx},
    element::{Element, ElementState},
    hit_test::HitTestResult,
    offset::Offset,
    renderer::Canvas,
    size::Size,
    text_baseline::TextBaseline,
};

mod any_view;
mod layout_marker;

pub use any_view::*;
pub use layout_marker::*;

pub trait View: ViewLayoutMarker {
    type State: Any
    where
        Self: Sized;

    fn is_similar(&self, other_state: &dyn Any) -> bool
    where
        Self: Sized,
    {
        TypeId::of::<Self::State>() == other_state.type_id()
    }

    fn mount(&self, ctx: &mut UpdateCtx) -> (Vec<Element>, Self::State)
    where
        Self: Sized;

    fn update(&self, element: &mut Element, old: &Self, ctx: &mut UpdateCtx);

    fn message(&self, element: &mut Element, ctx: MessageCtx);

    /// Returns the minimum width that this box could be without failing to
    /// correctly paint its contents within itself, without clipping.
    ///
    /// The height argument may give a specific height to assume. The given height
    /// can be infinite, meaning that the intrinsic width in an unconstrained
    /// environment is being requested.
    ///
    /// In cases where computing the minimum intrinsic width is not possible, or is
    /// prohibitively expensive, this function should return [`None`]. Callers should
    /// be prepared to handle this case by providing a fallback width (typically
    /// zero).
    ///
    /// Calling this function is expensive as it can result in O(N^2) behavior.
    fn min_intrinsic_width(
        &self,
        element: &Element,
        height: Positive<f32>,
    ) -> Option<PositiveFinite<f32>>;

    /// Returns the smallest width beyond which increasing the width never
    /// decreases the preferred height. The preferred height is the value that
    /// would be returned by [`View::min_intrinsic_height`] for that width.
    ///
    /// The height argument may give a specific height to assume. The given height
    /// can be infinite, meaning that the intrinsic width in an unconstrained
    /// environment is being requested.
    ///
    /// In cases where computing the minimum intrinsic width is not possible, or is
    /// prohibitively expensive, this function should return [`None`]. Callers should
    /// be prepared to handle this case by providing a fallback width (typically
    /// zero).
    ///
    /// Calling this function is expensive as it can result in O(N^2) behavior.
    fn max_intrinsic_width(
        &self,
        element: &Element,
        height: Positive<f32>,
    ) -> Option<PositiveFinite<f32>>;

    /// Returns the minimum height that this box could be without failing to
    /// correctly paint its contents within itself, without clipping.
    ///
    /// The width argument may give a specific width to assume. The given width
    /// can be infinite, meaning that the intrinsic height in an unconstrained
    /// environment is being requested.
    ///
    /// In cases where computing the minimum intrinsic height is not possible, or is
    /// prohibitively expensive, this function should return [`None`]. Callers should
    /// be prepared to handle this case by providing a fallback height (typically
    /// zero).
    ///
    /// Calling this function is expensive as it can result in O(N^2) behavior.
    fn min_intrinsic_height(
        &self,
        element: &Element,
        width: Positive<f32>,
    ) -> Option<PositiveFinite<f32>>;

    /// Returns the smallest height beyond which increasing the height never
    /// decreases the preferred width. The preferred width is the value that
    /// would be returned by [`View::min_intrinsic_width`] for that height.
    ///
    /// The width argument may give a specific width to assume. The given width
    /// can be infinite, meaning that the intrinsic height in an unconstrained
    /// environment is being requested.
    ///
    /// In cases where computing the minimum intrinsic height is not possible, or is
    /// prohibitively expensive, this function should return [`None`]. Callers should
    /// be prepared to handle this case by providing a fallback height (typically
    /// zero).
    ///
    /// Calling this function is expensive as it can result in O(N^2) behavior.
    fn max_intrinsic_height(
        &self,
        element: &Element,
        width: Positive<f32>,
    ) -> Option<PositiveFinite<f32>>;

    /// Returns the [`Size`] that this [`View`] would like to be given the
    /// provided [`Constraints`].
    ///
    /// The size returned by this method is guaranteed to be the same size that
    /// this [`View`] computes for itself during layout given the same
    /// constraints.
    ///
    /// This function should only be called on one's children. Calling this
    /// function couples the child with the parent so that when the child's layout
    /// changes, the parent is also laid out.
    ///
    /// Calling this function is expensive as it can result in O(N^2) behavior.
    fn measure(&self, element: &Element, constraints: Constraints) -> Size;

    fn layout(&self, element: &mut Element, constraints: Constraints) -> Size;

    /// Returns the distance from the top of the box to the first baseline of the
    /// box's contents for the given `constraints`, or [`None`] if this [`View`]
    /// does not have any baselines.
    ///
    /// Unlike [`View::distance_to_baseline`], this method takes [`Constraints`]
    /// as an argument and computes the baseline location as if the [`View`] was
    /// laid out by the parent using those [`Constraints`].
    ///
    /// Similar to the intrinsic width/height and [`View::measure`], calling this
    /// function in [`View::layout`] is expensive, as it can result in O(N^2) layout
    /// performance, where N is the number of render objects in the render subtree.
    fn measure_baseline(
        &self,
        element: &Element,
        constraints: Constraints,
        baseline: TextBaseline,
    ) -> Option<PositiveFinite<f32>>;

    /// Returns the distance from the y-coordinate of the position of the [`View`]
    /// to the y-coordinate of the first given baseline in the [`View`]'s
    /// contents.
    ///
    /// Used by certain layout models to align adjacent [`View`]s on a common
    /// baseline, regardless of padding, font size differences, etc. If there is
    /// no baseline, this function returns [`None`].
    ///
    /// Only call this function after calling [`View::layout`] on this
    /// [`View`]. You are only allowed to call this from the parent of this box
    /// during that parent's [performLayout] or [paint] functions.
    fn distance_to_baseline(
        &self,
        element: &mut Element,
        baseline: TextBaseline,
    ) -> Option<PositiveFinite<f32>>;

    /// Determines the set of [`View`]s located at the given position.
    ///
    /// Returns true, and adds any [`View`]s that contain the point to the
    /// given hit test result, if this [`View`] or one of its descendants
    /// absorbs the hit (preventing [`View`]s below this one from being hit).
    /// Returns false if the hit can continue to other [`View`]s below this one.
    ///
    /// The caller is responsible for transforming `position` from global
    /// coordinates to its location relative to the origin of this [`View`].
    /// This [`View`] is responsible for checking whether the given position is
    /// within its bounds.s
    ///
    /// Hit testing requires layout to be up-to-date but does not require drawing
    /// to be up-to-date. That means an [`View`] can rely upon [`View::layout`]
    /// having been called in [`View::hit_test`] but cannot rely upon [`View::draw`]
    /// having been called.
    fn hit_test(&self, element: &Element, result: &mut HitTestResult, position: Offset) -> bool;

    fn draw(&self, element: &mut Element, canvas: &mut Canvas);
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
        let (elements, state) = <T as View>::mount(self, ctx);

        if TypeId::of::<T::State>() == TypeId::of::<ElementState>()
            && size_of::<T::State>() == size_of::<ElementState>()
        {
            // Since this is an owned value, we need to mark it as a manually dropped value so that
            // it doesn't get immediately dropped when we return it after transmuting it.
            let state = ManuallyDrop::new(state);

            // SAFETY: This is probably safe so long as there are no TypeId + size collisions
            let state = unsafe { std::mem::transmute_copy::<T::State, ElementState>(&state) };

            return (elements, state);
        }

        (elements, ElementState::new(state))
    }
}

impl ViewLayoutMarker for () {
    type Width = Bounded;
    type Height = Bounded;

    type WidthIntrinsic = HasIntrinsic;
    type HeightIntrinsic = HasIntrinsic;
}

impl View for () {
    type State = ();

    fn mount(&self, _: &mut UpdateCtx) -> (Vec<Element>, Self::State) {
        (Vec::new(), ())
    }

    fn update(&self, _: &mut Element, _: &Self, _: &mut UpdateCtx) {}

    fn message(&self, _: &mut Element, _: MessageCtx) {}

    fn min_intrinsic_width(&self, _: &Element, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        Some(as_const!(PositiveFinite, f32, 0.0))
    }

    fn max_intrinsic_width(&self, _: &Element, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        Some(as_const!(PositiveFinite, f32, 0.0))
    }

    fn min_intrinsic_height(&self, _: &Element, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        Some(as_const!(PositiveFinite, f32, 0.0))
    }

    fn max_intrinsic_height(&self, _: &Element, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        Some(as_const!(PositiveFinite, f32, 0.0))
    }

    fn measure(&self, _: &Element, constraints: Constraints) -> Size {
        constraints.smallest()
    }

    fn layout(&self, _: &mut Element, constraints: Constraints) -> Size {
        constraints.smallest()
    }

    fn measure_baseline(
        &self,
        _: &Element,
        _: Constraints,
        _: TextBaseline,
    ) -> Option<PositiveFinite<f32>> {
        None
    }

    fn distance_to_baseline(
        &self,
        _: &mut Element,
        _: TextBaseline,
    ) -> Option<PositiveFinite<f32>> {
        None
    }

    fn hit_test(&self, _: &Element, _: &mut HitTestResult, _: Offset) -> bool {
        // TODO(trevin): should this add itself to the hit test result?
        false
    }

    fn draw(&self, _: &mut Element, _: &mut Canvas) {}
}
