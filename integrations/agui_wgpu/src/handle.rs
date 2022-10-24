use wgpu::{Device, Queue};

pub struct RenderHandle {
    pub device: Device,
    pub queue: Queue,
}
