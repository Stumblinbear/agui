use std::sync::Arc;

use parking_lot::Mutex;

use crate::{
    renderer::fonts::VelloFonts,
    view::{VelloView, VelloViewHandle},
};

pub mod fonts;
#[cfg(feature = "window")]
pub mod window;

#[derive(Default, Clone)]
pub struct VelloRenderer {
    fonts: Arc<Mutex<VelloFonts>>,
}

impl VelloRenderer {
    pub fn new() -> VelloRenderer {
        Self::default()
    }

    pub(crate) fn new_view(&self) -> (VelloView, VelloViewHandle) {
        let view = VelloView::new(Arc::clone(&self.fonts));
        let handle = view.handle();

        (view, handle)
    }
}

impl PartialEq for VelloRenderer {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.fonts, &other.fonts)
    }
}
