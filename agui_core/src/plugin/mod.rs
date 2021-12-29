use downcast_rs::{impl_downcast, Downcast};

use crate::{context::WidgetContext, event::WidgetEvent};

/// A plugin for the widget system.
///
/// It's primarily designed as a widget without being a widget. It receives updates just the same
/// as a widget with a `build()` method, just without existing in the tree.
pub trait WidgetPlugin: Downcast + Send + Sync {
    fn on_update(&self, ctx: &WidgetContext);

    fn on_events(&self, ctx: &WidgetContext, events: &[WidgetEvent]);
}

impl_downcast!(WidgetPlugin);
