use std::{marker::PhantomData, ops::Deref, sync::Arc};

use agui_core::{
    element::{ContextDirtyRenderObject, ContextElement},
    listenable::EventEmitterHandle,
    render::{RenderObjectImpl, RenderObjectIntrinsicSizeContext, RenderObjectLayoutContext},
    unit::{Constraints, IntrinsicDimension, Size},
    util::ptr_eq::PtrEqual,
    widget::{IntoWidget, Widget},
};
use agui_elements::{
    render::RenderObjectWidget,
    stateful::{ContextWidgetStateMut, StatefulBuildContext, StatefulWidget, WidgetState},
    stateless::{StatelessBuildContext, StatelessWidget},
};
use agui_inheritance::ContextInheritedMut;
use agui_macros::{build, RenderObjectWidget, StatefulWidget, StatelessWidget, WidgetProps};
use agui_primitives::sized_box::SizedBox;
use agui_renderer::{CurrentRenderView, RenderViewId, Renderer, ViewBoundary};
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

    pub renderer: Arc<dyn Renderer<Target = winit::window::Window>>,

    pub child: Widget,
}

impl<WindowFn> IntoWidget for Window<WindowFn>
where
    WindowFn: Fn() -> WindowBuilder + 'static,
{
    fn into_widget(self) -> Widget {
        // Windows must be created within their own render view
        build! {
            <ViewBoundary> {
                binding: self.renderer,

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

            window_size: None,
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

    window_size: Option<Size>,
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
        if !ctx.widget.renderer.is_exact_ptr(&old_widget.renderer) {
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

                if !current_renderer.is_exact_ptr(&default_renderer) {
                    if let Some(default_renderer) = default_renderer {
                        self.renderer = Some(WindowRenderer::Default(default_renderer));

                        needs_rebind = true;
                    }
                }
            }

            if needs_rebind {
                self.bind_renderer(ctx);
            }

            build! {
                <CurrentWindow> {
                    handle: current_window.clone(),

                    child: <WinitWindowLayout> {
                        size: self.window_size.expect("window_size must be set when current_window is set"),
                        child: ctx.widget.child.clone(),
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

        let resize_event_cb = ctx.callback(|ctx, new_size: Size| {
            if ctx.state.window_size == Some(new_size) {
                return;
            }

            ctx.set_state(move |state| {
                state.window_size.replace(new_size);
            });
        });

        let on_window_created = ctx.callback(move |ctx, window: WinitWindowHandle| {
            let mouse_input_event_cb = mouse_input_event_cb.clone();
            let resize_event_cb = resize_event_cb.clone();

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
                        } else if let WindowEvent::Resized(size) = event {
                            resize_event_cb.call(Size::new(size.width as f32, size.height as f32));
                        }
                    },
                ));

                let size = window.inner_size();

                state.window.replace(window);

                state
                    .window_size
                    .replace(Size::new(size.width as f32, size.height as f32));
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
