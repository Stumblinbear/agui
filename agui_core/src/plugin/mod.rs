use downcast_rs::{impl_downcast, Downcast};

use crate::{context::WidgetContext, WidgetManager};

/// A plugin for the widget manager.
/// 
/// It's primarily designed as a widget without being a widget. It receives updates just the same
/// as a widget with a `build()` method, just without existing in the tree.
pub trait WidgetPlugin: Downcast {
    fn on_update(&self, manager: &WidgetManager, ctx: &WidgetContext);
}

impl_downcast!(WidgetPlugin);
