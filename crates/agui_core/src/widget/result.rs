use std::convert::Infallible;

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

impl std::ops::FromResidual<Option<Infallible>> for BuildResult {
    fn from_residual(_: Option<Infallible>) -> Self {
        BuildResult::None
    }
}

impl<E> std::ops::FromResidual<Result<Infallible, E>> for BuildResult
where
    E: std::error::Error + 'static,
{
    fn from_residual(result: Result<Infallible, E>) -> Self {
        Self::Err(Box::new(result.unwrap_err()))
    }
}

impl From<Widget> for BuildResult {
    fn from(widget: Widget) -> Self {
        Self::Some(vec![widget])
    }
}

impl From<&Widget> for BuildResult {
    fn from(widget: &Widget) -> Self {
        Self::Some(vec![widget.clone()])
    }
}

impl<'a, I> From<I> for BuildResult
where
    I: IntoIterator<Item = &'a Widget>,
{
    fn from(iter: I) -> Self {
        let widgets = iter.into_iter().map(Widget::clone).collect::<Vec<_>>();

        if widgets.is_empty() {
            Self::None
        } else {
            Self::Some(widgets)
        }
    }
}
