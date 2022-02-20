use crate::{widget::HandlerId, plugin::PluginId, widget::WidgetId};

/// A combined-type for anything that can listen for events in the system.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ListenerId {
    Widget(WidgetId),
    Handler(WidgetId, HandlerId),
    Plugin(PluginId),
}

impl ListenerId {
    /// Returns `None` if not tied to a widget.
    pub fn widget_id(&self) -> Option<WidgetId> {
        match self {
            Self::Widget(widget_id) | Self::Handler(widget_id, ..) => Some(*widget_id),
            Self::Plugin(..) => None,
        }
    }
}

impl From<WidgetId> for ListenerId {
    fn from(widget_id: WidgetId) -> Self {
        Self::Widget(widget_id)
    }
}

impl From<(WidgetId, HandlerId)> for ListenerId {
    fn from((widget_id, handler_id): (WidgetId, HandlerId)) -> Self {
        Self::Handler(widget_id, handler_id)
    }
}

impl From<PluginId> for ListenerId {
    fn from(plugin_id: PluginId) -> Self {
        Self::Plugin(plugin_id)
    }
}
