use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};

pub trait RenderWindow {
    type Target: HasRawWindowHandle + HasRawDisplayHandle;

    fn render(&self);
}
