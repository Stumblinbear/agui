use crate::unit::{Offset, Size};

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Default)]
pub(crate) struct LayoutResult {
    pub parent_uses_size: bool,

    pub size: Option<Size>,
    pub offset: Option<Offset>,
}
