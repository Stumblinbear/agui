use crate::widget::{IntoWidget, Widget};

pub trait Plugin {
    /// Allow the plugin to inject widgets into the tree.
    ///
    /// This is called when the app is first created, and is used to inject the root widget into the
    /// tree. The `child` parameter must be returned as a descendant of the returned widget.
    fn build<T: IntoWidget>(&self, child: impl Into<Option<T>>) -> Widget;
}
