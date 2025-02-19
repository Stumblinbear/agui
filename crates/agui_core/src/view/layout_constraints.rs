use std::{marker::PhantomData, rc::Rc, sync::Arc};

use crate::view::View;

mod sealed {
    pub trait LayoutConstraint {}
}

/// Indicates that the view has no upper bound for the given axis and will expand to fill as much
/// space as possible.
pub struct Unbounded;
impl sealed::LayoutConstraint for Unbounded {}

/// Indicates that the view has an upper bound for the given axis that is not infinite.
pub struct Bounded;
impl sealed::LayoutConstraint for Bounded {}

/// Used to indicate that the view does not have a constraint for the given axis and will inherit
/// the constraint from its child.
pub struct InheritedBound;
impl sealed::LayoutConstraint for InheritedBound {}

#[diagnostic::on_unimplemented(message = "fireuhgiu")]
pub trait ViewLayoutConstraints {
    type Width: sealed::LayoutConstraint;
    type Height: sealed::LayoutConstraint;
}

impl<Width, Height> ViewLayoutConstraints for &dyn View<Width = Width, Height = Height>
where
    Width: sealed::LayoutConstraint,
    Height: sealed::LayoutConstraint,
{
    type Width = Width;
    type Height = Height;
}

impl<Width, Height> ViewLayoutConstraints for Box<dyn View<Width = Width, Height = Height>>
where
    Width: sealed::LayoutConstraint,
    Height: sealed::LayoutConstraint,
{
    type Width = Width;
    type Height = Height;
}

impl<Width, Height> ViewLayoutConstraints for Rc<dyn View<Width = Width, Height = Height>>
where
    Width: sealed::LayoutConstraint,
    Height: sealed::LayoutConstraint,
{
    type Width = Width;
    type Height = Height;
}

impl<Width, Height> ViewLayoutConstraints for Arc<dyn View<Width = Width, Height = Height>>
where
    Width: sealed::LayoutConstraint,
    Height: sealed::LayoutConstraint,
{
    type Width = Width;
    type Height = Height;
}

pub trait ResolveLayoutConstraint {
    type Value: sealed::LayoutConstraint;
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
