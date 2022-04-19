use layer::Layer;

pub mod draw_call;
pub mod layer;

pub struct RenderManager {
    pub layers: Vec<Layer>,
}
