use std::rc::Rc;

use super::node::RenderNode;

#[derive(Default)]
pub struct RenderCanvas {
    pub nodes: Vec<Rc<RenderNode>>,
}
