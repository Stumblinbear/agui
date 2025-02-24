use std::marker::PhantomData;

mod private {
    pub trait Sealed {}
}

pub trait LayoutBoundMarker: private::Sealed + 'static {}

pub trait LayoutIntrinsicMarker: private::Sealed + 'static {}

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
