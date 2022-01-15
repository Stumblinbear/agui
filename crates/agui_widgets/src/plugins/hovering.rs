use std::collections::HashSet;

use agui_core::{
    context::WidgetContext, event::WidgetEvent, plugin::WidgetPlugin, widget::WidgetId,
};

use crate::state::mouse::Mouse;

#[derive(Debug, Default)]
pub struct HoveringPluginState {
    pub widget_ids: HashSet<WidgetId>,
}

impl HoveringPluginState {
    pub fn is_hovering(&self, ctx: &WidgetContext) -> bool {
        self.widget_ids.contains(
            &ctx.get_self()
                .widget_id()
                .expect("cannot check hover state outside of a widget context"),
        )
    }
}

#[derive(Default)]
pub struct HoveringPlugin;

impl WidgetPlugin for HoveringPlugin {
    fn pre_update(&self, _ctx: &mut WidgetContext) {}

    fn on_update(&self, _ctx: &mut WidgetContext) {}

    fn post_update(&self, ctx: &mut WidgetContext) {
        let hovering = ctx.init_global(HoveringPluginState::default);

        if let Some(mouse) = ctx.try_use_global::<Mouse>() {
            match &mouse.read().pos {
                Some(pos) => {
                    let hovering_ids = ctx
                        .get_tree()
                        .iter()
                        .filter(|widget_id| match ctx.get_rect_for(*widget_id) {
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

    fn on_events(&self, _ctx: &mut WidgetContext, _events: &[WidgetEvent]) {}
}

pub trait HoveringExt<'ui> {
    fn is_hovering(&mut self) -> bool;
}

impl<'ui> HoveringExt<'ui> for WidgetContext<'ui> {
    fn is_hovering(&mut self) -> bool {
        self.init_global(HoveringPluginState::default)
            .read()
            .is_hovering(self)
    }
}
