use std::{sync::mpsc, time::Instant};

use agui_core::{engine::Engine, unit::Offset, widget::IntoWidget};
use agui_inheritance::InheritancePlugin;
use agui_macros::build;
use agui_renderer::{RenderViewId, RenderViewPlugin, Renderer};
#[cfg(feature = "vello")]
use agui_vello;
use agui_vello::{VelloBinding, VelloRenderer};
use agui_winit::WinitBindingEvent;
#[cfg(feature = "winit")]
use agui_winit::{WinitBinding, WinitWindowHandle};
use rustc_hash::FxHashMap;
use vello::glyph::fello::raw::FontRef;
use winit::{
    dpi::PhysicalPosition,
    event::{Event as WinitEvent, MouseScrollDelta, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::WindowId,
};

#[cfg(not(all(feature = "vello", feature = "winit")))]
compile_error!("app feature requires both winit and vello to be enabled");

pub fn run_app(root: impl IntoWidget) {
    let (update_notifier_tx, update_notifier_rx) = mpsc::channel();

    let mut renderer = VelloRenderer::new().expect("failed to init renderer");

    renderer.get_fonts().lock().add_font(
        FontRef::new(include_bytes!("../examples/fonts/DejaVuSans.ttf"))
            .expect("failed to load font"),
    );

    let (winit_binding_tx, winit_binding_rx) = mpsc::channel();

    let mut engine = Engine::builder()
        .with_notifier(update_notifier_tx.clone())
        .add_plugin(InheritancePlugin::default())
        .add_plugin(RenderViewPlugin::default())
        .with_root(build! {
            <WinitBinding> {
                tx: winit_binding_tx,

                child: <VelloBinding> {
                    fonts: renderer.get_fonts().clone(),

                    child: root.into_widget(),
                },
            }
        })
        .build();

    let mut windows = FxHashMap::<WindowId, WinitWindowHandle>::default();
    let mut render_view_window = FxHashMap::<RenderViewId, WindowId>::default();
    let mut window_render_view = FxHashMap::<WindowId, RenderViewId>::default();

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

        while let Ok(binding_event) = winit_binding_rx.try_recv() {
            match binding_event {
                WinitBindingEvent::CreateWindow(element_id, builder, callback) => {
                    tracing::debug!("creating window for element: {:?}", element_id);

                    let window = WinitWindowHandle::new(builder.build(window_target).unwrap());

                    let size = window.inner_size();

                    let window_id = window.id();

                    let render_view_plugin = engine
                        .get_plugins()
                        .get::<RenderViewPlugin>()
                        .expect("no render view plugin");

                    let render_view_id = render_view_plugin
                        .get_view(element_id)
                        .expect("no render view");

                    renderer.create_view(
                        &engine,
                        render_view_id,
                        &*window,
                        size.width,
                        size.height,
                    );

                    windows.insert(window_id, window.clone());
                    render_view_window.insert(render_view_id, window_id);
                    window_render_view.insert(window_id, render_view_id);

                    callback.call(window);
                }

                WinitBindingEvent::CloseWindow(element_id) => {}
            }
        }

        let mut requires_update = false;

        match event {
            WinitEvent::WindowEvent { event, window_id } => {
                // Since this event has a mutable reference which we can influence, we need to handle it a bit differently.
                if let WindowEvent::ScaleFactorChanged {
                    scale_factor: _,
                    new_inner_size: _,
                } = event
                {
                } else if let Some(event) = event.to_static() {
                    // Emit the event before handling it to give the user a chance to respond to it before
                    // making changes that may cause further events to be emitted. This ensures they arrive
                    // in a sensible order.
                    if let Some(window) = windows.get_mut(&window_id) {
                        window.events().emit(&event);
                    }

                    match event {
                        WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,

                        WindowEvent::Destroyed => {
                            windows.remove(&window_id);
                            render_view_window
                                .remove(&window_render_view.remove(&window_id).unwrap());
                        }

                        WindowEvent::Resized(size) => {
                            if let Some(window) = windows.get_mut(&window_id) {
                                renderer.resize(
                                    &engine,
                                    *window_render_view.get(&window.id()).unwrap(),
                                    size.width,
                                    size.height,
                                );

                                window.request_redraw();
                            } else {
                                tracing::error!(
                                    "a redraw was requested, but the window doesn't exist: {:?}",
                                    window_id
                                );
                            }
                        }

                        WindowEvent::Moved(pos) => {
                            let window_pos = Offset {
                                x: pos.x as f32,
                                y: pos.y as f32,
                            };

                            // self.manager.fire_event(window_pos);
                            // self.manager
                            //     .set_global::<WindowPosition, _>(move |state| *state = window_pos);
                        }

                        WindowEvent::Focused(focused) => {
                            let window_focused = focused;

                            // self.manager.fire_event(window_focused);
                            // self.manager
                            //     .set_global::<WindowFocus, _>(move |state| *state = window_focused);
                        }

                        WindowEvent::CursorMoved { position: pos, .. } => {
                            let mouse_pos = Some(Offset {
                                x: pos.x as f32,
                                y: pos.y as f32,
                            });

                            // self.manager.fire_event(mouse_pos);
                            // self.manager
                            //     .set_global::<MousePos, _>(move |state| *state = mouse_pos);
                            // self.manager
                            //     .set_global::<Mouse, _>(move |state| state.pos = mouse_pos);
                        }

                        WindowEvent::CursorLeft { .. } => {
                            // let mouse_pos = None;

                            // self.manager.fire_event(mouse_pos);
                            // self.manager
                            //     .set_global::<MousePos, _>(move |state| *state = mouse_pos);
                            // self.manager
                            //     .set_global::<Mouse, _>(move |state| state.pos = mouse_pos);
                        }

                        WindowEvent::MouseWheel { delta, .. } => {
                            let scroll = match delta {
                                MouseScrollDelta::LineDelta(x, y) => Offset { x, y },

                                MouseScrollDelta::PixelDelta(PhysicalPosition { x, y }) => Offset {
                                    x: x as f32,
                                    y: y as f32,
                                },
                            };

                            // self.manager.fire_event(scroll);
                            // self.manager
                            //     .set_global::<Scroll, _>(move |state| *state = scroll);
                        }

                        WindowEvent::MouseInput {
                            button,
                            state: value,
                            ..
                        } => {
                            // let button_state = match value {
                            //     ElementState::Pressed => MouseButtonState::Pressed,
                            //     ElementState::Released => MouseButtonState::Released,
                            // };

                            // self.manager.fire_event(match button {
                            //     winit::event::MouseButton::Left => MouseButton::Left(button_state),
                            //     winit::event::MouseButton::Right => MouseButton::Right(button_state),
                            //     winit::event::MouseButton::Middle => MouseButton::Middle(button_state),
                            //     winit::event::MouseButton::Other(i) => {
                            //         MouseButton::Other(i, button_state)
                            //     }
                            // });

                            // self.manager
                            //     .set_global::<MouseButtons, _>(move |state| match button {
                            //         winit::event::MouseButton::Left => state.left = button_state,
                            //         winit::event::MouseButton::Right => state.right = button_state,
                            //         winit::event::MouseButton::Middle => state.middle = button_state,
                            //         winit::event::MouseButton::Other(i) => {
                            //             state.other.insert(i, button_state);
                            //         }
                            //     });

                            // self.manager
                            //     .set_global::<Mouse, _>(move |state| match button {
                            //         winit::event::MouseButton::Left => state.button.left = button_state,
                            //         winit::event::MouseButton::Right => {
                            //             state.button.right = button_state
                            //         }
                            //         winit::event::MouseButton::Middle => {
                            //             state.button.middle = button_state
                            //         }
                            //         winit::event::MouseButton::Other(i) => {
                            //             state.button.other.insert(i, button_state);
                            //         }
                            //     });
                        }

                        WindowEvent::ReceivedCharacter(c) => {
                            // self.manager.fire_event(KeyboardCharacter(c));
                        }

                        WindowEvent::KeyboardInput { input, .. } => {
                            if let Some(key) = input.virtual_keycode {
                                // let key: KeyCode = unsafe { std::mem::transmute(key as u32) };

                                // let key_state = match input.state {
                                //     ElementState::Pressed => KeyState::Pressed,
                                //     ElementState::Released => KeyState::Released,
                                // };

                                // self.manager.fire_event(KeyboardInput(key, key_state));

                                // self.manager.set_global::<Keyboard, _>(move |state| {
                                //     state.keys.insert(key, key_state);
                                // });
                            }
                        }

                        WindowEvent::ModifiersChanged(modifiers) => {
                            // self.manager.set_global::<Keyboard, _>(move |state| {
                            //     state.modifiers = unsafe { mem::transmute(modifiers) };
                            // });
                        }

                        _ => {}
                    }
                }
            }

            WinitEvent::RedrawRequested(window_id) => {
                if let Some(render_view_id) = window_render_view.get(&window_id) {
                    renderer.render(*render_view_id);
                } else {
                    tracing::error!(
                        "a redraw was requested, but the window doesn't exist: {:?}",
                        window_id
                    );
                }
            }

            WinitEvent::UserEvent(_) => {
                requires_update = true;
            }

            _ => (),
        }

        if requires_update {
            let now = Instant::now();

            let events = engine.update();

            if !events.is_empty() {
                tracing::info!("updated in: {:?}", Instant::now().duration_since(now));

                // TODO: limit redraws only to the windows that show visual changes
                windows.iter_mut().for_each(|(window_id, window)| {
                    renderer.redraw(
                        &engine,
                        *window_render_view.get(window_id).unwrap(),
                        &events,
                    );

                    window.request_redraw();
                });
            } else {
                tracing::warn!("an update event was received, but no changes were made");
            }
        }
    });
}
