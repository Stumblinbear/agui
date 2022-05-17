use std::rc::Rc;

use agpu::Buffer;

use super::{node::RenderNode, texture::RenderTexture};

pub struct RenderLayer {
    pub pos: Buffer,

    pub texture: Option<Rc<RenderTexture>>,

    pub nodes: Vec<RenderNode>,
}
