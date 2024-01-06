use crate::{
    element::ContextRenderObject,
    render::{object::RenderObject, RenderObjectId},
    util::tree::Tree,
};

mod hit_test;
mod intrinsic_size;
mod layout;
mod mount;
mod unmount;

pub use hit_test::*;
pub use intrinsic_size::*;
pub use layout::*;
pub use mount::*;
pub use unmount::*;

pub struct RenderObjectContext<'ctx> {
    pub(crate) render_object_tree: &'ctx Tree<RenderObjectId, RenderObject>,

    pub render_object_id: &'ctx RenderObjectId,
}

impl ContextRenderObject for RenderObjectContext<'_> {
    fn render_object_id(&self) -> RenderObjectId {
        *self.render_object_id
    }
}
