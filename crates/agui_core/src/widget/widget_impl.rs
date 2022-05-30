use downcast_rs::Downcast;

use crate::manager::Data;

use super::{BuildContext, BuildResult};

/// Implements the widget's `build()` method.
pub trait WidgetImpl: std::fmt::Debug + Downcast + Sized {
    type State: Data + Default;

    /// Called whenever this widget is rebuilt.
    ///
    /// This method may be called when any parent is rebuilt or when its internal state changes.
    fn build(&self, ctx: &mut BuildContext<Self>) -> BuildResult;
}
