use crate::context::{MessageCtx, UpdateCtx};

/// The operation a [`dispatch`](crate::widget::Widget::dispatch) walk should perform
/// when it arrives at its destination element.
pub enum Dispatch<'b, 'a: 'b> {
    Message(&'b mut MessageCtx),
    Rebuild(&'b mut UpdateCtx<'a>),
    /// Rebuild the destination because a value it depends on changed, running its
    /// dependency-change hook before the rebuild.
    DependencyChanged(&'b mut UpdateCtx<'a>),
}
