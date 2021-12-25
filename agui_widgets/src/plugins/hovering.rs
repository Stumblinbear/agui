use std::collections::HashSet;

use agui_core::{context::WidgetContext, plugin::WidgetPlugin, WidgetManager};

use crate::state::{hovering::Hovering, mouse::Mouse};

#[derive(Default)]
pub struct HoveringPlugin {}

impl WidgetPlugin for HoveringPlugin {
    fn on_update(&self, manager: &WidgetManager, ctx: &WidgetContext) {
        let hovering = ctx.init_global::<Hovering>();

        if let Some(mouse) = ctx.get_global::<Mouse>() {
            match &mouse.read().pos {
                Some(pos) => {
                    let hovering_ids = manager
                        .get_tree()
                        .iter()
                        .filter(|widget_id| match manager.get_rect(widget_id) {
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
}
