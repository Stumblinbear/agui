use downcast_rs::Downcast;

use crate::widget::BuildResult;

use super::{BuildContext, StatefulWidget};

/// Implements the widget's `build()` method.
pub trait StatelessWidget: std::fmt::Debug + Downcast + Sized {
    fn build(&self, ctx: &mut BuildContext<Self>) -> BuildResult;
}

impl<W> StatefulWidget for W
where
    W: StatelessWidget,
{
    type State = ();

    fn build(&self, ctx: &mut BuildContext<Self>) -> BuildResult {
        self.build(ctx)
    }
}
