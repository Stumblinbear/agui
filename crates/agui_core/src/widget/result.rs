use super::{Widget, WidgetRef};

/// Encapsulates the result of a widget `build()` method.
#[non_exhaustive]
pub enum BuildResult {
    /// Indicates that the widget has no children.
    None,

    /// The widget contains children.
    Some(Vec<WidgetRef>),

    /// The widget has failed to build properly, and should construction should halt.
    ///
    /// # Panics
    ///
    /// Currently this results in a `panic!()`, however that may change in the future.
    Err(Box<dyn std::error::Error>),
}

impl<W> From<W> for BuildResult
where
    W: Widget,
{
    fn from(widget: W) -> Self {
        Self::Some(vec![WidgetRef::new(widget)])
    }
}

impl From<WidgetRef> for BuildResult {
    fn from(widget: WidgetRef) -> Self {
        Self::Some(vec![widget])
    }
}

impl From<&WidgetRef> for BuildResult {
    fn from(widget: &WidgetRef) -> Self {
        Self::Some(vec![WidgetRef::clone(widget)])
    }
}

impl From<Vec<WidgetRef>> for BuildResult {
    fn from(widgets: Vec<WidgetRef>) -> Self {
        if widgets.is_empty() {
            Self::None
        } else {
            Self::Some(widgets)
        }
    }
}

impl From<&Vec<WidgetRef>> for BuildResult {
    fn from(widgets: &Vec<WidgetRef>) -> Self {
        if widgets.is_empty() {
            Self::None
        } else {
            Self::Some(widgets.clone())
        }
    }
}
