use crate::{unit::Key, widget::WidgetId};

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WidgetKey(pub(crate) Option<WidgetId>, pub(crate) Key);

impl WidgetKey {
    pub fn get_owner(&self) -> Option<WidgetId> {
        self.0
    }

    pub fn get_key(&self) -> Key {
        self.1
    }
}

impl std::fmt::Debug for WidgetKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.1.fmt(f)
    }
}

impl std::fmt::Display for WidgetKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.1.fmt(f)
    }
}
