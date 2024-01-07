use std::marker::PhantomData;

use agui_core::{
    element::{ContextDirtyRenderObject, RenderObjectCreateContext, RenderObjectUpdateContext},
    render::object::{
        RenderObjectImpl, RenderObjectIntrinsicSizeContext, RenderObjectLayoutContext,
    },
    unit::{Constraints, IntrinsicDimension, Size},
    widget::{IntoWidget, Widget},
};
use agui_elements::{
    render::RenderObjectWidget,
    stateful::{ContextWidgetStateMut, StatefulBuildContext, StatefulWidget, WidgetState},
};
use agui_inheritance::ContextInherited;
use agui_macros::{build, RenderObjectWidget, StatefulWidget};
use agui_primitives::sized_box::SizedBox;
use winit::window::WindowBuilder;

use crate::CurrentWindow;
use crate::{handle::WinitWindowHandle, WinitWindowManager};

#[derive(StatefulWidget)]
pub struct WinitWindow<WindowFn>
where
    WindowFn: Fn() -> WindowBuilder + Send + Sync + Clone + 'static,
{
    pub window: WindowFn,

    pub child: Widget,
}

impl<WindowFn> StatefulWidget for WinitWindow<WindowFn>
where
    WindowFn: Fn() -> WindowBuilder + Send + Sync + Clone + 'static,
{
    type State = WinitWindowState<WindowFn>;

    fn create_state(&self) -> Self::State {
        WinitWindowState {
            phantom: PhantomData,

            window: None,
        }
    }
}

pub struct WinitWindowState<WindowFn> {
    phantom: PhantomData<WindowFn>,

    window: Option<WinitWindowHandle>,
}

impl<WindowFn> WidgetState for WinitWindowState<WindowFn>
where
    WindowFn: Fn() -> WindowBuilder + Send + Sync + Clone + 'static,
{
    type Widget = WinitWindow<WindowFn>;

    fn init_state(&mut self, ctx: &mut StatefulBuildContext<Self>) {
        // let mouse_input_event_cb = ctx.callback(
        //     |_ctx, (device_id, state, button): (DeviceId, ElementState, MouseButton)| {
        //         // println!(
        //         //     "Mouse input event: {:?} {:?} {:?}",
        //         //     device_id, state, button
        //         // );
        //     },
        // );

        // let resize_event_cb = ctx.callback(|ctx, new_size: Size| {
        //     if ctx.state.window_size == Some(new_size) {
        //         return;
        //     }

        //     ctx.set_state(move |state| {
        //         state.window_size.replace(new_size);
        //     });
        // });

        let on_window_created = ctx.callback(move |ctx, window: WinitWindowHandle| {
            // let mouse_input_event_cb = mouse_input_event_cb.clone();
            // let resize_event_cb = resize_event_cb.clone();

            ctx.set_state(|state| {
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

                state.window.replace(window);

                // state
                //     .window_size
                //     .replace(Size::new(size.width as f32, size.height as f32));
            });
        });

        let Some(window_manager) = ctx.find_inherited_widget::<WinitWindowManager>() else {
            return tracing::error!("windowing plugin not found");
        };

        if let Err(err) = window_manager.create_window(ctx.widget.window.clone(), on_window_created)
        {
            tracing::error!("failed to create window: {:?}", err);
        }
    }

    fn updated(&mut self, ctx: &mut StatefulBuildContext<Self>, old_widget: &Self::Widget) {}

    fn build(&mut self, ctx: &mut StatefulBuildContext<Self>) -> Widget {
        if let Some(current_window) = &self.window {
            build! {
                <CurrentWindow> {
                    handle: current_window.clone(),

                    child: <WinitWindowLayout> {
                        size: Size::ZERO,
                        child: ctx.widget.child.clone(),
                    }
                }
            }
        } else {
            SizedBox::shrink().into_widget()
        }
    }
}

#[derive(RenderObjectWidget)]
struct WinitWindowLayout {
    size: Size,

    child: Widget,
}

impl RenderObjectWidget for WinitWindowLayout {
    type RenderObject = RenderWinitWindow;

    fn children(&self) -> Vec<Widget> {
        vec![self.child.clone()]
    }

    fn create_render_object(&self, ctx: &mut RenderObjectCreateContext) -> Self::RenderObject {
        RenderWinitWindow { size: self.size }
    }

    fn update_render_object(
        &self,
        ctx: &mut RenderObjectUpdateContext,
        render_object: &mut Self::RenderObject,
    ) {
        render_object.update_size(ctx, self.size);
    }
}

struct RenderWinitWindow {
    size: Size,
}

impl RenderWinitWindow {
    fn update_size(&mut self, ctx: &mut RenderObjectUpdateContext, size: Size) {
        if self.size == size {
            return;
        }

        self.size = size;
        ctx.mark_needs_layout();
    }
}

impl RenderObjectImpl for RenderWinitWindow {
    fn intrinsic_size(
        &self,
        ctx: &mut RenderObjectIntrinsicSizeContext,
        dimension: IntrinsicDimension,
        cross_extent: f32,
    ) -> f32 {
        ctx.iter_children().next().map_or(0.0, |child| {
            child.compute_intrinsic_size(dimension, cross_extent)
        })
    }

    fn layout(&self, ctx: &mut RenderObjectLayoutContext, _: Constraints) -> Size {
        let mut children = ctx.iter_children_mut();

        while let Some(mut child) = children.next() {
            child.layout(Constraints::from(self.size));
        }

        self.size
    }
}
