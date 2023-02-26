use crate::unit::{Layout, LayoutType};

#[derive(Default)]
pub struct LayoutResult {
    pub layout_type: LayoutType,
    pub layout: Layout,
}
