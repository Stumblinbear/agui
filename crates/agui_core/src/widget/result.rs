use crate::manager::widget::WidgetBuilder;

use super::Widget;

/// Encapsulates the result of a widget `build()` method.
#[non_exhaustive]
pub enum BuildResult {
    /// Indicates that the widget has no children.
    None,

    /// The widget contains children.
    Some(Vec<Widget>),

    /// The widget has failed to build properly, and should construction should halt.
    ///
    /// # Panics
    ///
    /// Currently this results in a `panic!()`, however that may change in the future.
    Err(Box<dyn std::error::Error>),
}

impl<W> From<W> for BuildResult
where
    W: WidgetBuilder,
{
    fn from(widget: W) -> Self {
        Self::Some(vec![Widget::from(widget)])
    }
}

impl From<Widget> for BuildResult {
    fn from(widget: Widget) -> Self {
        Self::Some(vec![widget])
    }
}

impl From<&Widget> for BuildResult {
    fn from(widget: &Widget) -> Self {
        widget.clone().into()
    }
}

impl From<Vec<Widget>> for BuildResult {
    fn from(widgets: Vec<Widget>) -> Self {
        if widgets.is_empty() {
            Self::None
        } else {
            Self::Some(widgets)
        }
    }
}

impl From<&Vec<Widget>> for BuildResult {
    fn from(widget: &Vec<Widget>) -> Self {
        widget.clone().into()
    }
}
