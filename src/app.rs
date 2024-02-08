use agui_core::widget::IntoWidget;
use agui_executor::EngineExecutor;
use agui_macros::build;
use agui_sync::notify;
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

    let controller = winit_app.create_controller();

    let shutdown_tx = notify::Flag::new();
    let mut shutdown_rx = shutdown_tx.subscribe();

    let handle = std::thread::Builder::new()
        .name("agui executor".to_string())
        .spawn(move || {
            let root = build! {
                <WinitWindowManager> {
                    controller: controller,

                    child: (func)().into_widget()
                }
            };

            // #[cfg(not(feature = "multi-threaded"))]
            let executor =
                agui_executor::LocalEngineExecutor::with_root(root).expect("failed to build tree");

            // #[cfg(feature = "multi-threaded")]
            // let executor = agui_executor::ThreadedEngineExecutor::default().with_root(root);

            executor.run_until(shutdown_rx.wait());
        })
        // TODO: Fall back to single threaded?
        .expect("failed to spawn thread");

    winit_app.run();

    shutdown_tx.notify();

    handle.join().expect("app thread panicked");

    Ok(())
}
