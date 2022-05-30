use crate::{unit::Key, widget::WidgetId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WidgetKey(pub(super) Option<WidgetId>, pub(super) Key);

impl WidgetKey {
    pub fn get_owner(&self) -> Option<WidgetId> {
        self.0
    }

    pub fn get_key(&self) -> Key {
        self.1
    }
}
