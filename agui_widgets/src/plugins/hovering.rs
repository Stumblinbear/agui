use std::collections::HashSet;

use agui_core::{context::WidgetContext, event::WidgetEvent, plugin::WidgetPlugin};

use crate::state::{hovering::Hovering, mouse::Mouse};

#[derive(Default)]
pub struct HoveringPlugin {}

impl WidgetPlugin for HoveringPlugin {
    fn on_update(&self, ctx: &WidgetContext) {
        let hovering = ctx.init_global(Hovering::default);

        if let Some(mouse) = ctx.get_global::<Mouse>() {
            match &mouse.read().pos {
                Some(pos) => {
                    let hovering_ids = ctx
                        .get_tree()
                        .iter()
                        .filter(|widget_id| match ctx.get_rect(widget_id) {
                            Some(rect) => rect.contains((pos.x as f32, pos.y as f32)),
                            None => false,
                        })
                        .collect::<HashSet<_>>();

                    // If there are any differing widgets, update the list
                    if hovering
                        .read()
                        .widget_ids
                        .symmetric_difference(&hovering_ids)
                        .next()
                        .is_some()
                    {
                        hovering.write().widget_ids = hovering_ids;
                    }
                }
                None => {
                    if !hovering.read().widget_ids.is_empty() {
                        hovering.write().widget_ids.clear();
                    }
                }
            }
        }
    }

    fn on_events(&self, _ctx: &WidgetContext, _events: &[WidgetEvent]) {}
}
