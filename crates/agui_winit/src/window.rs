use std::{marker::PhantomData, ops::Deref, sync::Arc};

use agui_core::{
    element::ContextElement,
    listenable::EventEmitterHandle,
    unit::{Constraints, IntrinsicDimension, Size},
    widget::{IntoWidget, Widget},
};
use agui_elements::{
    layout::{IntrinsicSizeContext, LayoutContext, WidgetLayout},
    stateful::{ContextWidgetStateMut, StatefulBuildContext, StatefulWidget, WidgetState},
};
use agui_inheritance::ContextInheritedMut;
use agui_macros::{build, LayoutWidget, StatefulWidget, WidgetProps};
use agui_primitives::sized_box::SizedBox;
use agui_renderer::{CurrentRenderView, DefaultRenderer, RenderView, RenderViewId, Renderer};
use winit::{
    event::{DeviceId, ElementState, MouseButton, WindowEvent},
    window::WindowBuilder,
};

use crate::WinitWindowEvent;
use crate::{handle::WinitWindowHandle, CurrentWindow, WinitPlugin};

#[derive(WidgetProps)]
pub struct Window<WindowFn>
where
    WindowFn: Fn() -> WindowBuilder + 'static,
{
    pub window: WindowFn,

    #[prop(default)]
    pub renderer: Option<Arc<dyn Renderer<Target = winit::window::Window>>>,

    #[prop(into)]
    pub child: Widget,
}

impl<WindowFn> IntoWidget for Window<WindowFn>
where
    WindowFn: Fn() -> WindowBuilder + 'static,
{
    fn into_widget(self) -> Widget {
        // Windows must be created within their own render view
        build! {
            <RenderView> {
                child: <WinitWindow> {
                    window: self.window,

                    renderer: self.renderer,

                    child: self.child.clone(),
                }
            }
        }
    }
}

#[derive(StatefulWidget)]
struct WinitWindow<WindowFn>
where
    WindowFn: Fn() -> WindowBuilder + 'static,
{
    window: WindowFn,

    renderer: Option<Arc<dyn Renderer<Target = winit::window::Window>>>,

    child: Widget,
}

impl<WindowFn> StatefulWidget for WinitWindow<WindowFn>
where
    WindowFn: Fn() -> WindowBuilder + 'static,
{
    type State = WinitWindowState<WindowFn>;

    fn create_state(&self) -> Self::State {
        WinitWindowState {
            phantom: PhantomData,

            render_view_id: None,

            renderer: None,

            window: None,
            event_listener: None,
        }
    }
}

enum WindowRenderer {
    Default(Arc<dyn Renderer<Target = winit::window::Window>>),
    Defined(Arc<dyn Renderer<Target = winit::window::Window>>),
}

impl Deref for WindowRenderer {
    type Target = Arc<dyn Renderer<Target = winit::window::Window>>;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Default(renderer) => renderer,
            Self::Defined(renderer) => renderer,
        }
    }
}

struct WinitWindowState<WindowFn> {
    phantom: PhantomData<WindowFn>,

    render_view_id: Option<RenderViewId>,

    renderer: Option<WindowRenderer>,

    window: Option<WinitWindowHandle>,
    event_listener: Option<EventEmitterHandle<WinitWindowEvent>>,
}

impl<WindowFn> WidgetState for WinitWindowState<WindowFn>
where
    WindowFn: Fn() -> WindowBuilder + 'static,
{
    type Widget = WinitWindow<WindowFn>;

    fn init_state(&mut self, ctx: &mut StatefulBuildContext<Self>) {
        if let Some(renderer) = ctx.widget.renderer.as_ref() {
            self.renderer = Some(WindowRenderer::Defined(Arc::clone(renderer)));
        }

        self.create_window(ctx);
    }

    fn updated(&mut self, ctx: &mut StatefulBuildContext<Self>, old_widget: &Self::Widget) {
        if !self.is_renderer_eq(ctx.widget.renderer.as_ref(), old_widget.renderer.as_ref()) {
            if let Some(renderer) = ctx.widget.renderer.as_ref() {
                self.renderer = Some(WindowRenderer::Defined(Arc::clone(renderer)));

                self.bind_renderer(ctx);
            }
        }
    }

    fn build(&mut self, ctx: &mut StatefulBuildContext<Self>) -> Widget {
        if let Some(current_window) = &self.window {
            let current_render_view_id = ctx
                .depend_on_inherited_widget::<CurrentRenderView>()
                .map(|current_render_view| current_render_view.id);

            let mut needs_rebind = false;

            if self.render_view_id != current_render_view_id {
                self.render_view_id = current_render_view_id;

                needs_rebind = true;
            }

            if !matches!(self.renderer.as_ref(), Some(WindowRenderer::Defined(_))) {
                let current_renderer = match self.renderer.as_ref() {
                    Some(WindowRenderer::Default(renderer)) => Some(renderer),
                    _ => None,
                };

                let default_renderer = ctx
                    .depend_on_inherited_widget::<DefaultRenderer<winit::window::Window>>()
                    .map(|default_renderer| Arc::clone(&default_renderer.renderer));

                if !self.is_renderer_eq(current_renderer, default_renderer.as_ref()) {
                    if let Some(default_renderer) = default_renderer {
                        self.renderer = Some(WindowRenderer::Default(default_renderer));

                        needs_rebind = true;
                    }
                }
            }

            if needs_rebind {
                self.bind_renderer(ctx);
            }

            let resize_notifier = ctx.callback(|_, _: ()| {});

            let current_size = current_window.inner_size();

            build! {
                <CurrentWindow> {
                    handle: current_window.clone(),

                    child: <WinitWindowLayout> {
                        handle: current_window.clone(),
                        child: ctx.widget.child.clone(),

                        listener: current_window.events().add_listener(move |WinitWindowEvent(ref event)| {
                            if let WindowEvent::Resized(size) = event {
                                if current_size != *size {
                                    resize_notifier.call(());
                                }
                            }
                        }),
                    }
                }
            }
        } else {
            SizedBox::shrink().into_widget()
        }
    }
}

impl<WindowFn> WinitWindowState<WindowFn>
where
    WindowFn: Fn() -> WindowBuilder + 'static,
{
    fn create_window(&self, ctx: &mut StatefulBuildContext<Self>) {
        if let Some(window) = &self.window {
            if let Err(window) = window.close() {
                return tracing::error!("failed to close old window: {:?}", window);
            }
        }

        let mouse_input_event_cb = ctx.callback(
            |_ctx, (device_id, state, button): (DeviceId, ElementState, MouseButton)| {
                // println!(
                //     "Mouse input event: {:?} {:?} {:?}",
                //     device_id, state, button
                // );
            },
        );

        let on_window_created = ctx.callback(move |ctx, window: WinitWindowHandle| {
            let mouse_input_event_cb = mouse_input_event_cb.clone();

            ctx.set_state(|state| {
                state.event_listener = Some(window.events().add_listener(
                    move |WinitWindowEvent(ref event)| {
                        if let WindowEvent::MouseInput {
                            device_id,
                            state,
                            button,
                            ..
                        } = event
                        {
                            mouse_input_event_cb.call((*device_id, *state, *button));
                        }
                    },
                ));

                state.window.replace(window);
            });
        });

        let Some(winit_plugin) = ctx.plugins.get::<WinitPlugin>() else {
            return tracing::error!("windowing plugin not found");
        };

        if let Err(err) =
            winit_plugin.create_window(ctx.element_id(), (ctx.widget.window)(), on_window_created)
        {
            tracing::error!("failed to create window: {:?}", err);
        }
    }

    fn is_renderer_eq(
        &self,
        renderer: Option<&Arc<dyn Renderer<Target = winit::window::Window>>>,
        other_renderer: Option<&Arc<dyn Renderer<Target = winit::window::Window>>>,
    ) -> bool {
        match (renderer, other_renderer) {
            // war crimes
            (Some(a), Some(b)) => std::ptr::eq(
                Arc::as_ptr(a) as *const _ as *const (),
                Arc::as_ptr(b) as *const _ as *const (),
            ),
            (None, None) => true,
            _ => false,
        }
    }

    fn bind_renderer(&self, ctx: &mut StatefulBuildContext<Self>) {
        let Some(window) = self.window.as_ref() else {
            return;
        };

        let Some(render_view_id) = self.render_view_id else {
            return;
        };

        let Some(renderer) = self.renderer.as_ref() else {
            return tracing::error!(
                "if no view renderer is provided, then a default view renderer must be specified"
            );
        };

        let Some(winit_plugin) = ctx.plugins.get_mut::<WinitPlugin>() else {
            return tracing::error!("windowing plugin not found");
        };

        if let Err(err) =
            winit_plugin.bind_renderer(window.id(), render_view_id, Arc::clone(renderer))
        {
            tracing::error!("failed to bind renderer: {:?}", err);
        }
    }

    // TODO: call this on unmount
    fn unbind_renderer(&mut self, ctx: &mut StatefulBuildContext<Self>) {
        let Some(window) = self.window.as_ref() else {
            return;
        };

        let Some(winit_plugin) = ctx.plugins.get_mut::<WinitPlugin>() else {
            return tracing::error!("windowing plugin not found");
        };

        winit_plugin.unbind_renderer(&window.id());
    }
}

#[derive(LayoutWidget)]
struct WinitWindowLayout {
    handle: WinitWindowHandle,
    child: Widget,

    listener: EventEmitterHandle<WinitWindowEvent>,
}

impl WidgetLayout for WinitWindowLayout {
    fn children(&self) -> Vec<Widget> {
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
