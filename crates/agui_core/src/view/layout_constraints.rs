use std::{marker::PhantomData, rc::Rc, sync::Arc};

use crate::view::AnyView;

pub trait LayoutConstraintMarker: 'static {}

/// Indicates that the view has no upper bound for the given axis and will expand to fill as much
/// space as possible.
pub struct Unbounded;
impl LayoutConstraintMarker for Unbounded {}

/// Indicates that the view has an upper bound for the given axis that is not infinite.
pub struct Bounded;
impl LayoutConstraintMarker for Bounded {}

/// Used to indicate that the view does not have a constraint for the given axis and will inherit
/// the constraint from its child.
pub struct InheritedBound;
impl LayoutConstraintMarker for InheritedBound {}

pub trait ViewLayoutConstraints {
    type Width: LayoutConstraintMarker;
    type Height: LayoutConstraintMarker;
}

impl<Width, Height> ViewLayoutConstraints for &dyn AnyView<Width = Width, Height = Height>
where
    Width: LayoutConstraintMarker,
    Height: LayoutConstraintMarker,
{
    type Width = Width;
    type Height = Height;
}

impl<Width, Height> ViewLayoutConstraints for Box<dyn AnyView<Width = Width, Height = Height>>
where
    Width: LayoutConstraintMarker,
    Height: LayoutConstraintMarker,
{
    type Width = Width;
    type Height = Height;
}

impl<Width, Height> ViewLayoutConstraints for Rc<dyn AnyView<Width = Width, Height = Height>>
where
    Width: LayoutConstraintMarker,
    Height: LayoutConstraintMarker,
{
    type Width = Width;
    type Height = Height;
}

impl<Width, Height> ViewLayoutConstraints for Arc<dyn AnyView<Width = Width, Height = Height>>
where
    Width: LayoutConstraintMarker,
    Height: LayoutConstraintMarker,
{
    type Width = Width;
    type Height = Height;
}

pub trait ResolveLayoutConstraint {
    type Value: LayoutConstraintMarker;
}

impl ResolveLayoutConstraint for Unbounded {
    type Value = Unbounded;
}

impl ResolveLayoutConstraint for Bounded {
    type Value = Bounded;
}

pub struct ResolveConstraintOr<Bound, Fallback>
where
    Fallback: ResolveLayoutConstraint,
{
    _phantom: PhantomData<(Bound, Fallback)>,
}

impl<Fallback> ResolveLayoutConstraint for ResolveConstraintOr<InheritedBound, Fallback>
where
    Fallback: ResolveLayoutConstraint,
{
    type Value = Fallback::Value;
}

impl<Fallback> ResolveLayoutConstraint for ResolveConstraintOr<Unbounded, Fallback>
where
    Fallback: ResolveLayoutConstraint,
{
    type Value = Unbounded;
}

impl<Fallback> ResolveLayoutConstraint for ResolveConstraintOr<Bounded, Fallback>
where
    Fallback: ResolveLayoutConstraint,
{
    type Value = Bounded;
}
