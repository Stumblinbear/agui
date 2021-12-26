use std::collections::HashSet;

use agui_core::{context::WidgetContext, plugin::WidgetPlugin, WidgetManager};

use crate::state::{hovering::Hovering, mouse::Mouse};

#[derive(Default)]
pub struct ProviderPlugin {}

impl WidgetPlugin for ProviderPlugin {
    fn on_update(&self, manager: &WidgetManager, ctx: &WidgetContext) {
        
    }
}

impl ProviderPlugin {
    
}
