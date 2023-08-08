use std::{cell::RefCell, ops::Deref};

use agui_core::{
    notifier::ListenerHandle,
    unit::{Constraints, Size},
    widget::{
        render_context::RenderContextBoundary, BuildContext, ContextInheritedMut, ContextWidget,
        ContextWidgetStateMut, InheritedWidget, IntoChild, LayoutContext, StatefulBuildContext,
        StatefulWidget, Widget, WidgetBuild, WidgetLayout, WidgetState,
    },
};
use agui_macros::{build, InheritedWidget, LayoutWidget, StatefulWidget, StatelessWidget};
use winit::window::WindowBuilder;

use crate::{
    event::WindowEvent, handle::WinitWindowHandle, windowing_controller::WinitWindowingController,
};

#[derive(StatelessWidget, Default)]
pub struct Window {
    pub window: WindowBuilder,

    pub child: Option<Widget>,
}

impl Window {
    pub fn new(builder: WindowBuilder) -> Self {
        Self {
            window: builder,

            child: None,
        }
    }

    pub fn with_child(mut self, child: impl IntoChild) -> Self {
        self.child = child.into_child();

        self
    }
}

impl WidgetBuild for Window {
    type Child = RenderContextBoundary;

    fn build(&self, _: &mut BuildContext<Self>) -> Self::Child {
        build! {
            // Windows must be created within their own render context
            RenderContextBoundary {
                child: WinitWindow {
                    window: self.window.clone(),

                    child: self.child.clone(),
                }
            }
        }
    }
}

#[derive(StatefulWidget, Default)]
struct WinitWindow {
    window: WindowBuilder,

    child: Option<Widget>,
}

impl StatefulWidget for WinitWindow {
    type State = WinitWindowState;

    fn create_state(&self) -> Self::State {
        WinitWindowState {
            window: None,

            child: self.child.clone(),
        }
    }
}

struct WinitWindowState {
    window: Option<WinitWindowHandle>,

    child: Option<Widget>,
}

impl WidgetState for WinitWindowState {
    type Widget = WinitWindow;

    type Child = Option<CurrentWindow>;

    fn init_state(&mut self, ctx: &mut StatefulBuildContext<Self>) {
        let Some(windowing) = ctx.depend_on_inherited_widget::<WinitWindowingController>() else {
            return tracing::error!("Windowing controller not found in the widget tree");
        };

        let create_cb = ctx.callback(|ctx, window| {
            ctx.set_state(|state| {
                state.window.replace(window);
            });
        });

        // I don't like cloning the window, here.
        windowing.create_window(ctx.get_element_id(), ctx.widget.window.clone(), create_cb);
    }

    fn build(&mut self, _: &mut StatefulBuildContext<Self>) -> Self::Child {
        if let Some(current_window) = &self.window {
            Some(CurrentWindow {
                handle: current_window.clone(),

                child: WinitWindowLayout {
                    handle: current_window.clone(),

                    listener: RefCell::new(None),

                    child: self.child.clone(),
                }
                .into_child(),
            })
        } else {
            None
        }
    }
}

#[derive(InheritedWidget)]
pub struct CurrentWindow {
    handle: WinitWindowHandle,

    #[child]
    child: Option<Widget>,
}

impl Deref for CurrentWindow {
    type Target = winit::window::Window;

    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}

impl InheritedWidget for CurrentWindow {
    #[allow(unused_variables)]
    fn should_notify(&self, old_widget: &Self) -> bool {
        self.handle.id() != old_widget.handle.id()
    }
}

#[derive(LayoutWidget)]
struct WinitWindowLayout {
    handle: WinitWindowHandle,

    listener: RefCell<Option<ListenerHandle<WindowEvent>>>,

    child: Option<Widget>,
}

impl WidgetLayout for WinitWindowLayout {
    type Children = Widget;

    fn build(&self, ctx: &mut BuildContext<Self>) -> Vec<Self::Children> {
        let notifier = ctx.callback(|_, _: ()| {});

        // We use interior mutability here to reduce the amount of nested widget fuckery
        self.listener
            .borrow_mut()
            .replace(self.handle.add_listener(move |event| {
                if let WindowEvent::Resized(..) = event {
                    notifier.call(());
                }
            }));

        Vec::from_iter(self.child.iter().cloned())
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
