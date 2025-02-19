use typed_floats::{as_const, Positive, PositiveFinite};

use crate::{
    constraints::Constraints,
    context::{LayoutCtx, MessageCtx, UpdateCtx},
    hit_test::HitTestResult,
    offset::Offset,
    size::Size,
    text_baseline::TextBaseline,
    tree::{Tree, TreeState},
};

pub trait ViewLifecycle {
    fn state(&self) -> TreeState;

    fn children(&self) -> Vec<Tree>;

    fn update(&self, ctx: UpdateCtx);

    fn message(&self, ctx: MessageCtx);
}

pub trait ViewLayout {
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
        tree: &Tree,
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
        tree: &Tree,
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
        tree: &Tree,
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
        tree: &Tree,
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
    fn measure(&self, tree: &Tree, constraints: Constraints) -> Size;

    fn layout(&self, ctx: LayoutCtx, constraints: Constraints);

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
        tree: &Tree,
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
        tree: &mut Tree,
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
    fn hit_test(&self, tree: &Tree, result: &mut HitTestResult, position: Offset) -> bool;
}

pub trait ViewDraw<Renderer = ()> {
    fn draw(&self, tree: &mut Tree, renderer: &mut Renderer);
}

pub trait View<Renderer = ()>: ViewLifecycle + ViewLayout + ViewDraw<Renderer> {}

impl<T, Renderer> View<Renderer> for T where T: ViewLifecycle + ViewLayout + ViewDraw<Renderer> {}

impl ViewLifecycle for () {
    fn state(&self) -> TreeState {
        TreeState::none()
    }

    fn children(&self) -> Vec<Tree> {
        Vec::new()
    }

    fn update(&self, _: UpdateCtx) {}

    fn message(&self, _: MessageCtx) {}
}

impl ViewLayout for () {
    fn min_intrinsic_width(&self, _: &Tree, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        Some(as_const!(PositiveFinite, f32, 0.0))
    }

    fn max_intrinsic_width(&self, _: &Tree, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        Some(as_const!(PositiveFinite, f32, 0.0))
    }

    fn min_intrinsic_height(&self, _: &Tree, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        Some(as_const!(PositiveFinite, f32, 0.0))
    }

    fn max_intrinsic_height(&self, _: &Tree, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        Some(as_const!(PositiveFinite, f32, 0.0))
    }

    fn measure(&self, _: &Tree, constraints: Constraints) -> Size {
        constraints.smallest()
    }

    fn layout(&self, mut ctx: LayoutCtx, constraints: Constraints) {
        ctx.size = constraints.smallest();
    }

    fn measure_baseline(
        &self,
        _: &Tree,
        _: Constraints,
        _: TextBaseline,
    ) -> Option<PositiveFinite<f32>> {
        None
    }

    fn distance_to_baseline(&self, _: &mut Tree, _: TextBaseline) -> Option<PositiveFinite<f32>> {
        None
    }

    fn hit_test(&self, _: &Tree, _: &mut HitTestResult, _: Offset) -> bool {
        // TODO(trevin): should this add itself to the hit test result?
        false
    }
}

impl<Renderer> ViewDraw<Renderer> for () {
    fn draw(&self, _: &mut Tree, _: &mut Renderer) {}
}
