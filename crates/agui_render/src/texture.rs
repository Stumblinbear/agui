use agui_core::paint::scene::Scene;

/// A backend that composes a scene into a `wgpu` texture.
pub trait TextureRenderer {
    /// A short stable identifier for the backend, used wherever a renderer must be named.
    const NAME: &'static str;

    /// The device the backend renders with, the one a target texture must be allocated on.
    fn device(&self) -> &wgpu::Device;

    /// The queue the backend submits its rendering work to.
    fn queue(&self) -> &wgpu::Queue;

    /// Composes `scene` into `target`, a view of a `width` by `height` texture. `scene`'s geometry is
    /// in logical pixels scaled by `scale` to physical pixels.
    fn render(
        &mut self,
        scene: &Scene,
        target: &wgpu::TextureView,
        width: u32,
        height: u32,
        scale: f64,
    );
}
