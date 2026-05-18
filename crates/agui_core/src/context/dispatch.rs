use crate::context::{MessageCtx, UpdateCtx};

/// The operation a [`dispatch`](crate::view::View::dispatch) walk should perform
/// when it arrives at its destination element.
pub enum Dispatch<'b, 'a: 'b> {
    Message(&'b mut MessageCtx),
    Rebuild(&'b mut UpdateCtx<'a>),
}
