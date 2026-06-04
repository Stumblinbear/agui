use typed_floats::{Positive, PositiveFinite};

use crate::{
    constraints::Constraints,
    hit_test::{HitTest, HitTestResult},
    offset::Offset,
    paint::PaintCtx,
    render_object::RenderObject,
    size::Size,
    text_baseline::TextBaseline,
};

mod any_render_box;

pub use any_render_box::*;

/// A render object in a 2D Cartesian coordinate system.
///
/// Its parent passes [`Constraints`] down, and the box chooses a [`Size`] within them. Boxes also
/// report intrinsic sizes and baselines, paint, and answer hit tests.
pub trait RenderBox: RenderObject {
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
    fn min_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>>;

    /// Returns the smallest width beyond which increasing the width never
    /// decreases the preferred height. The preferred height is the value that
    /// would be returned by [`Self::min_intrinsic_height`] for that width.
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
    fn max_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>>;

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
    fn min_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>>;

    /// Returns the smallest height beyond which increasing the height never
    /// decreases the preferred width. The preferred width is the value that
    /// would be returned by [`Self::min_intrinsic_width`] for that height.
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
    fn max_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>>;

    /// Returns the [`Size`] that this [`RenderBox`] would like to be given the
    /// provided [`Constraints`].
    ///
    /// The size returned by this method is guaranteed to be the same size that
    /// this [`RenderBox`] computes for itself during layout given the same
    /// constraints.
    ///
    /// This function should only be called on one's children. Calling this
    /// function couples the child with the parent so that when the child's layout
    /// changes, the parent is also laid out.
    ///
    /// Calling this function is expensive as it can result in O(N^2) behavior.
    fn measure(&self, constraints: Constraints) -> Size;

    fn layout(&mut self, constraints: Constraints) -> Size;

    /// Returns the distance from the top of the box to the first baseline of the
    /// box's contents for the given `constraints`, or [`None`] if this [`RenderBox`]
    /// does not have any baselines.
    ///
    /// Unlike [`RenderBox::distance_to_baseline`], this method takes [`Constraints`]
    /// as an argument and computes the baseline location as if the [`RenderBox`] was
    /// laid out by the parent using those [`Constraints`].
    ///
    /// Similar to the intrinsic width/height and [`RenderBox::measure`], calling this
    /// function in [`RenderBox::layout`] is expensive, as it can result in O(N^2) layout
    /// performance, where N is the number of render objects in the render subtree.
    fn measure_baseline(
        &self,
        constraints: Constraints,
        baseline: TextBaseline,
    ) -> Option<PositiveFinite<f32>>;

    /// Returns the distance from the y-coordinate of the position of the [`RenderBox`]
    /// to the y-coordinate of the first given baseline in the [`RenderBox`]'s
    /// contents.
    ///
    /// Used by certain layout models to align adjacent [`RenderBox`]s on a common
    /// baseline, regardless of padding, font size differences, etc. If there is
    /// no baseline, this function returns [`None`].
    ///
    /// Only call this function after calling [`RenderBox::layout`] on this
    /// [`RenderBox`]. You are only allowed to call this from the parent of this box
    /// during that parent's [`RenderBox::layout`] or [`RenderBox::paint`] functions.
    fn distance_to_baseline(&mut self, baseline: TextBaseline) -> Option<PositiveFinite<f32>>;

    /// Determines the set of box render objects located at `position`.
    ///
    /// Returns [`HitTest::Absorb`], and adds any render objects that contain the point to `result`,
    /// if this render object or one of its descendants absorbs the hit (preventing render objects
    /// below this one from being hit). Returns [`HitTest::Pass`] if the hit can continue to render
    /// objects below this one.
    ///
    /// The caller is responsible for transforming `position` into this box's local coordinate space.
    /// This box is responsible for checking whether `position` is within its bounds.
    ///
    /// Hit testing requires layout to be up to date but not paint: an implementation may rely on
    /// [`RenderBox::layout`] having been called, but not on [`RenderBox::paint`].
    fn hit_test(&self, result: &mut HitTestResult, position: Offset) -> HitTest;

    /// Paints this box and its descendants. `offset` is this box's top-left in the coordinate space of
    /// the enclosing boundary's layer. A box draws its own geometry at `offset` and paints each child at
    /// `offset` plus that child's layout position.
    fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset);
}
