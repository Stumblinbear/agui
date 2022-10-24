use crate::{handle::RenderHandle, storage::RenderStorage};

pub struct RenderContext<'ctx> {
    pub handle: &'ctx RenderHandle,

    pub storage: &'ctx mut RenderStorage,
}
