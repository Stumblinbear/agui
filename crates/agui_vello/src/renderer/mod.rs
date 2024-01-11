use crate::renderer::binding::VelloViewBinding;

pub mod binding;

#[cfg(feature = "window")]
pub mod window;

#[derive(Default, Clone)]
pub struct VelloRenderer {}

impl VelloRenderer {
    pub fn new() -> VelloRenderer {
        Self::default()
    }

    pub fn new_view_binding(&self) -> VelloViewBinding {
        VelloViewBinding
    }
}
