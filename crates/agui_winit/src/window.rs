use std::{cell::RefCell, ops::Deref};

use agui_core::{
    listeners::EventEmitterHandle,
    unit::{Constraints, Size},
    widget::{
        view::RenderView, BuildContext, ContextInheritedMut, ContextWidget, ContextWidgetStateMut,
        InheritedWidget, IntoWidget, LayoutContext, StatefulBuildContext, StatefulWidget, Widget,
        WidgetBuild, WidgetLayout, WidgetState,
    },
};
use agui_macros::{build, InheritedWidget, LayoutWidget, StatefulWidget, StatelessWidget};
use agui_primitives::sized_box::SizedBox;
use winit::window::WindowBuilder;

use crate::{binding::WinitBinding, event::WinitWindowEvent, handle::WinitWindowHandle};

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
        WinitWindowState { window: None }
    }
}

struct WinitWindowState {
    window: Option<WinitWindowHandle>,
}

impl WidgetState for WinitWindowState {
    type Widget = WinitWindow;

    fn init_state(&mut self, ctx: &mut StatefulBuildContext<Self>) {
        let Some(binding) = ctx.depend_on_inherited_widget::<WinitBinding>() else {
            return tracing::error!("Windowing controller not found in the widget tree");
        };

        let create_cb = ctx.callback(|ctx, window| {
            ctx.set_state(|state| {
                state.window.replace(window);
            });
        });

        // I don't like cloning the window, here.
        binding.create_window(ctx.get_element_id(), ctx.widget.window.clone(), create_cb);
    }

    fn build(&mut self, ctx: &mut StatefulBuildContext<Self>) -> Widget {
        if let Some(current_window) = &self.window {
            CurrentWindow {
                handle: current_window.clone(),

                child: WinitWindowLayout {
                    handle: current_window.clone(),
                    child: ctx.widget.child.clone(),

                    listener: RefCell::new(None),
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

    listener: RefCell<Option<EventEmitterHandle<WinitWindowEvent>>>,
}

impl WidgetLayout for WinitWindowLayout {
    fn build(&self, ctx: &mut BuildContext<Self>) -> Vec<Widget> {
        let notifier = ctx.callback(|_, _: ()| {});

        let current_size = self.handle.inner_size();

        // We use interior mutability here to reduce the amount of nested widget fuckery
        self.listener
            .borrow_mut()
            .replace(self.handle.events().add_listener(move |event| {
                if let WinitWindowEvent::Resized(size) = event {
                    if current_size != *size {
                        notifier.call(());
                    }
                }
            }));

        vec![self.child.clone()]
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
