use std::{
    error::Error,
    fmt,
    sync::{mpsc, Arc},
};

use agui_core::{
    callback::Callback,
    element::ElementId,
    listenable::EventEmitter,
    plugin::Plugin,
    unit::{Offset, Size},
};
use agui_renderer::{RenderViewId, Renderer};
use rustc_hash::FxHashMap;
use winit::{
    dpi::PhysicalPosition,
    event::{MouseScrollDelta, WindowEvent},
    event_loop::{ControlFlow, EventLoopWindowTarget},
    window::{WindowBuilder, WindowId},
};

use crate::{controller::WinitSendError, WinitWindowEvent, WinitWindowHandle};

pub struct WinitPlugin {
    windows: FxHashMap<WindowId, WinitWindowHandle>,
    window_renderer: FxHashMap<WindowId, Arc<dyn ViewRenderer>>,

    event_notifier_tx: mpsc::Sender<()>,

    action_queue_tx: mpsc::Sender<WinitBindingAction>,
    action_queue_rx: mpsc::Receiver<WinitBindingAction>,
}

impl WinitPlugin {
    pub fn new(event_notifier_tx: mpsc::Sender<()>) -> Self {
        let (action_queue_tx, action_queue_rx) = mpsc::channel();

        Self {
            windows: FxHashMap::default(),
            window_renderer: FxHashMap::default(),

            event_notifier_tx,

            action_queue_tx,
            action_queue_rx,
        }
    }
}

impl Plugin for WinitPlugin {}

impl WinitPlugin {
    pub(crate) fn bind_renderer(
        &mut self,
        window_id: WindowId,
        render_view_id: RenderViewId,
        renderer: Arc<dyn Renderer<Target = winit::window::Window>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let window = self
            .windows
            .get(&window_id)
            .expect("cannot bind a renderer to a window that does not exist");

        tracing::debug!("binding {:?} to {:?}", render_view_id, window_id);

        let size = window.inner_size();

        let view_renderer = renderer.bind(
            render_view_id,
            window,
            Size::new(size.width as f32, size.height as f32),
        )?;

        self.window_renderer.insert(window_id, view_renderer);

        Ok(())
    }

    pub(crate) fn unbind_renderer(&mut self, window_id: &WindowId) {
        self.window_renderer
            .remove(window_id)
            .expect("cannot unbind a renderer from a window that does not exist");
    }

    pub fn process_queue(
        &mut self,
        window_target: &EventLoopWindowTarget<()>,
        control_flow: &mut ControlFlow,
    ) {
        while let Ok(action) = self.action_queue_rx.try_recv() {
            match action {
                WinitBindingAction::CreateWindow(element_id, builder, callback) => {
                    tracing::debug!("creating window for element: {:?}", element_id);

                    let window = WinitWindowHandle {
                        handle: Arc::new(
                            builder
                                .build(window_target)
                                .expect("failed to create window"),
                        ),
                        event_emitter: EventEmitter::default(),
                        action_queue_tx: self.action_queue_tx.clone(),
                    };

                    let window_id = window.id();

                    self.windows.insert(window_id, window.clone());

                    callback.call(window);
                }

                WinitBindingAction::CloseWindow(window_id) => {
                    self.windows.remove(&window_id);
                }
            }
        }
    }

    pub fn render(&mut self, window_id: WindowId) {
        if let Some(view_renderer) = self.window_renderer.get_mut(&window_id) {
            view_renderer.render();
        } else {
            tracing::error!(
                "cannot render to {:?} because no renderer is bound",
                window_id
            );
        }
    }

    pub fn handle_event(
        &mut self,
        window_target: &EventLoopWindowTarget<()>,
        window_id: WindowId,
        event: WindowEvent,
        control_flow: &mut ControlFlow,
    ) {
        // Since this event has a mutable reference which we can influence, we need to handle it a bit differently.
        if let WindowEvent::ScaleFactorChanged {
            scale_factor: _,
            new_inner_size: _,
        } = event
        {
        } else if let Some(event) = event.to_static() {
            match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,

                WindowEvent::Destroyed => {
                    self.windows.remove(&window_id);
                }

                WindowEvent::Resized(size) => {
                    if let Some(window) = self.windows.get(&window_id) {
                        // if let Some(view_renderer) = self.window_renderer.get_mut(&window_id) {
                        //     view_renderer.resize(Size::new(size.width as f32, size.height as f32));
                        // }

                        window.request_redraw();
                    } else {
                        tracing::error!(
                            "a resize event was received for {:?}, but it does not exist",
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

            if let Some(window) = self.windows.get_mut(&window_id) {
                window.events().emit(&WinitWindowEvent(event));
            }
        }
    }
}
