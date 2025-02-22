use std::{marker::PhantomData, rc::Rc, sync::Arc};

use crate::view::AnyView;

mod private {
    pub trait Sealed {}
}

pub trait LayoutBoundMarker: private::Sealed {}

pub trait LayoutIntrinsicMarker: private::Sealed {}

/// Indicates that the view has no upper bound for the given axis and will expand to fill as much
/// space as possible.
pub struct Unbounded;
impl private::Sealed for Unbounded {}
impl LayoutBoundMarker for Unbounded {}

/// Indicates that the view has an upper bound for the given axis that is not infinite.
pub struct Bounded;
impl private::Sealed for Bounded {}
impl LayoutBoundMarker for Bounded {}

/// Used to indicate that the view inherits its bound for the given axes from its child.
pub struct InheritedBound;
impl private::Sealed for InheritedBound {}

pub struct NoIntrinsic;
impl private::Sealed for NoIntrinsic {}
impl LayoutIntrinsicMarker for NoIntrinsic {}

pub struct HasIntrinsic;
impl private::Sealed for HasIntrinsic {}
impl LayoutIntrinsicMarker for HasIntrinsic {}

pub trait ViewLayoutMarker {
    type Width: LayoutBoundMarker;
    type Height: LayoutBoundMarker;

    type WidthIntrinsic: LayoutIntrinsicMarker;
    type HeightIntrinsic: LayoutIntrinsicMarker;
}

pub trait ResolveLayoutMarker {
    type Value: LayoutBoundMarker;
}

impl ResolveLayoutMarker for Unbounded {
    type Value = Unbounded;
}

impl ResolveLayoutMarker for Bounded {
    type Value = Bounded;
}

pub struct ResolveLayoutMarkerOr<Bound, Fallback> {
    _phantom: PhantomData<(Bound, Fallback)>,
}

impl<Fallback> ResolveLayoutMarker for ResolveLayoutMarkerOr<InheritedBound, Fallback>
where
    Fallback: ResolveLayoutMarker,
{
    type Value = Fallback::Value;
}

impl<Fallback> ResolveLayoutMarker for ResolveLayoutMarkerOr<Unbounded, Fallback> {
    type Value = Unbounded;
}

impl<Fallback> ResolveLayoutMarker for ResolveLayoutMarkerOr<Bounded, Fallback> {
    type Value = Bounded;
}

macros::impl_view_layout_constraints!(
    &dyn AnyView<
        Width = Width,
        Height = Height,
        WidthIntrinsic = WidthIntrinsic,
        HeightIntrinsic = HeightIntrinsic,
    >
);
macros::impl_view_layout_constraints!(
    Box<
        dyn AnyView<
            Width = Width,
            Height = Height,
            WidthIntrinsic = WidthIntrinsic,
            HeightIntrinsic = HeightIntrinsic,
        >,
    >
);
macros::impl_view_layout_constraints!(
    Rc<
        dyn AnyView<
            Width = Width,
            Height = Height,
            WidthIntrinsic = WidthIntrinsic,
            HeightIntrinsic = HeightIntrinsic,
        >,
    >
);
macros::impl_view_layout_constraints!(
    Arc<
        dyn AnyView<
            Width = Width,
            Height = Height,
            WidthIntrinsic = WidthIntrinsic,
            HeightIntrinsic = HeightIntrinsic,
        >,
    >
);

mod macros {
    // Used to implement View for the given smart pointer (e.g. Box, Rc, Arc)
    macro_rules! impl_view_layout_constraints {
        (
            // The smart pointer type
            $ptr:ty
        ) => {
            impl<Width, Height, WidthIntrinsic, HeightIntrinsic> ViewLayoutMarker for $ptr
            where
                Width: LayoutBoundMarker,
                Height: LayoutBoundMarker,
                WidthIntrinsic: LayoutIntrinsicMarker,
                HeightIntrinsic: LayoutIntrinsicMarker,
            {
                type Width = Width;
                type Height = Height;

                type WidthIntrinsic = WidthIntrinsic;
                type HeightIntrinsic = HeightIntrinsic;
            }
        };
    }

    pub(crate) use impl_view_layout_constraints;
}
