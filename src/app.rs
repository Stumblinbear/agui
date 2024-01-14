use std::{thread, time::Instant};

use agui_core::{engine::Engine, widget::IntoWidget};
use agui_macros::build;
use agui_winit::{WinitApp, WinitWindowManager};

#[cfg(not(all(feature = "vello", feature = "winit")))]
compile_error!("app feature requires both winit and vello to be enabled");

pub fn run_app<F, W>(func: F) -> Result<(), Box<dyn std::error::Error>>
where
    F: FnOnce() -> W + Send + 'static,
    W: IntoWidget,
{
    // renderer.get_fonts().lock().add_font(
    //     FontRef::new(include_bytes!("../examples/fonts/DejaVuSans.ttf"))
    //         .expect("failed to load font"),
    // );

    let winit_app = WinitApp::default();

    let event_loop = winit_app.event_loop.create_proxy();

    thread::spawn(move || {
        let root = build! {
            <WinitWindowManager> {
                event_loop: event_loop,

                child: (func)().into_widget()
            }
        };

        let mut engine = Engine::with_root(root);

        // TODO: add a way to actually stop the engine
        loop {
            let start = Instant::now();

            engine.update();

            tracing::debug!(elapsed = ?start.elapsed(), "update complete");

            engine.wait_for_update();
        }
    });

    winit_app.run();

    Ok(())
}
