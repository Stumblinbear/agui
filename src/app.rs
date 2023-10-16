use std::{sync::mpsc, time::Instant};

use agui_core::{engine::Engine, widget::IntoWidget};
use agui_inheritance::InheritancePlugin;
use agui_renderer::{DefaultRenderer, RenderViewPlugin};
#[cfg(feature = "vello")]
use agui_vello::VelloPlugin;
#[cfg(feature = "winit")]
use agui_winit::WinitPlugin;
use winit::window::Window;
use winit::{
    event::Event as WinitEvent,
    event_loop::{ControlFlow, EventLoopBuilder},
};

#[cfg(not(all(feature = "vello", feature = "winit")))]
compile_error!("app feature requires both winit and vello to be enabled");

pub fn run_app(root: impl IntoWidget) -> Result<(), Box<dyn std::error::Error>> {
    let (update_notifier_tx, update_notifier_rx) = mpsc::channel();

    let vello_plugin = VelloPlugin::new();

    let renderer = vello_plugin.create_renderer::<winit::window::Window>()?;

    // renderer.get_fonts().lock().add_font(
    //     FontRef::new(include_bytes!("../examples/fonts/DejaVuSans.ttf"))
    //         .expect("failed to load font"),
    // );

    let engine = Engine::builder()
        .with_notifier(update_notifier_tx.clone())
        .add_plugin(InheritancePlugin::default())
        .add_plugin(RenderViewPlugin::default());

    #[cfg(feature = "winit")]
    let engine = { engine.add_plugin(WinitPlugin::new(update_notifier_tx.clone())) };

    #[cfg(feature = "vello")]
    let engine = { engine.add_plugin(vello_plugin) };

    let test: DefaultRenderer<Window> = DefaultRenderer::builder()
        .renderer(renderer)
        .child(root.into_widget())
        .build();

    let mut engine = engine.with_root(test).build();

    let event_loop = EventLoopBuilder::<()>::with_user_event().build();

    let event_loop_proxy = event_loop.create_proxy();

    // Wake up the event loop when the engine has changes to process
    std::thread::spawn(move || {
        let _ = event_loop_proxy.send_event(());

        while update_notifier_rx.recv().is_ok() {
            let _ = event_loop_proxy.send_event(());
        }
    });

    event_loop.run(move |event, window_target, control_flow| {
        *control_flow = ControlFlow::Wait;

        let mut requires_update = false;

        let winit_plugin = engine
            .get_plugins_mut()
            .get_mut::<WinitPlugin>()
            .expect("no winit plugin");

        winit_plugin.process_queue(window_target, control_flow);

        match event {
            WinitEvent::WindowEvent { event, window_id } => {
                winit_plugin.handle_event(window_target, window_id, event, control_flow);
            }

            WinitEvent::RedrawRequested(window_id) => {
                winit_plugin.render(window_id);

                requires_update = true;
            }

            WinitEvent::UserEvent(event) => {
                requires_update = true;
            }

            _ => (),
        }

        if requires_update {
            let now = Instant::now();

            engine.update();

            tracing::info!("updated in: {:?}", Instant::now().duration_since(now));

            // // TODO: limit redraws only to the windows that show visual changes
            // windows.iter_mut().for_each(|(window_id, window)| {
            //     window.request_redraw();
            // });
        }
    });
}
