use std::collections::HashSet;

use agui_core::{
    engine::event::WidgetEvent,
    plugin::{EnginePlugin, PluginContext},
    widget::{BuildContext, WidgetContext, WidgetId},
};

use crate::state::mouse::Mouse;

#[derive(Debug, Default)]
pub struct HoveringPluginState {
    pub widget_ids: HashSet<WidgetId>,
}

impl HoveringPluginState {
    pub fn is_hovering(&self, widget_id: WidgetId) -> bool {
        self.widget_ids.contains(&widget_id)
    }
}

#[derive(Default)]
pub struct HoveringPlugin;

impl EnginePlugin for HoveringPlugin {
    fn on_update(&self, _ctx: &mut PluginContext) {}

    fn on_build(&self, _ctx: &mut PluginContext) {}

    fn on_layout(&self, ctx: &mut PluginContext) {
        let hovering = ctx.init_global(HoveringPluginState::default);

        if let Some(mouse) = ctx.try_use_global::<Mouse>() {
            match &mouse.read().pos {
                Some(pos) => {
                    let hovering_ids = ctx
                        .get_tree()
                        .iter()
                        .filter(|widget_id| {
                            match ctx.get_tree().get(*widget_id).and_then(|node| node.rect) {
                                Some(rect) => rect.contains((pos.x as f32, pos.y as f32)),
                                None => false,
                            }
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

    fn on_events(&self, _ctx: &mut PluginContext, _events: &[WidgetEvent]) {}
}

pub trait HoveringExt {
    fn is_hovering(&mut self) -> bool;
}

impl<'ui, 'ctx> HoveringExt for BuildContext<'ui, 'ctx> {
    fn is_hovering(&mut self) -> bool {
        self.init_global(HoveringPluginState::default)
            .read()
            .is_hovering(self.get_widget())
    }
}

impl<'ui, 'ctx> HoveringExt for WidgetContext<'ui, 'ctx> {
    fn is_hovering(&mut self) -> bool {
        self.init_global(HoveringPluginState::default)
            .read()
            .is_hovering(self.get_widget())
    }
}
