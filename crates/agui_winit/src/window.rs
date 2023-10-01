use std::ops::Deref;

use agui_core::{
    listeners::EventEmitterHandle,
    unit::{Constraints, IntrinsicDimension, Size},
    widget::{
        view::RenderView, BuildContext, ContextInheritedMut, ContextWidget, ContextWidgetStateMut,
        InheritedWidget, IntoWidget, IntrinsicSizeContext, LayoutContext, StatefulBuildContext,
        StatefulWidget, Widget, WidgetBuild, WidgetLayout, WidgetState,
    },
};
use agui_macros::{build, InheritedWidget, LayoutWidget, StatefulWidget, StatelessWidget};
use agui_primitives::sized_box::SizedBox;
use winit::{
    event::{DeviceId, ElementState, MouseButton, WindowEvent},
    window::WindowBuilder,
};

use crate::{binding::WinitBinding, handle::WinitWindowHandle};

#[derive(StatelessWidget)]
pub struct Window {
    pub window: WindowBuilder,

    pub child: Widget,
}

impl WidgetBuild for Window {
    fn build(&self, _: &mut BuildContext<Self>) -> Widget {
        // Windows must be created within their own render context
        RenderView {
            child: build! {
                <WinitWindow> {
                    window: self.window.clone(),

                    child: self.child.clone(),
                }
            },
        }
        .into_widget()
    }
}

#[derive(StatefulWidget)]
struct WinitWindow {
    window: WindowBuilder,

    child: Widget,
}

impl StatefulWidget for WinitWindow {
    type State = WinitWindowState;

    fn create_state(&self) -> Self::State {
        WinitWindowState::default()
    }
}

#[derive(Default)]
struct WinitWindowState {
    window: Option<WinitWindowHandle>,

    event_listener: Option<EventEmitterHandle<WindowEvent<'static>>>,
}

impl WidgetState for WinitWindowState {
    type Widget = WinitWindow;

    fn init_state(&mut self, ctx: &mut StatefulBuildContext<Self>) {
        let Some(binding) = ctx.depend_on_inherited_widget::<WinitBinding>() else {
            return tracing::error!("Windowing controller not found in the widget tree");
        };

        let mouse_input_event_cb = ctx.callback(
            |ctx, (device_id, state, button): (DeviceId, ElementState, MouseButton)| {
                println!(
                    "Mouse input event: {:?} {:?} {:?}",
                    device_id, state, button
                );

                ctx.get_element_id();
            },
        );

        // I don't like cloning the window, here.
        binding.create_window(
            ctx.get_element_id(),
            ctx.widget.window.clone(),
            ctx.callback(move |ctx, window: WinitWindowHandle| {
                let mouse_input_event_cb = mouse_input_event_cb.clone();

                ctx.set_state(|state| {
                    state.event_listener = Some(window.events().add_listener(move |event| {
                        if let WindowEvent::MouseInput {
                            device_id,
                            state,
                            button,
                            ..
                        } = event
                        {
                            mouse_input_event_cb.call((*device_id, *state, *button));
                        }
                    }));

                    state.window.replace(window);
                });
            }),
        );
    }

    fn build(&mut self, ctx: &mut StatefulBuildContext<Self>) -> Widget {
        if let Some(current_window) = &self.window {
            let resize_notifier = ctx.callback(|_, _: ()| {});

            let current_size = current_window.inner_size();

            CurrentWindow {
                handle: current_window.clone(),

                child: WinitWindowLayout {
                    handle: current_window.clone(),
                    child: ctx.widget.child.clone(),

                    listener: current_window.events().add_listener(move |event| {
                        if let WindowEvent::Resized(size) = event {
                            if current_size != *size {
                                resize_notifier.call(());
                            }
                        }
                    }),
                }
                .into_widget(),
            }
            .into_widget()
        } else {
            SizedBox::shrink().into_widget()
        }
    }
}

#[derive(InheritedWidget)]
pub struct CurrentWindow {
    handle: WinitWindowHandle,

    child: Widget,
}

impl Deref for CurrentWindow {
    type Target = WinitWindowHandle;

    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}

impl InheritedWidget for CurrentWindow {
    fn get_child(&self) -> Widget {
        self.child.clone()
    }

    #[allow(unused_variables)]
    fn should_notify(&self, old_widget: &Self) -> bool {
        self.handle.id() != old_widget.handle.id()
    }
}

#[derive(LayoutWidget)]
struct WinitWindowLayout {
    handle: WinitWindowHandle,
    child: Widget,

    listener: EventEmitterHandle<WindowEvent<'static>>,
}

impl WidgetLayout for WinitWindowLayout {
    fn get_children(&self) -> Vec<Widget> {
        vec![self.child.clone()]
    }

    fn intrinsic_size(
        &self,
        ctx: &mut IntrinsicSizeContext,
        dimension: IntrinsicDimension,
        cross_extent: f32,
    ) -> f32 {
        ctx.iter_children().next().map_or(0.0, |child| {
            child.compute_intrinsic_size(dimension, cross_extent)
        })
    }

    fn layout(&self, ctx: &mut LayoutContext, _: Constraints) -> Size {
        let mut children = ctx.iter_children_mut();

        let size = self.handle.inner_size();
        let size = Size::new(size.width as f32, size.height as f32);

        while let Some(mut child) = children.next() {
            child.compute_layout(Constraints::from(size));
        }

        size
    }
}
