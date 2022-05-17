use std::rc::Rc;

use agpu::Buffer;

use super::draw_call::DrawCall;

pub struct RenderNode {
    pub pos: Buffer,

    pub draw_calls: Vec<Rc<DrawCall>>,
}
