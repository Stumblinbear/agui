//! The seam between a windowing backend and a renderer: a windowing crate drives any
//! [`WindowRenderer`], and a rendering backend implements one, so neither depends on the other.

use std::sync::Arc;

use agui_core::paint::compositing::CompositedFrame;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

/// Presents composed frames to a single window's surface.
pub trait WindowRenderer {
    /// Builds the surface for `window`, sized `width` by `height` in physical pixels.
    fn attach<W>(&mut self, window: Arc<W>, width: u32, height: u32)
    where
        W: HasWindowHandle + HasDisplayHandle + Send + Sync + 'static;

    /// Resizes the surface to `width` by `height` physical pixels.
    fn resize(&mut self, width: u32, height: u32);

    /// Presents `frame`, whose geometry is in logical pixels, scaled by `scale_factor` to physical
    /// pixels.
    fn present(&mut self, frame: &CompositedFrame, scale_factor: f64);
}
