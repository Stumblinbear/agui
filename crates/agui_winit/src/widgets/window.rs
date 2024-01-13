use std::{marker::PhantomData, pin::pin, thread};

use agui_core::{
    unit::Size,
    widget::{IntoWidget, Widget},
};
use agui_elements::stateful::{
    ContextWidgetStateMut, StatefulBuildContext, StatefulWidget, WidgetState,
};
use agui_inheritance::ContextInherited;
use agui_macros::{build, StatefulWidget};
use agui_primitives::sized_box::SizedBox;
use agui_renderer::RenderWindow;
use futures::prelude::stream::StreamExt;
use winit::{event::WindowEvent, window::WindowBuilder};

use crate::{handle::WinitWindowHandle, WinitWindowManager};
use crate::{widgets::window_layout::WinitWindowLayout, CurrentWindow};

#[derive(StatefulWidget)]
pub struct WinitWindow<WindowFn, RendererFn, Renderer>
where
    WindowFn: Fn() -> WindowBuilder + Send + Sync + Clone + 'static,
    RendererFn: Fn(WinitWindowHandle) -> Renderer + Send + Sync + Clone + 'static,
    Renderer: RenderWindow + 'static,
{
    pub window: WindowFn,

    pub renderer: RendererFn,

    pub child: Widget,
}

impl<WindowFn, RendererFn, Renderer> StatefulWidget for WinitWindow<WindowFn, RendererFn, Renderer>
where
    WindowFn: Fn() -> WindowBuilder + Send + Sync + Clone + 'static,
    RendererFn: Fn(WinitWindowHandle) -> Renderer + Send + Sync + Clone + 'static,
    Renderer: RenderWindow + 'static,
{
    type State = WinitWindowState<WindowFn, RendererFn, Renderer>;

    fn create_state(&self) -> Self::State {
        WinitWindowState {
            phantom: PhantomData,

            window: None,
            window_size: Size::ZERO,
        }
    }
}

pub struct WinitWindowState<WindowFn, RendererFn, Renderer> {
    phantom: PhantomData<(WindowFn, RendererFn, Renderer)>,

    window: Option<WinitWindowHandle>,
    window_size: Size,
}

impl<WindowFn, RendererFn, Renderer> WidgetState
    for WinitWindowState<WindowFn, RendererFn, Renderer>
where
    WindowFn: Fn() -> WindowBuilder + Send + Sync + Clone + 'static,
    RendererFn: Fn(WinitWindowHandle) -> Renderer + Send + Sync + Clone + 'static,
    Renderer: RenderWindow + 'static,
{
    type Widget = WinitWindow<WindowFn, RendererFn, Renderer>;

    fn init_state(&mut self, ctx: &mut StatefulBuildContext<Self>) {
        // let mouse_input_event_cb = ctx.callback(
        //     |_ctx, (device_id, state, button): (DeviceId, ElementState, MouseButton)| {
        //         // println!(
        //         //     "Mouse input event: {:?} {:?} {:?}",
        //         //     device_id, state, button
        //         // );
        //     },
        // );

        let resize_event_cb = ctx.callback(|ctx, new_window_size: Size| {
            if ctx.state.window_size == new_window_size {
                return;
            }

            ctx.set_state(move |state| {
                state.window_size = new_window_size;
            });
        });

        let Some(window_manager) = ctx.find_inherited_widget::<WinitWindowManager>() else {
            return tracing::error!("windowing plugin not found");
        };

        let on_window_created = ctx.callback(move |ctx, window: WinitWindowHandle| {
            // let mouse_input_event_cb = mouse_input_event_cb.clone();
            // let resize_event_cb = resize_event_cb.clone();

            thread::spawn({
                let window = window.clone();
                let resize_event_cb = resize_event_cb.clone();

                || {
                    futures::executor::block_on(async move {
                        let mut events = pin!(window.events());

                        while let Some(event) = events.next().await {
                            if let WindowEvent::Resized(size) = event.as_ref() {
                                resize_event_cb
                                    .call(Size::new(size.width as f32, size.height as f32));
                            }
                        }
                    });
                }
            });

            ctx.set_state(|state| {
                state.window.replace(window);

                // state.event_listener = Some(window.events().add_listener(
                //     move |WinitWindowEvent(ref event)| {
                //         if let WindowEvent::MouseInput {
                //             device_id,
                //             state,
                //             button,
                //             ..
                //         } = event
                //         {
                //             mouse_input_event_cb.call((*device_id, *state, *button));
                //         } else if let WindowEvent::Resized(size) = event {
                //             resize_event_cb.call(Size::new(size.width as f32, size.height as f32));
                //         }
                //     },
                // ));

                // let size = window.inner_size();

                // state
                //     .window_size
                //     .replace(Size::new(size.width as f32, size.height as f32));
            });
        });

        if let Err(err) = window_manager.create_window(
            ctx.widget.window.clone(),
            ctx.widget.renderer.clone(),
            on_window_created,
        ) {
            tracing::error!("failed to create window: {:?}", err);
        }
    }

    fn updated(&mut self, _: &mut StatefulBuildContext<Self>, _: &Self::Widget) {
        // TODO: recreate the window?
    }

    fn build(&mut self, ctx: &mut StatefulBuildContext<Self>) -> Widget {
        if let Some(window) = &self.window {
            build! {
                <WinitWindowLayout> {
                    size: self.window_size,

                    child: <CurrentWindow> {
                        handle: window.clone(),

                        child: ctx.widget.child.clone()
                    }
                }
            }
        } else {
            SizedBox::shrink().into_widget()
        }
    }
}
