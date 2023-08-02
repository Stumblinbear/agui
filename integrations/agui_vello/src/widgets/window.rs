use agui::{prelude::*, widget::render_context::RenderContextBoundary};
use winit::window::WindowBuilder;

use crate::bindings::{WinitWindowHandle, WinitWindowingController};

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
            winit: None,

            child: self.child.clone(),
        }
    }
}

struct WinitWindowState {
    winit: Option<WinitWindowHandle>,

    child: Option<Widget>,
}

impl WidgetState for WinitWindowState {
    type Widget = WinitWindow;

    type Child = WinitWindowLayout;

    fn init_state(&mut self, ctx: &mut StatefulBuildContext<Self>) {
        let Some(windowing) = ctx.depend_on_inherited_widget::<WinitWindowingController>() else {
            return tracing::error!("Windowing controller not found in the widget tree");
        };

        let create_cb = ctx.callback(|ctx, handle| {
            ctx.set_state(|state| {
                state.winit.replace(handle);
            });
        });

        // I don't like cloning the window, here.
        windowing.create_window(ctx.get_element_id(), ctx.widget.window.clone(), create_cb);
    }

    // TODO: handle window re-creation if the WinitWindowingController is replaced?

    fn build(&mut self, _: &mut StatefulBuildContext<Self>) -> Self::Child {
        // TODO: sync the window size
        WinitWindowLayout {
            size: Size::new(800.0, 600.0),

            child: self.child.clone(),
        }
    }
}

#[derive(LayoutWidget)]
struct WinitWindowLayout {
    size: Size,

    child: Option<Widget>,
}

impl WidgetLayout for WinitWindowLayout {
    type Children = Widget;

    fn build(&self, _: &mut BuildContext<Self>) -> Vec<Self::Children> {
        Vec::from_iter(self.child.iter().cloned())
    }

    fn layout(&self, ctx: &mut LayoutContext, _: Constraints) -> Size {
        let mut children = ctx.iter_children_mut();

        while let Some(mut child) = children.next() {
            child.compute_layout(Constraints::from(self.size));
        }

        self.size
    }
}
