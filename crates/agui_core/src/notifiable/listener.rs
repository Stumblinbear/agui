use crate::{computed::ComputedId, plugin::PluginId, widget::WidgetId};

/// A combined-type for anything that can listen for events in the system.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ListenerId {
    Widget(WidgetId),
    Computed(WidgetId, ComputedId),
    Plugin(PluginId),
}

impl ListenerId {
    /// Returns `None` if not tied to a widget.
    pub fn widget_id(&self) -> Option<WidgetId> {
        match self {
            Self::Widget(widget_id) | Self::Computed(widget_id, _) => Some(*widget_id),
            Self::Plugin(_) => None,
        }
    }
}

impl From<WidgetId> for ListenerId {
    fn from(widget_id: WidgetId) -> Self {
        Self::Widget(widget_id)
    }
}

impl From<(WidgetId, ComputedId)> for ListenerId {
    fn from((widget_id, computed_id): (WidgetId, ComputedId)) -> Self {
        Self::Computed(widget_id, computed_id)
    }
}

impl From<PluginId> for ListenerId {
    fn from(plugin_id: PluginId) -> Self {
        Self::Plugin(plugin_id)
    }
}
