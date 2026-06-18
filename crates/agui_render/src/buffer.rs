use crate::paint::scene::Scene;

/// A backend that composes a scene into a pixel buffer on the CPU.
pub trait BufferRenderer {
    /// A short stable identifier for the backend, used wherever a renderer must be named.
    const NAME: &'static str;

    /// Composes `scene` into a `width` by `height` image, returning its row-major RGBA8 pixels.
    /// `scene`'s geometry is in logical pixels scaled by `scale` to physical pixels.
    fn render(&mut self, scene: &Scene, width: u32, height: u32, scale: f64) -> Vec<u8>;
}
