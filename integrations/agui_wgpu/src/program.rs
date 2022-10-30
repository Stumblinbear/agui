use std::{
    mem,
    ops::{Deref, DerefMut},
    time::Instant,
};

use agui::{
    manager::WidgetManager,
    unit::{Point, Size},
    widgets::state::{
        keyboard::{KeyCode, KeyState},
        mouse::{MouseButtonState, MousePos, Scroll},
        window::{WindowFocus, WindowPosition, WindowSize},
    },
};
use wgpu::{CompositeAlphaMode, PresentMode, Surface, SurfaceConfiguration, TextureUsages};
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, Event as WinitEvent, MouseScrollDelta, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
    window::{Window, WindowBuilder},
};

use crate::{handle::RenderHandle, manager::RenderManager};

pub struct AguiProgram {
    event_loop: EventLoop<()>,

    window: Window,

    handle: RenderHandle,

    surface_config: SurfaceConfiguration,
    surface: Surface,

    manager: WidgetManager,
    renderer: RenderManager,
}

impl Deref for AguiProgram {
    type Target = WidgetManager;

    fn deref(&self) -> &Self::Target {
        &self.manager
    }
}

impl DerefMut for AguiProgram {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.manager
    }
}

impl AguiProgram {
    pub fn new(title: &str, size: Size) -> Self {
        let event_loop = EventLoopBuilder::<()>::with_user_event().build();

        let window = WindowBuilder::new()
            .with_title(title)
            .with_decorations(true)
            .with_resizable(true)
            .with_transparent(false)
            .with_inner_size(winit::dpi::PhysicalSize {
                width: size.width,
                height: size.height,
            })
            .build(&event_loop)
            .unwrap();

        let instance = wgpu::Instance::new(wgpu::Backends::all());

        let surface = unsafe { instance.create_surface(&window) };

        let adapter =
            futures::executor::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            }))
            .unwrap();

        let (device, queue) = futures::executor::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::default(),
                limits: wgpu::Limits::default(),
                label: None,
            },
            None,
        ))
        .unwrap();

        let size = window.inner_size();
        let surface_format = surface.get_supported_formats(&adapter)[0];
        let surface_config = SurfaceConfiguration {
            alpha_mode: CompositeAlphaMode::Opaque,
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width as u32,
            height: size.height as u32,
            present_mode: PresentMode::Fifo,
        };

        surface.configure(&device, &surface_config);

        let handle = RenderHandle { device, queue };

        let renderer = RenderManager::new(
            &handle,
            Size {
                width: size.width as f32,
                height: size.height as f32,
            },
        );

        Self {
            event_loop,

            window,

            handle,

            surface_config,
            surface,

            manager: WidgetManager::new(),
            renderer,
        }
    }

    pub fn run(mut self) -> ! {
        self.event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;

            match event {
                WinitEvent::RedrawRequested(..) => {
                    let output_frame = match self.surface.get_current_texture() {
                        Ok(frame) => frame,

                        Err(wgpu::SurfaceError::Outdated) => {
                            // This error occurs when the app is minimized on Windows.
                            // Silently return here to prevent spamming the console with:
                            // "The underlying surface has changed, and therefore the swap chain must be updated"
                            return;
                        }

                        Err(e) => {
                            eprintln!("Dropped frame with error: {}", e);
                            return;
                        }
                    };

                    let output_view = output_frame
                        .texture
                        .create_view(&wgpu::TextureViewDescriptor::default());

                    self.renderer.render(&self.handle, &output_view);

                    output_frame.present();
                }

                WinitEvent::MainEventsCleared => {
                    self.window.request_redraw();
                }

                WinitEvent::WindowEvent { event, .. } => match event {
                    WindowEvent::Resized(size) => {
                        // Resize with 0 width and height is used by winit to signal a minimize event on Windows.
                        // See: https://github.com/rust-windowing/winit/issues/208
                        // This solves an issue where the app would panic when minimizing on Windows.
                        if size.width > 0 && size.height > 0 {
                            self.surface_config.width = size.width;
                            self.surface_config.height = size.height;

                            self.surface
                                .configure(&self.handle.device, &self.surface_config);

                            self.renderer.resize(
                                &self.handle,
                                Size {
                                    width: size.width as f32,
                                    height: size.height as f32,
                                },
                            );

                            let window_size = WindowSize {
                                width: size.width as f32,
                                height: size.height as f32,
                            };

                            // self.manager.fire_event(window_size);
                            // self.manager
                            //     .set_global::<WindowSize, _>(move |state| *state = window_size);
                        }
                    }

                    WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit;
                    }

                    WindowEvent::Moved(pos) => {
                        let window_pos = WindowPosition {
                            x: pos.x as f32,
                            y: pos.y as f32,
                        };

                        // self.manager.fire_event(window_pos);
                        // self.manager
                        //     .set_global::<WindowPosition, _>(move |state| *state = window_pos);
                    }

                    WindowEvent::Focused(focused) => {
                        let window_focused = WindowFocus(focused);

                        // self.manager.fire_event(window_focused);
                        // self.manager
                        //     .set_global::<WindowFocus, _>(move |state| *state = window_focused);
                    }

                    WindowEvent::CursorMoved { position: pos, .. } => {
                        let mouse_pos = MousePos(Some(Point {
                            x: pos.x as f32,
                            y: pos.y as f32,
                        }));

                        // self.manager.fire_event(mouse_pos);
                        // self.manager
                        //     .set_global::<MousePos, _>(move |state| *state = mouse_pos);
                        // self.manager
                        //     .set_global::<Mouse, _>(move |state| state.pos = mouse_pos);
                    }

                    WindowEvent::CursorLeft { .. } => {
                        let mouse_pos = MousePos(None);

                        // self.manager.fire_event(mouse_pos);
                        // self.manager
                        //     .set_global::<MousePos, _>(move |state| *state = mouse_pos);
                        // self.manager
                        //     .set_global::<Mouse, _>(move |state| state.pos = mouse_pos);
                    }

                    WindowEvent::MouseWheel { delta, .. } => {
                        let scroll = Scroll(match delta {
                            MouseScrollDelta::LineDelta(x, y) => Point { x, y },

                            MouseScrollDelta::PixelDelta(PhysicalPosition { x, y }) => Point {
                                x: x as f32,
                                y: y as f32,
                            },
                        });

                        // self.manager.fire_event(scroll);
                        // self.manager
                        //     .set_global::<Scroll, _>(move |state| *state = scroll);
                    }

                    WindowEvent::MouseInput {
                        button,
                        state: value,
                        ..
                    } => {
                        let button_state = match value {
                            ElementState::Pressed => MouseButtonState::Pressed,
                            ElementState::Released => MouseButtonState::Released,
                        };

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
                            let key: KeyCode = unsafe { mem::transmute(key as u32) };

                            let key_state = match input.state {
                                ElementState::Pressed => KeyState::Pressed,
                                ElementState::Released => KeyState::Released,
                            };

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
                },

                _ => (),
            }

            let now = Instant::now();

            let events = self.manager.update();

            if !events.is_empty() {
                tracing::info!("updated in: {:?}", Instant::now().duration_since(now));

                self.renderer.redraw(&self.handle, &self.manager, &events);

                self.window.request_redraw();
            }
        });
    }
}
