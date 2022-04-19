use crate::draw_call::DrawCall;

pub struct Layer {
    pub segments: Vec<LayerSegment>,
}

pub struct LayerSegment {
    pub draw_calls: Vec<DrawCall>,
}
