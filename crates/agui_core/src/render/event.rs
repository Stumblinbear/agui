/// Used to indicate a change to the render tree.
#[derive(Debug, Copy, Clone)]
#[non_exhaustive]
pub enum RenderEvent {
    /// A render node has changed in the layout.
    Drawn { render_id: RenderId },

    /// A render node has been destroyed.
    Destroyed { render_id: RenderId },
}
