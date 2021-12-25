use std::collections::HashSet;

use agui_core::{context::WidgetContext, widget::WidgetId};

#[derive(Default)]
pub struct Hovering {
    pub widget_ids: HashSet<WidgetId>,
}

impl Hovering {
    pub fn is_hovering(&self, ctx: &WidgetContext) -> bool {
        self.widget_ids.contains(&ctx.get_self())
    }
}
