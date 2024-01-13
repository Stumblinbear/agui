use crate::view::VelloView;

#[cfg(feature = "window")]
pub mod window;

#[derive(Default, Clone)]
pub struct VelloRenderer {}

impl VelloRenderer {
    pub fn new() -> VelloRenderer {
        Self::default()
    }

    pub fn new_view(&self) -> VelloView {
        VelloView
    }
}
