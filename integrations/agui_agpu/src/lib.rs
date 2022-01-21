mod ext;

use agpu::{Frame, GpuHandle, GpuProgram};
use agui::{canvas::Canvas, engine::render::Renderer};
pub use ext::*;

pub struct AgpuRenderer {
    pub gpu: GpuHandle,
}

impl AgpuRenderer {
    pub fn using_gpu(gpu: GpuHandle) -> Self {
        Self { gpu }
    }

    pub fn from_program(program: &GpuProgram) -> Self {
        Self {
            gpu: GpuHandle::clone(&program.gpu),
        }
    }

    pub fn render(&mut self, frame: Frame) {}
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Copy, Clone)]
pub struct AgpuPicture {
    layer: u32,
    color: [f32; 4],
}

impl Renderer<AgpuPicture> for AgpuRenderer {
    fn draw(&self, canvas: &Canvas) -> AgpuPicture {
        todo!()
    }

    fn render(&self, picture: &AgpuPicture) {
        todo!()
    }
}
