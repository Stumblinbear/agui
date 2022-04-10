use downcast_rs::Downcast;

use crate::widget::{BuildContext, BuildResult};

use super::StatefulWidget;

/// Implements the widget's `build()` method.
pub trait StatelessWidget: std::fmt::Debug + Downcast {
    fn build(&self, ctx: &mut BuildContext<()>) -> BuildResult;
}

impl<W> StatefulWidget for W
where
    W: StatelessWidget,
{
    type State = ();

    fn build(&self, ctx: &mut BuildContext<()>) -> BuildResult {
        self.build(ctx)
    }
}
