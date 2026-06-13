//! winit windowing for agui: an event loop, an OS window, a wgpu surface, and the per-frame pipeline
//! drive behind one [`run`] entry point, so an app is just a widget tree.

mod app;
mod driver;

use agui_core::prelude::{element::*, render_object::*};
use winit::event_loop::EventLoop;

pub use agui_core::scheduling::Vsync;

use crate::{app::App, driver::WakeUp, driver::WindowDriver};

/// How a window is created: its title and its initial inner size in logical pixels.
pub struct WindowOptions {
    pub title: String,
    pub width: u32,
    pub height: u32,
}

/// Runs a window presenting `build`'s widget tree, returning when the window closes.
///
/// `build` receives the [`Vsync`] the runtime ticks each frame, so an animating widget can drive
/// itself from it. The call blocks for the program's lifetime: it owns the event loop.
pub fn run_app<V>(options: WindowOptions, build: impl FnOnce(Vsync) -> V)
where
    V: Widget + 'static,
    V::Render: RenderBox,
{
    let vsync = Vsync::new();
    let widget = build(vsync.clone());

    let event_loop = EventLoop::<WakeUp>::with_user_event().build().unwrap();
    let driver = WindowDriver::new(widget, vsync, event_loop.create_proxy());

    let mut app = App::new(options, driver);
    event_loop.run_app(&mut app).unwrap();
}
