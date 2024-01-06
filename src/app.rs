use std::{sync::Arc, thread, time::Instant};

use agui_core::{engine::Engine, widget::IntoWidget};
use agui_inheritance::InheritancePlugin;
use agui_macros::build;
use agui_vello::VelloViewBinding;
use agui_winit::{WinitApp, WinitWindowController, WinitWindowManager};
use parking_lot::{Condvar, Mutex};

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

    let controller = WinitWindowController::new(winit_app.event_loop.create_proxy());

    thread::spawn(move || {
        let root = build! {
            <WinitWindowManager> {
                controller: controller,

                child: <agui_vello::VelloView> {
                    binding: || VelloViewBinding,

                    child: (func)().into_widget(),
                }
            }
        };

        let notifier = Arc::new((Mutex::new(false), Condvar::new()));

        let mut engine = Engine::builder()
            .with_notifier({
                let notifier = Arc::clone(&notifier);

                move || {
                    let (mutex, cond) = &*notifier;
                    let mut guard = mutex.lock();
                    *guard = true;
                    cond.notify_one();
                }
            })
            .add_plugin(InheritancePlugin::default())
            .with_root(root)
            .build();

        // TODO: add a way to actually stop the engine
        loop {
            let start = Instant::now();

            engine.update();

            tracing::debug!(elapsed = ?start.elapsed(), "update complete");

            let (mutex, cond) = &*notifier;
            let mut guard = mutex.lock();
            while !*guard {
                cond.wait(&mut guard);
            }
            *guard = false;
        }
    });

    winit_app.run();

    Ok(())
}
