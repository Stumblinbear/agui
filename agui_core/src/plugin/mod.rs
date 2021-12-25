use downcast_rs::{impl_downcast, Downcast};

use crate::{context::WidgetContext, WidgetManager};

pub trait WidgetPlugin: Downcast {
    fn on_update(&self, manager: &WidgetManager, ctx: &WidgetContext);
}

impl_downcast!(WidgetPlugin);
