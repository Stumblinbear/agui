use crate::render::RenderObjectImpl;

#[derive(Default)]
pub struct RenderView {}

impl RenderObjectImpl for RenderView {
    fn is_sized_by_parent(&self) -> bool {
        true
    }
}
