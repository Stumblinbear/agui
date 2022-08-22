use crate::widget::WidgetId;

/// Used to indicate a change to canvases in the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RenderEvent {
    /// A canvas has been spawned.
    Spawned {},

    /// A canvas has been redrawn.
    Rebuilt {},

    /// A canvas has been reparented.
    Reparent {},

    /// A canvas has been destroyed.
    Destroyed {},
}

// impl RenderEvent {
//     pub fn canvas_id(&self) -> &CanvasId {
//         match self {
//             WidgetEvent::Spawned { widget_id, .. }
//             | WidgetEvent::Rebuilt { widget_id, .. }
//             | WidgetEvent::Reparent { widget_id, .. }
//             | WidgetEvent::Layout { widget_id, .. }
//             | WidgetEvent::Destroyed { widget_id, .. } => widget_id,
//         }
//     }
// }
